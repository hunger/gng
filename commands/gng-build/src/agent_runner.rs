// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A `Runner` customized for use in `gng-build`

use gng_build_shared::constants::container as cc;
use gng_build_shared::constants::environment as ce;
use gng_contained_command::{Binding, Runner, RunnerBuilder};

use std::convert::TryFrom;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use eyre::{eyre, Result};
use rand::Rng;

// ----------------------------------------------------------------------
// - Constants:
// ----------------------------------------------------------------------

const MESSAGE_PREFIX_LEN: usize = 8;

// ----------------------------------------------------------------------
// - Helpers:
// ----------------------------------------------------------------------

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

#[tracing::instrument(level = "debug", skip(child, message_callback))]
fn handle_agent_output(
    mut child: std::process::Child,
    message_prefix: &str,
    message_callback: &impl Fn(&gng_build_shared::MessageType, &str) -> eyre::Result<()>,
) -> Result<()> {
    tracing::debug!("Handling output of build-agent");

    let stderr = BufReader::new(
        child
            .stderr
            .take()
            .ok_or_else(|| eyre!("Could not capture stderr of gng-build-agent."))?,
    );

    let stderr_thread = std::thread::spawn(move || {
        lazy_static::lazy_static! {
            static ref PREFIX: String =
                std::env::var(ce::GNG_AGENT_ERROR_PREFIX).unwrap_or_else(|_| String::from("AGENT[stderr]> "));
        }

        tracing::debug!("STDERR handler thread is up");

        for line in stderr.lines() {
            match line {
                Ok(line) => eprintln!("{}{}", *PREFIX, line),
                Err(e) => eprintln!("ERROR: {}", e),
            }
        }

        tracing::debug!("STDERR handler thread is done");
    });

    tracing::debug!("Processing STDOUT  in main thread.");

    let reader = BufReader::new(
        child
            .stdout
            .take()
            .ok_or_else(|| eyre!("Could not capture stdout of gng-build-agent."))?,
    );

    for line in reader.lines() {
        match line {
            Ok(line) => process_line(message_prefix, message_callback, &line)?,
            Err(e) => return Err(eyre!(e)),
        }
    }

    tracing::debug!("STDOUT handling done.");

    stderr_thread
        .join()
        .expect("Failed to join stderr handler thread.");

    tracing::debug!("STDERR thread was joined.");

    let exit_status = child.wait()?;

    tracing::debug!(
        "build-agent has finished with exit status {:?}",
        exit_status
    );

    match exit_status.code() {
        Some(0) => Ok(()),
        Some(exit_code) => Err(eyre!("Agent failed with exit-status: {}.", exit_code)),
        None => Err(eyre!("Agent killed by signal.")),
    }
}

fn process_line(
    message_prefix: &str,
    message_callback: &impl Fn(&gng_build_shared::MessageType, &str) -> eyre::Result<()>,
    line: &str,
) -> Result<()> {
    lazy_static::lazy_static! {
        static ref PREFIX: String =
            std::env::var(ce::GNG_AGENT_OUTPUT_PREFIX).unwrap_or_else(|_| String::from("AGENT[stdout]> "));
    }

    let (message_type, line) = find_type_and_contents(message_prefix, line);
    if message_type.is_empty() {
        println!("{}{}", *PREFIX, line);
    } else {
        let message_type = gng_build_shared::MessageType::try_from(String::from(message_type))
            .map_err(|e| eyre!(e))?;
        message_callback(&message_type, line)?;
    }
    Ok(())
}

fn root_directory(scratch: &std::path::Path) -> std::path::PathBuf {
    scratch.join("rootfs")
}
fn work_directory(scratch: &std::path::Path) -> std::path::PathBuf {
    scratch.join("work")
}

fn install_directory(scratch: &std::path::Path) -> std::path::PathBuf {
    scratch.join("install")
}

fn create_directory_tree(scratch: &std::path::Path) -> Result<()> {
    std::fs::create_dir(&root_directory(scratch))?;
    std::fs::create_dir(&root_directory(scratch).join("usr"))?;
    std::fs::create_dir(&work_directory(scratch))?;
    std::fs::create_dir(&install_directory(scratch))?;

    Ok(())
}

// ----------------------------------------------------------------------
// - AgentRunner:
// ----------------------------------------------------------------------

/// A Specialized `Runner` to run `gng-build-agent`
pub struct AgentRunner {
    runner: Runner,
    scratch_directory: PathBuf,
}

