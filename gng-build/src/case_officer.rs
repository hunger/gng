// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle `gng-build-agent`s

use crate::handler::Handler;
use crate::Mode;

use gng_build_shared::constants::container as cc;
use gng_build_shared::constants::environment as ce;
use gng_shared::is_executable;

use std::convert::TryFrom;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use eyre::{eyre, Result, WrapErr};
use rand::Rng;

// - Helper:
// ----------------------------------------------------------------------

const BUILDER_MACHINE_ID: &str = "0bf95bb771364ef997e1df5eb3b26422";

const MESSAGE_PREFIX_LEN: usize = 8;

#[tracing::instrument]
fn prepare_for_systemd_nspawn(root_directory: &Path) -> Result<()> {
    std::fs::create_dir(root_directory.join("usr"))?;
    std::fs::create_dir(root_directory.join("usr/local"))?;
    std::fs::create_dir(root_directory.join("tmp"))?;
    std::fs::create_dir(root_directory.join("run"))?;
    std::fs::create_dir(root_directory.join("proc"))?;
    std::fs::create_dir(root_directory.join("sys"))?;
    std::fs::create_dir(root_directory.join("gng"))?;

    Ok(())
}

fn bind(read_only: bool, from: &Path, to: &Path) -> OsString {
    let mut result = OsString::from("--bind");
    result.push(if read_only {
        OsString::from("-ro=")
    } else {
        OsString::from("=")
    });
    result.push(from.as_os_str());
    result.push(":");
    result.push(to.as_os_str());
    result
}

fn overlay(paths: &[PathBuf]) -> OsString {
    if paths.is_empty() {
        return OsString::new();
    }

    let mut prefix = OsString::from("--overlay=");
    let mut result = OsString::new();
    for p in paths {
        result.push(prefix);
        result.push(p.as_os_str());
        prefix = OsString::from(":");
    }
    result
}

fn setenv(name: &str, value: &str) -> OsString {
    let mut result = OsString::from("--setenv=");
    result.push(name);
    result.push("=");
    result.push(value);
    result
}

fn random_string(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let ascii = std::iter::repeat(())
        .map(|()| rng.sample(rand::distributions::Alphanumeric))
        .take(len)
        .collect::<Vec<u8>>();
    String::from_utf8(ascii).expect("Input should have been ASCII!")
}

fn find_type_and_contents<'a>(message_prefix: &'a str, line: &'a str) -> (&'a str, &'a str) {
    if line.len() < 4 /* "MSG_" */ + MESSAGE_PREFIX_LEN + 7
    /* "_TYPE: " */
    {
        return ("", line);
    }
    if !line.starts_with("MSG_") {
        return ("", line);
    }

    if &line[4..4 + MESSAGE_PREFIX_LEN] != message_prefix {
        return ("", line);
    }

    if !line[4 + MESSAGE_PREFIX_LEN + 1 + 4..].starts_with(": ") {
        return ("", line);
    }

    (
        &line[4 + MESSAGE_PREFIX_LEN + 1..4 + MESSAGE_PREFIX_LEN + 1 + 4],
        &line[4 + MESSAGE_PREFIX_LEN + 7..],
    )
}

fn build_script(pkgsrc_directory: &Path) -> Result<PathBuf> {
    let build_file = pkgsrc_directory.join(gng_build_shared::BUILD_SCRIPT);
    if !build_file.is_file() {
        return Err(eyre!(format!(
            "No {} file found in {}.",
            gng_build_shared::BUILD_SCRIPT,
            pkgsrc_directory.to_string_lossy()
        )));
    }
    Ok(build_file)
}

fn validate_executable(path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize().wrap_err(format!(
        "Failed to canonicalize executable path \"{}\".",
        path.to_string_lossy()
    ))?;

    if !path.is_file() {
        return Err(eyre!(
            "Executable \"{}\" is not a file.",
            path.to_string_lossy()
        ));
    }
    if !is_executable(&path) {
        return Err(eyre!(
            "Executable \"{}\" is not marked executable.",
            path.to_string_lossy()
        ));
    }

    Ok(path)
}

