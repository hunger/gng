// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Logging setup code

use clap::Clap;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

// TODO: Can this be done DRY-er?

fn setup_pretty_logger() -> crate::Result<()> {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("GNG_LOG"))
        .try_init()
        .map_err(|e| crate::Error::Runtime {
            message: e.to_string(),
        })
}

fn setup_full_logger() -> crate::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("GNG_LOG"))
        .try_init()
        .map_err(|e| crate::Error::Runtime {
            message: e.to_string(),
        })
}

fn setup_compact_logger() -> crate::Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("GNG_LOG"))
        .try_init()
        .map_err(|e| crate::Error::Runtime {
            message: e.to_string(),
        })
}

fn setup_json_logger() -> crate::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("GNG_LOG"))
        .try_init()
        .map_err(|e| crate::Error::Runtime {
            message: e.to_string(),
        })
}

// ----------------------------------------------------------------------
// - LogFormat:
// ----------------------------------------------------------------------

/// The output format to be used for log messages
#[derive(Debug)]
pub enum LogFormat {
    /// Pretty, human-readable output of log messages.
    Pretty,
    /// Full output of log messages
    Full,
    /// Compact output of log messages.
    Compact,
    /// JSON output of log messages.
    Json,
}

impl std::str::FromStr for LogFormat {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "pretty" => Ok(Self::Pretty),
            "full" => Ok(Self::Full),
            "compact" => Ok(Self::Compact),
            "json" => Ok(Self::Json),
            _ => Err(crate::Error::Runtime {
                message: format!("\"{}\" is not a supported output kind for log messages", s),
            }),
        }
    }
}

// ----------------------------------------------------------------------
// - LogArgs:
// ----------------------------------------------------------------------

/// Logging related arguments for command line parsing
#[derive(Debug, Clap)]
pub struct LogArgs {
    /// Set the output format for
    #[clap(
        long,
        default_value = "pretty",
        display_order = 5000,
        env = "GNG_LOG_FORMAT",
        value_name = "pretty|full|compact|json"
    )]
    log_format: LogFormat,
}

impl LogArgs {
    /// Install a default tracing subscriber
    ///
    /// # Errors
    /// a `crate::Error::Runtime` is returned if the setup fails
    pub fn setup_logging(&self) -> crate::Result<()> {
        match self.log_format {
            LogFormat::Pretty => setup_pretty_logger(),
            LogFormat::Full => setup_full_logger(),
            LogFormat::Compact => setup_compact_logger(),
            LogFormat::Json => setup_json_logger(),
        }?;
        tracing::trace!("Tracing initialized.");
        Ok(())
    }
}