impl AgentRunner {
    /// Constructor
    ///
    /// # Errors
    /// May return an `Error` when some of the provided directories are not found
    pub fn new(
        scratch_directory: &Path,
        agent_binary: &Path,
        lua_directory: &Path,
        build_script: &Path,
        nspawn_binary: &Path,
    ) -> Result<Self> {
        let scratch_directory = scratch_directory.to_path_buf();

        if !scratch_directory.is_dir() {
            return Err(eyre::eyre!(
                "Scratch directory \"{}\" does not exist",
                scratch_directory.to_string_lossy()
            ));
        }

        create_directory_tree(&scratch_directory)?;

        let mut builder = RunnerBuilder::new(root_directory(&scratch_directory))
            .systemd_nspawn(&gng_core::validate_executable(nspawn_binary)?)
            .add_binding(Binding::tmpfs(&cc::GNG_DIR))
            .add_binding(Binding::ro(
                &gng_core::validate_executable(agent_binary)?,
                &cc::GNG_BUILD_AGENT_EXECUTABLE,
            ))
            .add_binding(Binding::ro(build_script, &cc::GNG_BUILD_SCRIPT))
            .add_binding(Binding::ro(lua_directory, &cc::GNG_LUA_DIR))
            .add_environment(format!(
                "{}={}",
                ce::GNG_BUILD_AGENT,
                cc::GNG_BUILD_AGENT_EXECUTABLE
                    .to_str()
                    .expect("Default value was invalid")
            ))
            .add_environment(format!(
                "{}={}",
                ce::GNG_BUILD_SCRIPT,
                cc::GNG_BUILD_SCRIPT
                    .to_str()
                    .expect("Default value was invalid")
            ))
            .add_environment(format!(
                "{}={}",
                ce::GNG_WORK_DIR,
                cc::GNG_WORK_DIR
                    .to_str()
                    .expect("Default value was invalid")
            ))
            .add_environment(format!(
                "{}={}",
                ce::GNG_INST_DIR,
                cc::GNG_INST_DIR
                    .to_str()
                    .expect("Default value was invalid")
            ))
            .add_environment(format!(
                "{}={}",
                ce::GNG_LUA_DIR,
                cc::GNG_LUA_DIR.to_str().expect("Default value was invalid")
            ));
        if let Ok(gng_log) = std::env::var("GNG_LOG") {
            builder = builder.add_environment(format!("GNG_LOG={}", gng_log));
        }

        if let Ok(gng_log_format) = std::env::var("GNG_LOG_FORMAT") {
            builder = builder.add_environment(format!("GNG_LOG_FORMAT={}", gng_log_format));
        }

        Ok(Self {
            runner: builder.build(),
            scratch_directory,
        })
    }

    /// Get the root directory used in the container
    #[must_use]
    pub fn root_directory(&self) -> std::path::PathBuf {
        root_directory(&self.scratch_directory)
    }

    /// Get the work directory used in the container
    #[must_use]
    pub fn work_directory(&self) -> std::path::PathBuf {
        work_directory(&self.scratch_directory)
    }

    /// Get the install directory used in the container
    #[must_use]
    pub fn install_directory(&self) -> std::path::PathBuf {
        install_directory(&self.scratch_directory)
    }

    fn create_command(
        &self,
        mode: &crate::Mode,
        message_prefix: &str,
    ) -> gng_contained_command::Command {
        let builder = gng_contained_command::CommandBuilder::new(&cc::GNG_BUILD_AGENT_EXECUTABLE)
            .add_environment(format!(
                "{}={}",
                &ce::GNG_AGENT_MESSAGE_PREFIX,
                message_prefix
            ));

        let usr_directory = self.root_directory().join("usr");

        let builder = match mode {
            crate::Mode::Query => builder
                .add_argument(&"query")
                .add_binding(Binding::ro(&self.work_directory(), &cc::GNG_WORK_DIR))
                .add_binding(Binding::tmpfs(&cc::GNG_INST_DIR)),
            crate::Mode::Prepare => builder
                .add_argument(&"prepare")
                .add_binding(Binding::rw(&self.work_directory(), &cc::GNG_WORK_DIR))
                .add_binding(Binding::tmpfs(&cc::GNG_INST_DIR)),
            crate::Mode::Build => builder
                .add_argument(&"build")
                .add_binding(Binding::rw(&self.work_directory(), &cc::GNG_WORK_DIR))
                .add_binding(Binding::tmpfs(&cc::GNG_INST_DIR)),
            crate::Mode::Check => builder
                .add_argument(&"check")
                .add_binding(Binding::rw(&self.work_directory(), &cc::GNG_WORK_DIR))
                .add_binding(Binding::tmpfs(&cc::GNG_INST_DIR)),
            crate::Mode::Install => builder
                .add_argument(&"install")
                .add_binding(Binding::ro(&self.work_directory(), &cc::GNG_WORK_DIR))
                .add_binding(Binding::tmpfs(&cc::GNG_INST_DIR))
                .add_binding(Binding::overlay(
                    &[&usr_directory, &self.install_directory()],
                    &PathBuf::from("/usr"),
                )),
            crate::Mode::Package => builder
                .add_argument(&"package")
                .add_binding(Binding::rw(&self.work_directory(), &cc::GNG_WORK_DIR))
                .add_binding(Binding::rw(&self.install_directory(), &cc::GNG_INST_DIR)),
        };
        builder.build()
    }

    /// Run a `gng-build-agent` in the specified mode
    #[tracing::instrument(level = "debug", skip(self, message_callback))]
    pub fn run(
        &self,
        mode: &crate::Mode,
        message_callback: &impl Fn(&gng_build_shared::MessageType, &str) -> eyre::Result<()>,
    ) -> Result<()> {
        tracing::debug!("Running in mode {:?}.", mode);
        let message_prefix = random_string(MESSAGE_PREFIX_LEN);
        let command = self.create_command(mode, &message_prefix);

        tracing::debug!("Running container");
        let child = self.runner.run(&command)?;

        handle_agent_output(child, &message_prefix, message_callback)
    }
}