fn path_buf_or_tempdir(
    path: &Option<PathBuf>,
    prefix: &str,
    scratch_directory: &Path,
    temp_dirs: &mut Vec<tempfile::TempDir>,
) -> Result<PathBuf> {
    if let Some(p) = path {
        return Ok(p.clone());
    }

    let tmp = tempfile::Builder::new()
        .prefix(prefix)
        .rand_bytes(8)
        .tempdir_in(scratch_directory)
        .wrap_err(format!(
            "Failed to create temporary directory in \"{}\".",
            scratch_directory.to_string_lossy()
        ))?;
    let path = tmp.path().to_owned();

    temp_dirs.push(tmp);

    Ok(path)
}

fn find_installed_agent(exe_directory: &std::path::Path) -> std::path::PathBuf {
    let base_search_dir = exe_directory.parent().unwrap_or(exe_directory);
    base_search_dir.join("lib/gng/gng-build-agent")
}

fn find_development_agent(exe_directory: &std::path::Path) -> std::path::PathBuf {
    exe_directory.join("gng-build-agent")
}

fn find_gng_build_agent() -> eyre::Result<PathBuf> {
    let exe_directory = std::env::current_exe()
        .wrap_err("Could not find current executable path.")?
        .parent()
        .ok_or_else(|| eyre::eyre!("Failed to get parent directory of current executable."))?
        .to_owned();

    tracing::trace!(
        "Looking for gng-build-agent relative to: \"{}\".",
        exe_directory.to_string_lossy()
    );

    for i in &[
        find_installed_agent(&exe_directory),
        find_development_agent(&exe_directory),
    ] {
        if validate_executable(i).is_ok() {
            tracing::debug!("Found gng-build-agent: \"{}\".", i.to_string_lossy());
            return Ok(i.clone());
        }
    }

    Err(eyre!("Could not find Lua directory for gng-build-agent"))
}

fn is_lua_directory(lua_directory: &std::path::Path) -> bool {
    lua_directory.join("startup.lua").is_file()
}

fn find_installed_lua_directory(agent_dir: &std::path::Path) -> std::path::PathBuf {
    let base_search_dir = agent_dir.parent().unwrap_or(agent_dir);
    base_search_dir.join("share/gng/gng-build-agent/lua")
}

fn find_development_lua_directory(agent_dir: &std::path::Path) -> std::path::PathBuf {
    let target_dir = agent_dir
        .ancestors()
        .find(|a| a.file_name() == Some(std::ffi::OsStr::new("target")))
        .unwrap_or(agent_dir);
    target_dir
        .parent()
        .unwrap_or(target_dir)
        .join("gng-build-agent/lua")
}

fn find_lua_directory(agent: &std::path::Path) -> eyre::Result<std::path::PathBuf> {
    let agent_dir = agent
        .parent()
        .ok_or_else(|| eyre!("Failed to get directory containing gng-build-agent executable"))?;

    for i in &[
        find_installed_lua_directory(agent_dir),
        find_development_lua_directory(agent_dir),
    ] {
        if is_lua_directory(i) {
            tracing::debug!("Found Lua directory: \"{}\".", i.to_string_lossy());
            return Ok(i.clone());
        }
    }

    Err(eyre!("Could not find Lua directory for gng-build-agent"))
}

// ----------------------------------------------------------------------
// - CaseOfficerBuilder:
// ----------------------------------------------------------------------

/// A builder for `CaseOfficer`
pub struct CaseOfficerBuilder {
    agent: Option<PathBuf>,
    nspawn_binary: PathBuf,

    lua_directory: Option<PathBuf>,
    scratch_directory: PathBuf,
    work_directory: Option<PathBuf>,
    install_directory: Option<PathBuf>,

    handlers: Vec<Box<dyn Handler>>,
}

