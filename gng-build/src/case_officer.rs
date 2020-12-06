// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle `gng-build-agent`s

use crate::message_handler::MessageHandler;
use crate::Mode;

use gng_build_shared::constants::container as cc;
use gng_build_shared::constants::environment as ce;
use gng_shared::is_executable;

use std::convert::TryFrom;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use eyre::WrapErr;
use rand::Rng;

// - Helper:
// ----------------------------------------------------------------------

const BUILDER_MACHINE_ID: &str = "0bf95bb771364ef997e1df5eb3b26422";

const MESSAGE_PREFIX_LEN: usize = 8;

#[tracing::instrument]
fn prepare_for_systemd_nspawn(root_directory: &Path) -> eyre::Result<()> {
    std::fs::create_dir(root_directory.join("usr"))?;
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

fn overlay(paths: &Vec<PathBuf>) -> OsString {
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
    std::iter::repeat(())
        .map(|()| rng.sample(rand::distributions::Alphanumeric))
        .take(len)
        .collect::<String>()
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

fn validate_executable(path: &mut Option<PathBuf>) -> eyre::Result<PathBuf> {
    if path.is_none() {
        return Err(eyre::eyre!("Path is not set."));
    }

    let path = path.take().expect("It was some just a if ago!").clone();
    if !path.is_file() {
        return Err(eyre::eyre!(
            "Path \"{}\" is not a file.",
            path.to_string_lossy()
        ));
    }
    if !is_executable(&path) {
        return Err(eyre::eyre!(
            "Path \"{}\" is not executable.",
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
) -> eyre::Result<PathBuf> {
    if path.is_some() {
        return Ok(path.as_ref().unwrap().clone());
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

// ----------------------------------------------------------------------
// - CaseOfficerBuilder:
// ----------------------------------------------------------------------

/// A builder for `CaseOfficer`
pub struct CaseOfficerBuilder {
    agent: Option<PathBuf>,
    nspawn_binary: PathBuf,

    scratch_directory: PathBuf,
    work_directory: Option<PathBuf>,
    inst_directory: Option<PathBuf>,

    message_handlers: Vec<Box<dyn MessageHandler>>,
}

impl Default for CaseOfficerBuilder {
    fn default() -> Self {
        Self {
            agent: None,
            nspawn_binary: PathBuf::from("/usr/bin/systemd-nspawn"),

            scratch_directory: std::env::temp_dir(),
            work_directory: None,
            inst_directory: None,

            message_handlers: vec![],
        }
    }
}

impl CaseOfficerBuilder {
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

    /// Set the `inst_directory` to use
    pub fn set_install_directory(&mut self, directory: &Path) -> &mut Self {
        self.inst_directory = Some(directory.to_owned());
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

    /// Add an `MessageHandler`
    pub fn add_message_handler(&mut self, handler: Box<dyn MessageHandler>) -> &mut Self {
        self.message_handlers.push(handler);
        self
    }

    fn build_script(&self, pkgsrc_directory: &Path) -> eyre::Result<PathBuf> {
        let build_file = pkgsrc_directory.join("build.rhai");
        if !build_file.is_file() {
            return Err(eyre::eyre!(format!(
                "No build.rhai file found in {}.",
                pkgsrc_directory.to_string_lossy()
            )));
        }
        Ok(build_file)
    }

    /// Evaluate a script file
    pub fn build(&mut self, pkgsrc_directory: &Path) -> eyre::Result<CaseOfficer> {
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
        let inst_directory = path_buf_or_tempdir(
            &self.inst_directory,
            "inst-",
            &self.scratch_directory,
            &mut temp_dirs,
        )?;

        Ok(CaseOfficer {
            build_script: self.build_script(pkgsrc_directory)?,
            nspawn_binary: validate_executable(&mut Some(std::mem::take(&mut self.nspawn_binary)))?,
            agent: validate_executable(&mut self.agent)?,

            root_directory,
            work_directory,
            inst_directory,

            message_handlers: std::mem::take(&mut self.message_handlers),
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

    root_directory: PathBuf,
    work_directory: PathBuf,
    inst_directory: PathBuf,

    message_handlers: Vec<Box<dyn MessageHandler>>,
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
            bind(inst_ro, &self.inst_directory, &cc::GNG_INST_DIR),
        ];
        result.append(extra_args);

        result.push(cc::GNG_BUILD_AGENT_EXECUTABLE.as_os_str().to_owned());
        result.push(OsString::from(mode_arg));

        result
    }

    fn mode_arguments(&self, mode: &Mode, message_prefix: &str) -> Vec<std::ffi::OsString> {
        let mut extra = match mode {
            Mode::QUERY => self.mode_args(true, true, &mut Vec::new(), "query"),
            Mode::PREPARE => self.mode_args(false, true, &mut Vec::new(), "prepare"),
            Mode::BUILD => self.mode_args(false, true, &mut Vec::new(), "build"),
            Mode::CHECK => self.mode_args(false, true, &mut Vec::new(), "check"),
            Mode::INSTALL => self.mode_args(
                true,
                false,
                &mut vec![overlay(&vec![
                    self.root_directory.join("usr"),
                    self.inst_directory.clone(),
                    PathBuf::from("/usr"),
                ])],
                "install",
            ),
            Mode::PACKAGE => self.mode_args(true, true, &mut Vec::new(), "polish"),
        };

        let mut result = vec![
            bind(true, &self.agent, &Path::new("/gng/build-agent")),
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
                &cc::GNG_BUILD_AGENT_EXECUTABLE.to_str().unwrap(),
            ),
            setenv(ce::GNG_WORK_DIR, &cc::GNG_WORK_DIR.to_str().unwrap()),
            setenv(ce::GNG_INST_DIR, &cc::GNG_INST_DIR.to_str().unwrap()),
            setenv(ce::GNG_AGENT_MESSAGE_PREFIX, &message_prefix),
        ];

        let rust_log = std::env::var("RUST_LOG");
        if rust_log.is_ok() {
            let mut env_var = OsString::from("--setenv=RUST_LOG=");
            env_var.push(rust_log.unwrap());
            result.push(env_var)
        }

        result.push(bind(
            true,
            &self.build_script,
            &Path::new("/gng/build.rhai"),
        ));

        result.append(&mut extra);

        result
    }

    #[tracing::instrument(level = "debug")]
    fn run_agent(
        &mut self,
        args: &Vec<OsString>,
        new_mode: &Mode,
        message_prefix: String,
    ) -> eyre::Result<()> {
        tracing::debug!("Starting gng-build-agent in systemd-nspawn");
        let mut child = std::process::Command::new(&self.nspawn_binary)
            .args(args)
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        tracing::trace!("Processing output of gng-build-agent");
        self.handle_agent_output(&mut child, new_mode, message_prefix)?;

        tracing::trace!("Waiting for gng-build-agent to finish");
        let exit_status = child.wait()?;

        match exit_status.code() {
            None => Err(eyre::eyre!("gng-build-agent was killed by a signal.")),
            Some(code) if code == 0 => Ok(()),
            Some(code) => Err(eyre::eyre!(
                "gng-build-agent failed with exit code {}.",
                code
            )),
        }
    }

    /// Switch into a new `Mode` of operation
    #[tracing::instrument(level = "trace")]
    fn switch_mode(&mut self, new_mode: &Mode) -> eyre::Result<()> {
        tracing::debug!("Switching mode to {:?}", new_mode);

        for h in self.message_handlers.iter_mut() {
            h.prepare(new_mode)?;
        }

        let message_prefix = random_string(MESSAGE_PREFIX_LEN);

        // Start agent:
        let nspawn_args = self.mode_arguments(new_mode, &message_prefix);
        assert!(!nspawn_args.is_empty());

        self.run_agent(&nspawn_args, new_mode, message_prefix)?;

        for h in self.message_handlers.iter_mut() {
            h.verify(new_mode)?;
        }

        Ok(())
    }

    fn process_line(&mut self, mode: &Mode, message_prefix: &str, line: &str) -> eyre::Result<()> {
        lazy_static::lazy_static! {
            static ref PREFIX: String =
                std::env::var(ce::GNG_AGENT_OUTPUT_PREFIX).unwrap_or(String::from("AGENT> "));
        }

        let (message_type, line) = find_type_and_contents(&message_prefix, &line);
        if message_type == "" {
            println!("{}{}", *PREFIX, line);
        } else {
            tracing::trace!("{}MSG_{}: {}", *PREFIX, message_type, line);

            let message_type = gng_build_shared::MessageType::try_from(String::from(message_type))
                .map_err(|e| eyre::eyre!(e))?;
            for h in self.message_handlers.iter_mut() {
                if h.handle(mode, &message_type, line)? {
                    break;
                }
            }
        }
        Ok(())
    }

    fn handle_agent_output(
        &mut self,
        child: &mut std::process::Child,
        mode: &Mode,
        message_prefix: String,
    ) -> eyre::Result<()> {
        let reader = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| eyre::eyre!("Could not capture stdout of gng-build-agent."))?,
        );

        for line in reader.lines() {
            match line {
                Ok(line) => self.process_line(mode, &message_prefix, &line)?,
                Err(e) => return Err(eyre::eyre!(e)),
            }
        }

        let exit_status = child.wait()?;

        match exit_status.code() {
            Some(0) => Ok(()),
            Some(exit_code) => Err(eyre::eyre!(format!(
                "Agent failed with exit-status: {}.",
                exit_code
            ))),
            None => Err(eyre::eyre!(format!("Agent killed by signal."))),
        }
    }

    /// Process a build
    #[tracing::instrument(level = "debug")]
    pub fn process(&mut self) -> eyre::Result<()> {
        let mut mode = Some(Mode::default());

        while mode.is_some() {
            let m = mode.expect("Mode was some!");
            self.switch_mode(&m)?;
            mode = m.next();
        }

        Ok(())
    }

    /// Clean up after a build
    #[tracing::instrument(level = "debug")]
    pub fn clean_up(&mut self) -> eyre::Result<()> {
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
