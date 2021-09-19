// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

use crate::{Error, Result};

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::{ffi::OsString, os::unix::prelude::OsStrExt};

// - Constants:
// ----------------------------------------------------------------------

const BUILDER_MACHINE_ID: [u8; 32] = [
    b'0', b'b', b'f', b'9', b'5', b'b', b'b', b'7', b'7', b'1', b'3', b'6', b'4', b'e', b'f', b'9',
    b'9', b'7', b'e', b'1', b'd', b'f', b'5', b'e', b'b', b'3', b'b', b'2', b'6', b'4', b'2', b'2',
];

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn nspawn_arguments() -> Vec<OsString> {
    vec![
        OsString::from("--quiet"),
        OsString::from("--volatile=yes"),
        OsString::from("--settings=off"),
        OsString::from("--register=off"),
        OsString::from("--resolv-conf=off"),
        OsString::from("--timezone=off"),
        OsString::from("--link-journal=no"),
        OsString::from("--console=pipe"),
    ]
}
fn prepare_for_systemd_nspawn(root_directory: &Path) -> Result<()> {
    let usr_dir = root_directory.join("usr");
    if !usr_dir.exists() {
        std::fs::create_dir(usr_dir)?;
    }

    Ok(())
}

// ----------------------------------------------------------------------
// - Runner:
// ----------------------------------------------------------------------

/// The `Runner` that will run a `Command` in a container
#[derive(Debug)]
pub struct Runner {
    systemd_nspawn_binary: PathBuf,
    sudo_binary: PathBuf,
    root_directory: PathBuf,
    machine_id: [u8; 32],
    bindings: Vec<crate::Binding>,
    environment: Vec<OsString>,
}

impl Runner {
    fn environment_arguments(&self, command: &crate::Command) -> Vec<OsString> {
        self.environment
            .iter()
            .chain(command.environment.iter())
            .map(|e| {
                let mut result = OsString::from("--setenv=");
                result.push(&e);
                result
            })
            .collect()
    }

    fn bind_arguments(&self, command: &crate::Command) -> Vec<OsString> {
        self.bindings
            .iter()
            .chain(command.bindings.iter())
            .map(|b| match b {
                crate::Binding::TmpFS(target) => {
                    let mut result = OsString::from("--tmpfs=");
                    result.push(target.as_os_str());
                    result
                }
                crate::Binding::RW(mapping) => {
                    let mut result = OsString::from("--bind=");
                    result.push(mapping.source.as_os_str());
                    result.push(OsString::from(":"));
                    result.push(mapping.target.as_os_str());
                    result
                }
                crate::Binding::RO(mapping) => {
                    let mut result = OsString::from("--bind-ro=");
                    result.push(mapping.source.as_os_str());
                    result.push(OsString::from(":"));
                    result.push(mapping.target.as_os_str());
                    result
                }
                crate::Binding::Inaccessible(target) => {
                    let mut result = OsString::from("--inaccessible=");
                    result.push(target.as_os_str());
                    result
                }
                crate::Binding::Overlay(mapping) => {
                    let mut result = OsString::from("--overlay=");
                    for s in &mapping.sources {
                        result.push(s.as_os_str());
                        result.push(OsString::from(":"));
                    }
                    result.push(mapping.target.as_os_str());
                    result
                }
                crate::Binding::OverlayRO(mapping) => {
                    let mut result = OsString::from("--overlay-ro=");
                    for s in &mapping.sources {
                        result.push(s.as_os_str());
                        result.push(OsString::from(":"));
                    }
                    result.push(mapping.target.as_os_str());
                    result
                }
            })
            .collect()
    }