impl Default for CaseOfficerBuilder {
    fn default() -> Self {
        Self {
            agent: None,
            nspawn_binary: PathBuf::from("/usr/bin/systemd-nspawn"),

            lua_directory: None,
            scratch_directory: std::env::temp_dir(),
            work_directory: None,
            install_directory: None,

            handlers: vec![],
        }
    }
}

impl CaseOfficerBuilder {
    /// Set the `lua_directory` to use
    pub fn set_lua_directory(&mut self, directory: &Path) -> &mut Self {
        self.lua_directory = Some(directory.to_owned());
        self
    }

    /// Set the `scratch_directory` to use
    pub fn set_scratch_directory(&mut self, directory: &Path) -> &mut Self {
        self.scratch_directory = directory.to_owned();
        self
    }

    /// Set the `work_directory` to use
    pub fn set_work_directory(&mut self, directory: &Path) -> &mut Self {
        self.work_directory = Some(directory.to_owned());
        self
    }

    /// Set the `install_directory` to use
    pub fn set_install_directory(&mut self, directory: &Path) -> &mut Self {
        self.install_directory = Some(directory.to_owned());
        self
    }

    /// Set the `src_directory` to use
    pub fn set_agent(&mut self, file: &Path) -> &mut Self {
        self.agent = Some(file.to_owned());
        self
    }

    /// Set the `src_directory` to use
    pub fn set_systemd_nspawn(&mut self, file: &Path) -> &mut Self {
        self.nspawn_binary = file.to_owned();
        self
    }

    /// Add an `Handler`
    pub fn add_handler(&mut self, handler: Box<dyn Handler>) -> &mut Self {
        self.handlers.push(handler);
        self
    }

    /// Evaluate a script file
    ///
    /// # Errors
    /// Generic Error
    pub fn build(&mut self, pkgsrc_directory: &Path) -> Result<CaseOfficer> {
        let mut temp_dirs = Vec::with_capacity(3);

        let root_directory =
            path_buf_or_tempdir(&None, "root-", &self.scratch_directory, &mut temp_dirs)?;
        prepare_for_systemd_nspawn(&root_directory)
            .wrap_err("Failed to set up root directory for nspawn use.")?;

        let work_directory = path_buf_or_tempdir(
            &self.work_directory,
            "work-",
            &self.scratch_directory,
            &mut temp_dirs,
        )?;
        let install_directory = path_buf_or_tempdir(
            &self.install_directory,
            "inst-",
            &self.scratch_directory,
            &mut temp_dirs,
        )?;

        let agent = if let Some(a) = &self.agent {
            tracing::debug!(
                "Using provided gng-build-agent: \"{}\".",
                a.to_string_lossy()
            );
            validate_executable(a)?
        } else {
            find_gng_build_agent()?
        };

        let lua_directory = if let Some(ld) = self.lua_directory.take() {
            tracing::debug!(
                "Using provided Lua directory: \"{}\".",
                ld.to_string_lossy()
            );
            ld
        } else {
            find_lua_directory(&agent)?
        };

        Ok(CaseOfficer {
            build_script: build_script(pkgsrc_directory)?,
            nspawn_binary: validate_executable(&std::mem::take(&mut self.nspawn_binary))?,
            agent,

            lua_directory,
            root_directory,
            work_directory,
            install_directory,

            handlers: std::mem::take(&mut self.handlers),
            temporary_directories: temp_dirs,
        })
    }
}

// ----------------------------------------------------------------------
// - CaseOfficer:
// ----------------------------------------------------------------------

/// The controller of the `gng-build-agent`
pub struct CaseOfficer {
    build_script: PathBuf,
    nspawn_binary: PathBuf,
    agent: PathBuf,

    lua_directory: PathBuf,
    root_directory: PathBuf,
    work_directory: PathBuf,
    install_directory: PathBuf,

    handlers: Vec<Box<dyn Handler>>,
    temporary_directories: Vec<tempfile::TempDir>,
}

