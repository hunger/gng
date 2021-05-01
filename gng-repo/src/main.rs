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

use std::convert::TryFrom;
use std::{path::PathBuf, str::FromStr};

use clap::Clap;
use eyre::{Result, WrapErr};

use gng_repository::repository_db::RepositoryDb;

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
    /// The URL to pull repository data from
    #[clap(long, value_name = "URL")]
    pull_url: Option<String>,
    /// The base URL for packet downloads
    #[clap(long = "packet-url", value_name = "URL")]
    packet_base_url: String,
    /// Declare repository dependencies separated by comma
    #[clap(long, value_name = "NAME", value_delimiter = ",")]
    dependencies: Vec<String>,
    /// Declare repository tags separated by comma
    #[clap(long, value_name = "TAG", value_delimiter = ",")]
    tags: Vec<String>,
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
struct RepositoryRemoveCommand {
    /// Name of the repository
    name: String,
}

fn handle_repository_command(db: &mut impl RepositoryDb, cmd: &RepositoryCommands) -> Result<()> {
    match &cmd.sub_command {
        RepositorySubCommands::List(cmd) => Ok(handle_repository_list_command(db, cmd)),
        RepositorySubCommands::Add(cmd) => handle_repository_add_command(db, cmd),
        RepositorySubCommands::Remove(cmd) => handle_repository_remove_command(db, cmd),
    }
}

fn print_json(repository: &gng_repository::Repository) {
    let mut dependency_str = String::new();
    let mut is_first = true;
    for d in &repository.dependencies {
        if !is_first {
            dependency_str.push(',');
        }
        is_first = false;
        dependency_str.push('"');
        dependency_str.push_str(&d.to_string());
        dependency_str.push('"');
    }

    let pull_url = repository.pull_url.as_ref().cloned().unwrap_or_default();
    let pull_url = if pull_url.is_empty() {
        String::new()
    } else {
        format!(r#""pull_url"="{}","#, pull_url)
    };

    let sources_base = repository
        .sources_base_directory
        .as_ref()
        .cloned()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let sources_base = if pull_url.is_empty() {
        String::new()
    } else {
        format!(r#""sources_base_directory"="{}","#, sources_base)
    };

    println!(
        r#"{{"name"="{}","uuid"="{}","priority"={},{}"packet_base_url"="{}",{}"dependencies"=[{}]}}"#,
        repository.name,
        repository.uuid,
        repository.priority,
        &pull_url,
        repository.packet_base_url,
        &sources_base,
        dependency_str
    );
}

fn print_human(repository: &gng_repository::Repository) {
    println!(
        "{}: {} ({})",
        &repository.priority, &repository.name, &repository.uuid
    );
    if let Some(url) = &repository.pull_url {
        println!("    Pull from      : \"{}\"", &url);
    } else {
        println!("    No remote packet data.");
    }
    println!("    Packet base URL: \"{}\"", &repository.packet_base_url);
    if let Some(sources) = &repository.sources_base_directory {
        println!("    Sources at     : \"{}\"", &sources.to_string_lossy());
    } else {
        println!("    No sources to build packets.");
    }
    if repository.dependencies.is_empty() {
        println!("    No repository dependencies.");
    } else {
        println!("    Dependencies   : {}", &repository.dependencies);
    }
    if repository.tags.is_empty() {
        println!("    No repository tags.");
    } else {
        println!("    Tags           : {}", &repository.tags);
    }
}

#[tracing::instrument(level = "trace", skip(db))]
fn handle_repository_list_command(db: &mut impl RepositoryDb, cmd: &RepositoryListCommand) {
    let repositories = db.list_repositories();

    if !repositories.is_empty() {
        for r in &repositories {
            if cmd.json {
                print_json(r);
            } else {
                print_human(r);
            }
        }
    }
}

#[tracing::instrument(level = "trace", skip(db))]
fn handle_repository_add_command(
    db: &mut impl RepositoryDb,
    cmd: &RepositoryAddCommand,
) -> Result<()> {
    let uuid = match &cmd.uuid {
        Some(u) => {
            gng_repository::Uuid::from_str(u).wrap_err("Invalid UUID provided on command line")
        }
        None => Ok(gng_repository::Uuid::new_v4()),
    }?;

    let data = gng_repository::Repository {
        name: gng_shared::Name::try_from(&cmd.name[..])?,
        uuid,
        priority: cmd.priority,
        pull_url: cmd.pull_url.clone(),
        packet_base_url: cmd.packet_base_url.clone(),
        sources_base_directory: cmd.source_base_directory.clone(),
        dependencies: gng_shared::Names::try_from(cmd.dependencies.clone())?,
        tags: gng_shared::Names::try_from(cmd.dependencies.clone())?,
    };

    db.add_repository(data)
        .wrap_err("Failed to add new repository")
}

#[tracing::instrument(level = "trace", skip(db))]
fn handle_repository_remove_command(
    db: &mut impl RepositoryDb,
    cmd: &RepositoryRemoveCommand,
) -> Result<()> {
    db.remove_repository(
        &gng_shared::Name::try_from(&cmd.name[..])
            .wrap_err("Invalid repository name was provided on command line")?,
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
fn handle_packet_command(db: &mut impl RepositoryDb, cmd: &PacketCommands) -> Result<()> {
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
fn handle_internal_command(db: &mut impl RepositoryDb, cmd: &InternalCommands) -> Result<()> {
    match cmd.sub_command {
        InternalSubCommands::Metadata => db
            .dump_metadata()
            .wrap_err("Repository storage backend failed to dump meta data."),
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

    let mut db = gng_repository::open(&args.db_directory).wrap_err(format!(
        "Failed to open repository at \"{}\".",
        args.db_directory.to_string_lossy()
    ))?;

    match args.command {
        Commands::Internal(cmd) => handle_internal_command(&mut db, &cmd),
        Commands::Repository(cmd) => handle_repository_command(&mut db, &cmd),
        Commands::Packet(cmd) => handle_packet_command(&mut db, &cmd),
    }
}
