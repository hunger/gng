// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// A `Command` that is supposed to get run
#[derive(Clone, Debug)]
pub struct Command {
    /// The command to run
    pub(crate) command: PathBuf,
    /// The arguments passed to the `command`
    pub(crate) arguments: Vec<OsString>,
    /// The environment to run the command in
    pub(crate) environment: Vec<OsString>,
    /// Additional Bindings for this one command
    pub(crate) bindings: Vec<crate::Binding>,
    /// Allow network access (default: `false`)
    pub(crate) enable_network: bool,
    /// Enable private users (default: `true`)
    pub(crate) enable_private_users: bool,
}

/// A builder for `Command`
#[derive(Clone)]
pub struct CommandBuilder {
    /// The `Command` that is getting build
    command: Command,
}

impl CommandBuilder {
    /// Create a `new` `CommandBuilder` that sets up `to_execute` as the command that will be run.
    #[must_use]
    pub fn new(to_execute: &Path) -> Self {
        let command = Command {
            command: to_execute.into(),
            arguments: Vec::new(),
            environment: Vec::new(),
            bindings: Vec::new(),
            enable_network: false,
            enable_private_users: true,
        };

        Self { command }
    }

    /// Set arguments
    #[must_use]
    pub fn set_arguments(mut self, args: &[OsString]) -> Self {
        self.command.arguments = args.to_vec();
        self
    }

    /// Add one argument
    #[must_use]
    pub fn add_argument<S: Into<OsString>>(mut self, arg: S) -> Self {
        self.command.arguments.push(arg.into());
        self
    }

    /// Set environment
    #[must_use]
    pub fn set_environment(mut self, env: &[OsString]) -> Self {
        self.command.environment = env.to_vec();
        self
    }

    /// Add one environment variable
    #[must_use]
    pub fn add_environment<S: Into<OsString>>(mut self, env: S) -> Self {
        self.command.environment.push(env.into());
        self
    }

    /// Set bindings
    #[must_use]
    pub fn set_bindings(mut self, bind: &[crate::Binding]) -> Self {
        self.command.bindings = bind.to_vec();
        self
    }

    /// Add one binding
    #[must_use]
    pub fn add_binding(mut self, bind: crate::Binding) -> Self {
        self.command.bindings.push(bind);
        self
    }

    /// Add one binding
    #[must_use]
    pub fn enable_network(mut self, with_network: bool) -> Self {
        self.command.enable_network = with_network;
        self
    }

    /// Build the actual `Command`
    #[must_use]
    pub fn build(self) -> Command {
        self.command
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::CommandBuilder;

    // Name:
    #[test]
    fn command_create() {
        let cmd = CommandBuilder::new(&PathBuf::from("foo"))
            .add_argument("bar")
            .add_argument("baz")
            .add_environment("foo=bar")
            .build();
        assert_eq!(cmd.command, PathBuf::from("foo"));
        assert_eq!(
            cmd.arguments,
            vec!(OsString::from("bar"), OsString::from("baz"))
        );
        assert_eq!(cmd.environment, vec!(OsString::from("foo=bar")));
    }
}