impl std::fmt::Debug for CaseOfficer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CaseOfficer")
    }
}

impl CaseOfficer {
    fn mode_args(
        &self,
        work_ro: bool,
        inst_ro: bool,
        extra_args: &mut Vec<OsString>,
        mode_arg: &str,
    ) -> Vec<OsString> {
        let mut result = vec![
            bind(work_ro, &self.work_directory, &cc::GNG_WORK_DIR),
            bind(inst_ro, &self.install_directory, &cc::GNG_INST_DIR),
            bind(true, &self.lua_directory, &cc::GNG_LUA_DIR),
        ];
        result.append(extra_args);

        result.push(cc::GNG_BUILD_AGENT_EXECUTABLE.as_os_str().to_owned());
        result.push(OsString::from(mode_arg));

        result
    }

    fn mode_arguments(&self, mode: &Mode, message_prefix: &str) -> Vec<std::ffi::OsString> {
        let mut extra = match mode {
            Mode::Query => self.mode_args(true, true, &mut Vec::new(), "query"),
            Mode::Prepare => self.mode_args(false, true, &mut Vec::new(), "prepare"),
            Mode::Build => self.mode_args(false, true, &mut Vec::new(), "build"),
            Mode::Check => self.mode_args(false, true, &mut Vec::new(), "check"),
            Mode::Install => self.mode_args(
                true,
                false,
                &mut vec![overlay(&[
                    self.root_directory.join("usr"),
                    self.install_directory.clone(),
                    PathBuf::from("/usr"),
                ])],
                "install",
            ),
            Mode::Package => self.mode_args(true, true, &mut Vec::new(), "polish"),
        };

        let mut result = vec![
            bind(true, &self.agent, Path::new("/gng/build-agent")),
            OsString::from("--quiet"),
            OsString::from("--settings=off"),
            OsString::from("--register=off"),
            // OsString::from("-U"), // --private-users=pick or no user name-spacing
            OsString::from("--private-network"),
            OsString::from("--resolv-conf=off"),
            OsString::from("--timezone=off"),
            OsString::from("--link-journal=no"),
            OsString::from("--directory"),
            OsString::from(self.root_directory.as_os_str()),
            OsString::from(format!("--uuid={}", BUILDER_MACHINE_ID)),
            OsString::from("--console=interactive"),
            OsString::from("--tmpfs=/gng"),
            OsString::from("--read-only"),
            setenv(
                ce::GNG_BUILD_AGENT,
                cc::GNG_BUILD_AGENT_EXECUTABLE.to_str().unwrap(),
            ),
            setenv(ce::GNG_WORK_DIR, cc::GNG_WORK_DIR.to_str().unwrap()),
            setenv(ce::GNG_INST_DIR, cc::GNG_INST_DIR.to_str().unwrap()),
            setenv(ce::GNG_LUA_DIR, cc::GNG_LUA_DIR.to_str().unwrap()),
            setenv(ce::GNG_AGENT_MESSAGE_PREFIX, message_prefix),
        ];

        if let Ok(rust_log) = std::env::var("RUST_LOG") {
            let mut env_var = OsString::from("--setenv=RUST_LOG=");
            env_var.push(rust_log);
            result.push(env_var)
        }

        result.push(bind(
            true,
            &self.build_script,
            Path::new(&format!("/gng/{}", gng_build_shared::BUILD_SCRIPT)),
        ));

        result.append(&mut extra);

        result
    }

    #[tracing::instrument(level = "debug")]
    fn run_agent(
        &mut self,
        ctx: &crate::handler::Context,
        args: &[OsString],
        new_mode: &Mode,
        message_prefix: String,
    ) -> Result<()> {
        tracing::debug!("Starting gng-build-agent in systemd-nspawn");
        let mut child = std::process::Command::new(&self.nspawn_binary)
            .args(args)
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        tracing::trace!("Processing output of gng-build-agent");
        self.handle_agent_output(ctx, &mut child, new_mode, &message_prefix)?;

        tracing::trace!("Waiting for gng-build-agent to finish");
        let exit_status = child.wait()?;

        match exit_status.code() {
            None => Err(eyre!("gng-build-agent was killed by a signal.")),
            Some(code) if code == 0 => Ok(()),
            Some(code) => Err(eyre!("gng-build-agent failed with exit code {}.", code)),
        }
    }

