// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle `gng-build-agent`s

// use crate::handler::Handler;
use crate::agent_runner::AgentRunner;
use crate::Mode;

use gng_build_shared::constants::container as cc;

use std::path::{Path, PathBuf};

use eyre::{eyre, Result, WrapErr};

// - Helper:
// ----------------------------------------------------------------------

fn build_script(pkgsrc_directory: &Path) -> Result<PathBuf> {
    let build_file = pkgsrc_directory.join(gng_build_shared::BUILD_SCRIPT);
    if !build_file.is_file() {
        return Err(eyre!(format!(
            "No {} file found in {}.",
            cc::GNG_BUILD_SCRIPT.to_str().unwrap(),
            pkgsrc_directory.to_string_lossy()
        )));
    }
    Ok(build_file)
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
        "Looking for agent relative to: \"{}\".",
        exe_directory.to_string_lossy()
    );

    for i in &[
        find_installed_agent(&exe_directory),
        find_development_agent(&exe_directory),
    ] {
        if gng_core::validate_executable(i).is_ok() {
            tracing::debug!("Using agent: \"{}\".", i.to_string_lossy());
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

    /// Evaluate a script file
    ///
    /// # Errors
    /// Generic Error
    pub fn build(&mut self, pkgsrc_directory: &Path) -> Result<CaseOfficer> {
        let mut temp_dirs = Vec::with_capacity(3);

        let root_directory =
            path_buf_or_tempdir(&None, "root-", &self.scratch_directory, &mut temp_dirs)?;

        if !root_directory.is_dir() {
            return Err(eyre!(
                "Root directory \"{}\" is not a directory.",
                root_directory.to_string_lossy(),
            ));
        }

        let work_directory = path_buf_or_tempdir(
            &self.work_directory,
            "work-",
            &self.scratch_directory,
            &mut temp_dirs,
        )?;
        if !work_directory.is_dir() {
            return Err(eyre!(
                "work directory \"{}\" is not a directory.",
                work_directory.to_string_lossy(),
            ));
        }

        let install_directory = path_buf_or_tempdir(
            &self.install_directory,
            "inst-",
            &self.scratch_directory,
            &mut temp_dirs,
        )?;
        if !install_directory.is_dir() {
            return Err(eyre!(
                "Install directory \"{}\" is not a directory.",
                install_directory.to_string_lossy(),
            ));
        }

        let agent = if let Some(a) = &self.agent {
            tracing::debug!(
                "Using provided gng-build-agent: \"{}\".",
                a.to_string_lossy()
            );
            gng_core::validate_executable(a)?
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

        let agent_runner = AgentRunner::new(
            &root_directory,
            &work_directory,
            &install_directory,
            &agent,
            &lua_directory,
            &build_script(pkgsrc_directory)?,
            &gng_core::validate_executable(&std::mem::take(&mut self.nspawn_binary))?,
        )?;

        Ok(CaseOfficer {
            agent_runner,

            temporary_directories: temp_dirs,
        })
    }
}

// ----------------------------------------------------------------------
// - CaseOfficer:
// ----------------------------------------------------------------------

/// The controller of the `gng-build-agent`
pub struct CaseOfficer {
    agent_runner: crate::agent_runner::AgentRunner,

    temporary_directories: Vec<tempfile::TempDir>,
}

impl CaseOfficer {
    /// Switch into a new `Mode` of operation
    #[tracing::instrument(level = "debug", skip(self, preparer, message_callback, clean_up))]
    fn switch_mode(
        &mut self,
        new_mode: &Mode,
        preparer: &impl Fn(&Mode) -> eyre::Result<()>,
        message_callback: &impl Fn(&Mode, &gng_build_shared::MessageType, &str) -> eyre::Result<()>,
        clean_up: &impl Fn(&Mode) -> eyre::Result<()>,
    ) -> Result<()> {
        tracing::debug!("Switching to mode {:?}", new_mode);

        preparer(new_mode)?;

        // Start agent:
        self.agent_runner
            .run(new_mode, &|mt, mc| message_callback(new_mode, mt, mc))?;

        clean_up(new_mode)?;

        tracing::debug!("Switching back out of Mode {:?}.", new_mode);

        Ok(())
    }

    /// Process a build
    #[tracing::instrument(level = "debug", skip(self, preparer, message_callback, clean_up))]
    pub fn process(
        &mut self,
        preparer: &impl Fn(&Mode) -> eyre::Result<()>,
        message_callback: &impl Fn(&Mode, &gng_build_shared::MessageType, &str) -> eyre::Result<()>,
        clean_up: &impl Fn(&Mode) -> eyre::Result<()>,
    ) -> Result<()> {
        tracing::debug!("Processing starts now.");

        let mut mode = Some(Mode::default());

        while mode.is_some() {
            let m = mode.expect("Mode was some!");
            self.switch_mode(&m, preparer, message_callback, clean_up)?;
            mode = m.next();
        }

        tracing::debug!("Processing done.");
        Ok(())
    }

    /// Clean up after a build
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn clean_up(&mut self) -> Result<()> {
        for td in self.temporary_directories.drain(..) {
            let p = td.path().to_owned();
            td.close().wrap_err(format!(
                "Failed to clean up temporary directory with path \"{}\".",
                p.to_string_lossy()
            ))?;
        }

        Ok(())
    }
}
