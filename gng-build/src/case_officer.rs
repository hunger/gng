// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object used to handle `gng-build-agent`s

use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

// - Helper:
// ----------------------------------------------------------------------

const BUILDER_MACHINE_ID: &str = "0bf95bb771364ef997e1df5eb3b26422";

#[tracing::instrument]
fn prepare_for_systemd_nspawn(root_directory: &Path) -> crate::Result<()> {
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

fn handle_agent_input(mut child: std::process::Child) -> crate::Result<()> {
    let reader = BufReader::new(child.stdout.take().ok_or_else(|| {
        crate::Error::AgentError(String::from("Could not capture stdout of gng-build-agent."))
    })?);

    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| println!("AGENT> {}", line));

    let exit_status = child.wait()?;

    match exit_status.code() {
        Some(0) => Ok(()),
        Some(exit_code) => Err(crate::Error::AgentError(format!(
            "Agent failed with exit-status: {}.",
            exit_code
        ))),
        None => Err(crate::Error::AgentError(format!("Agent killed by signal."))),
    }
}

// ----------------------------------------------------------------------
// - CaseOfficer:
// ----------------------------------------------------------------------

/// The controller of the `gng-build-agent`
#[derive(Debug)]
pub struct CaseOfficer {
    package_directory: PathBuf,

    root_directory: tempfile::TempDir,
    install_directory: tempfile::TempDir,
    result_directory: tempfile::TempDir,

    agent: PathBuf,

    nspawn_binary: PathBuf,
}

impl CaseOfficer {
    /// Create a new `CaseOfficer`
    #[tracing::instrument]
    pub fn new(
        work_directory: &Path,
        package_directory: &Path,
        agent: &Path,
    ) -> crate::Result<Self> {
        let nspawn_binary = Path::new("/usr/bin/systemd-nspawn");
        if !nspawn_binary.is_file() {
            return Err(crate::Error::FileMissing(nspawn_binary.to_path_buf()));
        }

        let root_dir = tempfile::Builder::new()
            .prefix("root-")
            .rand_bytes(6)
            .tempdir_in(&work_directory)?;
        prepare_for_systemd_nspawn(&root_dir.path())?;

        let install_dir = tempfile::Builder::new()
            .prefix("ínstall-")
            .rand_bytes(6)
            .tempdir_in(&work_directory)?;

        let result_dir = tempfile::Builder::new()
            .prefix("result-")
            .rand_bytes(6)
            .tempdir_in(&work_directory)?;

        Ok(CaseOfficer {
            package_directory: package_directory.to_path_buf(),

            root_directory: root_dir,
            install_directory: install_dir,
            result_directory: result_dir,

            agent: agent.to_path_buf(),
            nspawn_binary: nspawn_binary.to_path_buf(),
        })
    }

    fn mode_arguments(&self, mode: &crate::Mode) -> Vec<std::ffi::OsString> {
        let mut extra = match mode {
            crate::Mode::IDLE => vec![],
            crate::Mode::QUERY => vec![
                OsString::from("--read-only"),
                bind(
                    true,
                    self.install_directory.path(),
                    &Path::new("/gng/install"),
                ),
                bind(
                    true,
                    self.result_directory.path(),
                    &Path::new("/gng/results"),
                ),
                OsString::from("/gng/build-agent"),
                OsString::from("query"),
            ],
            crate::Mode::BUILD => vec![
                OsString::from("--read-only"),
                bind(
                    true,
                    self.install_directory.path(),
                    &Path::new("/gng/install"),
                ),
                bind(
                    true,
                    self.result_directory.path(),
                    &Path::new("/gng/results"),
                ),
                OsString::from("/gng/build-agent"),
                OsString::from("build"),
            ],
            crate::Mode::CHECK => vec![
                OsString::from("--read-only"),
                bind(
                    true,
                    self.install_directory.path(),
                    &Path::new("/gng/install"),
                ),
                bind(
                    true,
                    self.result_directory.path(),
                    &Path::new("/gng/results"),
                ),
                OsString::from("/gng/build-agent"),
                OsString::from("check"),
            ],
            crate::Mode::INSTALL => vec![
                OsString::from("--read-only"),
                bind(
                    false,
                    self.install_directory.path(),
                    &Path::new("/gng/install"),
                ),
                overlay(&vec![
                    self.root_directory.path().join("usr"),
                    self.install_directory.path().to_path_buf(),
                    PathBuf::from("/usr"),
                ]),
                bind(
                    true,
                    self.result_directory.path(),
                    &Path::new("/gng/results"),
                ),
                OsString::from("/gng/build-agent"),
                OsString::from("install"),
            ],
            crate::Mode::PACKAGE => vec![
                OsString::from("--read-only"),
                bind(
                    true,
                    self.install_directory.path(),
                    &Path::new("/gng/install"),
                ),
                bind(
                    true,
                    self.result_directory.path(),
                    &Path::new("/gng/results"),
                ),
                OsString::from("/gng/build-agent"),
                OsString::from("package"),
            ],
        };
        if extra.is_empty() {
            extra
        } else {
            let mut result = vec![
                OsString::from("--quiet"),
                OsString::from("--settings=off"),
                OsString::from("--register=off"),
                // OsString::from("-U"), // --private-users=pick or no user name-spacing
                OsString::from("--private-network"),
                OsString::from("--resolv-conf=off"),
                OsString::from("--timezone=off"),
                OsString::from("--link-journal=no"),
                bind(true, &self.agent, &Path::new("/gng/build-agent")),
                OsString::from("--directory"),
                OsString::from(self.root_directory.path().as_os_str()),
                OsString::from(format!("--uuid={}", BUILDER_MACHINE_ID)),
                OsString::from("--console=interactive"),
                OsString::from("--tmpfs=/gng"),
            ];

            let rust_log = std::env::var("RUST_LOG");
            if rust_log.is_ok() {
                let mut env_var = OsString::from("--setenv=RUST_LOG=");
                env_var.push(rust_log.unwrap());
                result.push(env_var)
            }

            result.append(&mut extra);
            result
        }
    }

    /// Switch into a new `Mode` of operation
    #[tracing::instrument]
    fn switch_mode(&mut self, new_mode: &crate::Mode) -> crate::Result<()> {
        tracing::debug!("Switching mode to {:?}", new_mode);

        // Start agent:
        let nspawn_args = self.mode_arguments(new_mode);
        assert!(!nspawn_args.is_empty());

        tracing::debug!(
            "Starting systemd-nspawn process with arguments {:?}.",
            nspawn_args
        );

        let child = std::process::Command::new(&self.nspawn_binary)
            .args(&nspawn_args)
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        handle_agent_input(child)?;

        Ok(())
    }

    /// Process a build
    pub fn process(&mut self) -> crate::Result<()> {
        let mut mode = crate::Mode::default();

        while mode != crate::Mode::IDLE {
            self.switch_mode(&mode)?;
            mode = mode.next()
        }

        Ok(())
    }
}