    fn create_ctx(&self) -> crate::handler::Context {
        crate::handler::Context {
            lua_directory: self.lua_directory.clone(),
            work_directory: self.work_directory.clone(),
            install_directory: self.install_directory.clone(),
            build_file: self.build_script.clone(),
            build_agent: self.agent.clone(),
        }
    }

    /// Switch into a new `Mode` of operation
    #[tracing::instrument(level = "trace")]
    fn switch_mode(&mut self, ctx: &crate::handler::Context, new_mode: &Mode) -> Result<()> {
        tracing::debug!("Switching mode to {:?}", new_mode);

        for h in &mut self.handlers {
            h.prepare(ctx, new_mode)?;
        }

        let message_prefix = random_string(MESSAGE_PREFIX_LEN);

        // Start agent:
        let nspawn_args = self.mode_arguments(new_mode, &message_prefix);
        assert!(!nspawn_args.is_empty());

        self.run_agent(ctx, &nspawn_args, new_mode, message_prefix)?;

        for h in &mut self.handlers {
            h.verify(ctx, new_mode)?;
        }

        Ok(())
    }

    fn process_line(
        &mut self,
        ctx: &crate::handler::Context,
        mode: &Mode,
        message_prefix: &str,
        line: &str,
    ) -> Result<()> {
        lazy_static::lazy_static! {
            static ref PREFIX: String =
                std::env::var(ce::GNG_AGENT_OUTPUT_PREFIX).unwrap_or_else(|_| String::from("AGENT> "));
        }

        let (message_type, line) = find_type_and_contents(message_prefix, line);
        if message_type.is_empty() {
            println!("{}{}", *PREFIX, line);
        } else {
            tracing::trace!("{}MSG_{}: {}", *PREFIX, message_type, line);

            let message_type = gng_build_shared::MessageType::try_from(String::from(message_type))
                .map_err(|e| eyre!(e))?;
            for h in &mut self.handlers {
                if h.handle(ctx, mode, &message_type, line)? {
                    break;
                }
            }
        }
        Ok(())
    }

    fn handle_agent_output(
        &mut self,
        ctx: &crate::handler::Context,
        child: &mut std::process::Child,
        mode: &Mode,
        message_prefix: &str,
    ) -> Result<()> {
        let reader = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| eyre!("Could not capture stdout of gng-build-agent."))?,
        );

        for line in reader.lines() {
            match line {
                Ok(line) => self.process_line(ctx, mode, message_prefix, &line)?,
                Err(e) => return Err(eyre!(e)),
            }
        }

        let exit_status = child.wait()?;

        match exit_status.code() {
            Some(0) => Ok(()),
            Some(exit_code) => Err(eyre!(format!(
                "Agent failed with exit-status: {}.",
                exit_code
            ))),
            None => Err(eyre!("Agent killed by signal.")),
        }
    }

    /// Process a build
    #[tracing::instrument(level = "debug")]
    pub fn process(&mut self) -> Result<()> {
        let mut mode = Some(Mode::default());
        let ctx = self.create_ctx();

        while mode.is_some() {
            let m = mode.expect("Mode was some!");
            self.switch_mode(&ctx, &m)?;
            mode = m.next();
        }

        Ok(())
    }

    /// Clean up after a build
    #[tracing::instrument(level = "debug")]
    pub fn clean_up(&mut self) -> Result<()> {
        for td in self.temporary_directories.drain(..) {
            let p = td.path().to_owned();
            td.close().wrap_err(format!(
                "Failed to clean up temporary directory with path \"{}\".",
                p.to_string_lossy()
            ))?
        }

        Ok(())
    }
}