    /// Run a `Command`
    #[tracing::instrument(level = "debug")]
    pub fn run(&self, command: &crate::Command) -> Result<std::process::Child> {
        tracing::trace!("running...");

        if !self.root_directory.is_dir() {
            return Err(Error::Config(format!(
                "\"{}\" is not a directory.",
                self.root_directory.to_string_lossy()
            )));
        }
        let nspawn = self.systemd_nspawn_binary.clone();

        prepare_for_systemd_nspawn(&self.root_directory)?;

        let (binary, mut args) = if gng_core::is_root() {
            (nspawn, Vec::new())
        } else {
            (
                gng_core::validate_executable(&self.sudo_binary)?,
                vec![nspawn.into_os_string()],
            )
        };

        args.append(&mut nspawn_arguments());

        if !command.enable_network {
            args.push(OsString::from("--private-network"));
        }

        {
            let mut tmp = OsString::from("--uuid=");
            tmp.push(std::ffi::OsStr::from_bytes(&self.machine_id[..]));
            args.push(tmp);
        }

        args.append(&mut self.environment_arguments(command));
        args.append(&mut self.bind_arguments(command));

        if command.enable_private_users {
            let effective_uid = std::fs::metadata("/proc/self")
                .map(|m| m.uid())
                .expect("/proc/self should be accessible to this process!");
            args.push(OsString::from(format!(
                "--private-users={}:1",
                effective_uid
            )));
        }

        let mut dir_arg = OsString::from("--directory=");
        dir_arg.push(self.root_directory.as_os_str());
        args.push(dir_arg);

        // Actual Command:
        args.push(command.command.as_os_str().to_os_string());
        args.append(&mut command.arguments.clone());

        tracing::debug!(
            "Running: \"{}\"{}",
            binary.to_string_lossy(),
            args.iter()
                .map(|a| format!(" \"{}\"", a.to_string_lossy()))
                .collect::<String>()
        );

        let child = std::process::Command::new(&binary)
            .args(&args)
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        tracing::trace!("Container has started.");
        Ok(child)
    }
}

// - RunnerBuilder:
// ----------------------------------------------------------------------

/// The `Runner` that will run a `Command` in a container
pub struct RunnerBuilder {
    runner: Runner,
}

impl RunnerBuilder {
    /// Create a `RunnerBuilder` that will create a container in `root_directory`
    pub fn new<P: Into<PathBuf>>(root_directory: P) -> Self {
        let mut machine_id = [0_u8; 32];
        machine_id.copy_from_slice(&BUILDER_MACHINE_ID);

        RunnerBuilder {
            runner: Runner {
                systemd_nspawn_binary: PathBuf::from("/usr/bin/systemd-nspawn"),
                sudo_binary: PathBuf::from("/usr/bin/sudo"),
                root_directory: root_directory.into(),
                machine_id,
                environment: Vec::new(),
                bindings: Vec::new(),
            },
        }
    }

    /// Set the `machine_id` of the container
    #[must_use]
    pub fn machine_id(mut self, id: [u8; 32]) -> Self {
        self.runner.machine_id = id;
        self
    }

    /// Set the path to the `systemd-nspawn` binary, which defaults to `/usr/bin/systemd-nspawn`
    #[must_use]
    pub fn systemd_nspawn<P: Into<PathBuf>>(mut self, nspawn: P) -> Self {
        self.runner.systemd_nspawn_binary = nspawn.into();
        self
    }

    /// Set the path to the `systemd-nspawn` binary, which defaults to `/usr/bin/systemd-nspawn`
    #[must_use]
    pub fn sudo<P: Into<PathBuf>>(mut self, sudo: P) -> Self {
        self.runner.sudo_binary = sudo.into();
        self
    }

    /// Set bindings
    #[must_use]
    pub fn set_bindings(mut self, bind: &[crate::Binding]) -> Self {
        self.runner.bindings = bind.to_vec();
        self
    }

    /// Add one binding
    #[must_use]
    pub fn add_binding(mut self, bind: crate::Binding) -> Self {
        self.runner.bindings.push(bind);
        self
    }

    /// Set environment
    #[must_use]
    pub fn set_environment(mut self, env: &[OsString]) -> Self {
        self.runner.environment = env.to_vec();
        self
    }

    /// Add one environment variable
    #[must_use]
    pub fn add_environment<S: Into<OsString>>(mut self, env: S) -> Self {
        self.runner.environment.push(env.into());
        self
    }

    /// Build the actual `Runner`
    #[must_use]
    pub fn build(self) -> Runner {
        self.runner
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::RunnerBuilder;

    // Name:
    #[test]
    fn runner_create() {
        let runner = RunnerBuilder::new("/rootfs")
            .machine_id([b'a'; 32])
            .systemd_nspawn("/foo/bar/nspawn")
            .build();
        assert_eq!(
            runner.systemd_nspawn_binary,
            PathBuf::from("/foo/bar/nspawn")
        );
        assert_eq!(runner.machine_id, [b'a'; 32]);
    }
}
