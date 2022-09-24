// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

// cSpell: ignore runtime runtimes oursh

//! Code to work with `runtimes` for the build process

use std::path::{Path, PathBuf};

use eyre::{Result, WrapErr};

// ---------------------------------------------------------------------------
// Helpers:
// ---------------------------------------------------------------------------

fn name_from_directory(directory: &Path) -> Result<String> {
    Ok(directory
        .file_name()
        .ok_or_else(|| {
            eyre::eyre!(
                "Could not extract a runtime name from directory \"{}\".",
                directory.to_string_lossy()
            )
        })?
        .to_string_lossy()
        .to_string())
}

fn read_toml_file(file: &Path) -> Result<RuntimeFile> {
    let contents = std::fs::read_to_string(&file).wrap_err(format!(
        "Failed to read runtime.toml in \"{}\".",
        file.to_string_lossy()
    ))?;

    toml::from_str(&contents).wrap_err(format!(
        "Failed to parse runtime definition file \"{}\".",
        file.to_string_lossy()
    ))
}

fn validate(directory: &Path, sub_directory: &str) -> Result<()> {
    let abs_sub_directory = directory.join(sub_directory);
    if !abs_sub_directory.is_dir() {
        return Err(eyre::eyre!(
            "Runtime root directory \"{}\" has no \"{sub_directory}\" directory.",
            directory.to_string_lossy()
        ));
    }
    Ok(())
}

fn validate_runtime_fs(directory: &Path) -> Result<()> {
    if !directory.is_dir() {
        return Err(eyre::eyre!(
            "Runtime root directory \"{}\" is not a directory.",
            directory.to_string_lossy()
        ));
    }

    validate(directory, "dev")?;
    validate(directory, "proc")?;
    validate(directory, "run")?;
    validate(directory, "sys")?;
    validate(directory, "tmp")?;
    validate(directory, "usr")?;

    Ok(())
}

// ----------------------------------------------------------------------
// - Runtime:
// ----------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct RuntimeFile {
    runtime: RuntimeSection,
    environment: std::collections::HashMap<String, String>,
}

impl RuntimeFile {
    fn environment(&self) -> Vec<(String, String)> {
        self.environment
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
}

#[derive(serde::Deserialize)]
struct RuntimeSection {
    runner: Vec<String>,
}

/// A definition or a runtime for use during a package build
#[derive(Clone, Debug, serde::Deserialize)]
pub struct RuntimeDefinition {
    /// The name of the runtime
    pub name: String,
    /// The path to the runtime's root directory
    pub root_directory: PathBuf,
    /// The program to start inside the runtime.
    pub runner: Vec<String>,
    /// The environment to set up
    pub environment: Vec<(String, String)>,
}

impl RuntimeDefinition {
    fn from_runtime_directory(directory: &Path) -> Result<Self> {
        if !directory.is_dir() {
            return Err(eyre::eyre!(
                "Runtime directory \"{}\" is not a directory.",
                directory.to_string_lossy()
            ));
        }

        let name = name_from_directory(directory)?;

        // Read runtime.toml
        let runtime_file = read_toml_file(&directory.join("runtime.toml"))?;

        // Validate root directory:
        let root_directory = directory.join("rt");
        validate_runtime_fs(&root_directory)?;

        Ok(Self {
            name,
            root_directory,
            runner: runtime_file.runtime.runner.clone(),
            environment: runtime_file.environment(),
        })
    }

    /// Scan a runtime directory for actual `RuntimeDefinition`s
    ///
    /// # Errors
    ///
    /// Report an error for invalid runtimes
    pub fn scan_for_runtimes(runtime_directory: &Path) -> Result<Vec<Self>> {
        std::fs::read_dir(runtime_directory)?
            .filter_map(|e| match e {
                Ok(e) => Some(e),
                Err(_) => None,
            })
            .map(|e| Self::from_runtime_directory(&e.path()))
            .collect()
    }
}

#[cfg(test)]
mod tests {

    use super::RuntimeFile;

    #[test]
    fn test_runtime_toml() {
        let runtime_file: RuntimeFile = toml::from_str("[runtime]\nrunner = [ \"/min_rt/bin/oursh\", \"-c\", \"{}\" ]\n\n[environment]\n PATH = \"/min_rt/bin\"").unwrap();

        assert_eq!(
            runtime_file.runtime.runner,
            vec!["/min_rt/bin/oursh", "-c", "{}"]
        );
        assert_eq!(
            runtime_file.environment(),
            vec![("PATH".to_string(), "/min_rt/bin".to_string())]
        );
    }
}
