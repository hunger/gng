// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! The `gng-repo` binary.

// Setup warnings/errors:
#![forbid(unsafe_code)]
#![deny(
    bare_trait_objects,
    unused_doc_comments,
    unused_import_braces,
    missing_docs
)]
// Clippy:
#![warn(clippy::all, clippy::nursery, clippy::pedantic)]
#![allow(clippy::non_ascii_literal, clippy::module_name_repetitions)]

use gng_db::{
    db::Db, LocalRepository, RemoteRepository, Repository, RepositoryRelation, RepositorySource,
    Uuid,
};

use clap::Clap;
use eyre::{Result, WrapErr};

use std::convert::TryFrom;
use std::{path::PathBuf, str::FromStr};

// - Helper:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
#[clap(name = "gng-repo", about = "A repository query tool for GnG.")]
struct Args {
    /// the directory containing the Lua runtime environment
    #[clap(
        long = "db-directory",
        parse(from_os_str),
        env = "GNG_DB_DIR",
        value_name = "DIR"
    )]
    db_directory: PathBuf,

    #[clap(subcommand)]
    command: Commands,

    #[clap(flatten)]
    logging: gng_shared::log::LogArgs,
}

#[derive(Debug, Clap)]
enum Commands {
    Internal(InternalCommands),
    Repository(RepositoryCommands),
    Packet(PacketCommands),
}

// ----------------------------------------------------------------------
// - RepositoryCommands:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
struct RepositoryCommands {
    #[clap(subcommand)]
    sub_command: RepositorySubCommands,
}

#[derive(Debug, Clap)]
enum RepositorySubCommands {
    /// List known repositories
    #[clap(display_order = 500)]
    List(RepositoryListCommand),
    /// Add a new repository
    #[clap(display_order = 500)]
    Add(RepositoryAddCommand),
    /// Remove a known repository
    #[clap(display_order = 500)]
    Remove(RepositoryRemoveCommand),
}

#[derive(Debug, Clap)]
struct RepositoryListCommand {
    #[clap(display_order = 1500, long)]
    json: bool,
}

#[derive(Debug, Clap)]
struct RepositoryAddCommand {
    /// Name of the repository
    name: String,
    /// The repository UUID to use (generated if unset)
    #[clap(long = "uuid", value_name = "UUID")]
    uuid: Option<String>,
    /// Repository priority (higher values are used first, default is 1000)
    #[clap(long, value_name = "PRIORITY", default_value("1000"))]
    priority: u32,

    // Source:
    /// The URL to pull repository data from
    #[clap(long, value_name = "URL", group = "source")]
    remote_url: Option<String>,
    /// The base URL for packet downloads
    #[clap(long, value_name = "URL", requires = "remote-url")]
    packets_url: Option<String>,

    /// The place to check for sources to build locally
    #[clap(long, value_name = "DIR", group = "source")]
    sources_base_directory: Option<std::path::PathBuf>,
    /// The base URL for packet downloads
    #[clap(long, value_name = "DIR", requires = "sources-base-directory")]
    export_directory: Option<std::path::PathBuf>,

    // Relation:
    /// The place to check for sources to build locally
    #[clap(long = "override", value_name = "REPO", group = "relation")]
    override_repository: Option<String>,
    /// The base URL for packet downloads
    #[clap(long, value_name = "REPO", value_delimiter = ",", group = "relation")]
    dependencies: Vec<String>,
}

#[derive(Debug, Clap)]
struct RepositoryRemoveCommand {
    /// Name of the repository
    name: String,
}

fn handle_repository_command(db: &mut impl Db, cmd: &RepositoryCommands) -> Result<()> {
    match &cmd.sub_command {
        RepositorySubCommands::List(cmd) => handle_repository_list_command(db, cmd),
        RepositorySubCommands::Add(cmd) => handle_repository_add_command(db, cmd),
        RepositorySubCommands::Remove(cmd) => handle_repository_remove_command(db, cmd),
    }
}

fn print_json(repository: &Repository) -> Result<()> {
    println!("{}", repository.to_json()?);
    Ok(())
}

fn print_human(repository: &Repository) {
    println!("{}", repository.to_pretty_string())
}

#[tracing::instrument(level = "trace", skip(db))]
fn handle_repository_list_command(db: &mut impl Db, cmd: &RepositoryListCommand) -> Result<()> {
    let repositories = db.list_repositories();

    if !repositories.is_empty() {
        for r in &repositories {
            if cmd.json {
                print_json(r)?;
            } else {
                print_human(r);
            }
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip(db))]
fn handle_repository_add_command(db: &mut impl Db, cmd: &RepositoryAddCommand) -> Result<()> {
    let uuid = match &cmd.uuid {
        Some(u) => Uuid::from_str(u).wrap_err("Invalid UUID provided on command line"),
        None => Ok(Uuid::new_v4()),
    }?;

    let source = match &cmd.remote_url {
        Some(remote_url) => RepositorySource::Remote(RemoteRepository {
            remote_url: remote_url.clone(),
            packets_url: cmd.packets_url.clone(),
        }),
        None => RepositorySource::Local(LocalRepository {
            sources_base_directory: cmd
                .sources_base_directory
                .as_ref()
                .expect("Clap should have made sure this is Some!")
                .clone(),
            export_directory: cmd.export_directory.clone(),
        }),
    };

    let relation = if let Some(override_repository) = &cmd.override_repository {
        RepositoryRelation::Override(
            db.resolve_repository(override_repository)
                .ok_or_else(|| eyre::eyre!("Override repository not found."))?,
        )
    } else {
        let dependencies = cmd
            .dependencies
            .iter()
            .map(|s| db.resolve_repository(s))
            .collect::<Option<Vec<Uuid>>>()
            .ok_or_else(|| eyre::eyre!("Failed to resolve dependency repository."))?;
        RepositoryRelation::Dependency(dependencies)
    };

    let data = Repository {
        name: gng_shared::Name::try_from(&cmd.name[..])?,
        uuid,
        priority: cmd.priority,
        relation,
        source,
    };

    db.add_repository(data)
        .wrap_err("Failed to add new repository")
}

#[tracing::instrument(level = "trace", skip(db))]
fn handle_repository_remove_command(db: &mut impl Db, cmd: &RepositoryRemoveCommand) -> Result<()> {
    db.remove_repository(
        &db.resolve_repository(&cmd.name).ok_or_else(|| {
            eyre::eyre!("Could not resolve \"{}\" to a known repository.", cmd.name)
        })?,
    )
    .wrap_err("Failed to remove repository")
}

// ----------------------------------------------------------------------
// - PacketCommands:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
struct PacketCommands {
    #[clap(subcommand)]
    sub_command: PacketSubCommands,
}

#[derive(Debug, Clap)]
enum PacketSubCommands {
    /// List known packets
    #[clap(display_order = 500)]
    List(PacketListCommand),
    /// Adopt a new packet into a `Repository`
    #[clap(display_order = 500)]
    Add(PacketAdoptCommand),
    /// Remove a packet
    #[clap(display_order = 500)]
    Remove(PacketRemoveCommand),
}

#[derive(Debug, Clap)]
struct PacketListCommand {
    #[clap(display_order = 1500, long)]
    json: bool,
}

#[derive(Debug, Clap)]
struct PacketAdoptCommand {
    /// Name of the repository
    name: String,
    /// The URL to pull repository data from
    #[clap(long, value_name = "URL")]
    pull_url: Option<String>,
    /// The base URL for packet downloads
    #[clap(long = "packet-url", value_name = "URL")]
    packet_base_url: String,
    /// Declare repository dependencies separated by comma
    #[clap(long, value_name = "NAME", value_delimiter = ",")]
    dependencies: Vec<String>,
    /// Repository priority (higher values are used first, default is 1000)
    #[clap(long, value_name = "PRIORITY", default_value("1000"))]
    priority: u32,
    /// Directory containing source packages used to build the packets of this repository
    #[clap(long = "sources", parse(from_os_str), value_name = "DIR")]
    source_base_directory: Option<std::path::PathBuf>,
    /// The repository UUID to use (generated if unset)
    #[clap(long = "uuid", value_name = "UUID")]
    uuid: Option<String>,
}

#[derive(Debug, Clap)]
struct PacketRemoveCommand {
    /// Name of the repository
    name: String,
}

#[tracing::instrument(level = "trace", skip(db))]
fn handle_packet_command(db: &mut impl Db, cmd: &PacketCommands) -> Result<()> {
    Ok(())
}

// ----------------------------------------------------------------------
// - InternalCommands:
// ----------------------------------------------------------------------

#[derive(Debug, Clap)]
struct InternalCommands {
    #[clap(subcommand)]
    sub_command: InternalSubCommands,
}

#[derive(Debug, Clap)]
enum InternalSubCommands {
    Metadata,
}

#[tracing::instrument(level = "trace", skip(db))]
fn handle_internal_command(db: &mut impl Db, cmd: &InternalCommands) -> Result<()> {
    match cmd.sub_command {
        InternalSubCommands::Metadata => Ok(()),
    }
}

// ----------------------------------------------------------------------
// - Entry Point:
// ----------------------------------------------------------------------

/// Entry point of the `gng-build` binary.
fn main() -> Result<()> {
    let args = Args::parse();

    let _app_span = args
        .logging
        .setup_logging()
        .wrap_err("Failed to set up logging.")?;

    tracing::debug!("Command line arguments: {:#?}", args);

    let mut db = gng_db::open(&args.db_directory).wrap_err(format!(
        "Failed to open repository at \"{}\".",
        args.db_directory.to_string_lossy()
    ))?;

    match args.command {
        Commands::Internal(cmd) => handle_internal_command(&mut db, &cmd),
        Commands::Repository(cmd) => handle_repository_command(&mut db, &cmd),
        Commands::Packet(cmd) => handle_packet_command(&mut db, &cmd),
    }
}
