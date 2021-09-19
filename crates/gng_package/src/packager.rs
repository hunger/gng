// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! The code to control the `Packet` creation process.

use crate::{FacetDefinition, PacketDefinition};

pub mod filtered;
pub mod storage;
pub mod switching;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

#[tracing::instrument(level = "debug", skip(packet, facet))]
fn storage_packager(
    packet: &PacketDefinition,
    facet: &FacetDefinition,
) -> eyre::Result<BoxedPackager> {
    Ok(Box::new(storage::StoragePackager::new(packet, facet)?))
}

// ----------------------------------------------------------------------
// - Packager:
// ----------------------------------------------------------------------

/// The `Packager` structure
pub trait Packager {
    /// A name used during debugging.
    fn debug_name(&self) -> String;

    /// Package a `Path`
    ///
    /// # Errors
    /// Return an `eyre::Result` when something goes wrong.
    fn package(&mut self, path: &crate::path::Path) -> eyre::Result<bool>;

    /// Finish packaging:
    ///
    /// # Errors
    /// Return an `eyre::Result` when something goes wrong.
    fn finish(&mut self) -> eyre::Result<Vec<std::path::PathBuf>>;
}

/// A boxed `Packager`
pub type BoxedPackager = Box<dyn Packager>;

/// A factory for a `Packager`
pub type PackagerFactory =
    dyn Fn(&PacketDefinition, &FacetDefinition) -> eyre::Result<BoxedPackager>;

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

#[tracing::instrument(level = "debug", skip(packet, facets, packager_factory))]
fn setup_faceted_action(
    packet: &PacketDefinition,
    facets: &[FacetDefinition],
    packager_factory: &PackagerFactory,
) -> eyre::Result<BoxedPackager> {
    let children = facets
        .iter()
        .filter(|f| {
            f.name
                .as_ref()
                .map_or(true, |n| !packet.merged_facets.contains(n))
        })
        .map(|f| {
            let facet_pretty_name =
                (f.name.as_ref()).map_or_else(String::new, |n| format!("-{}", n));
            let filter = f.filter.clone();
            Ok(Box::new(filtered::FilteredPackager::new(
                format!("{}{}", packet.name, facet_pretty_name),
                filter,
                packager_factory(packet, f)?,
            )) as BoxedPackager)
        })
        .collect::<eyre::Result<Vec<_>>>()?;
    tracing::trace!(
        "Packager created for packet \"{}\" -- using {} facets.",
        &packet.name,
        children.len(),
    );
    assert!(!children.is_empty());
    Ok(Box::new(switching::SwitchingPackager::new(children)))
}

// ----------------------------------------------------------------------
// - Constructor:
// ----------------------------------------------------------------------

/// Create a packet function for a set of `PacketDefinition`s and `FacetDefinition`s
///
/// # Errors
/// Returns an `eyre::Result` when something goes wrong.
#[tracing::instrument(level = "trace", skip(packets, facets))]
pub fn create_packager(
    packets: &[PacketDefinition],
    facets: &[FacetDefinition],
) -> eyre::Result<BoxedPackager> {
    create_packager_with_factory(packets, facets, &storage_packager)
}

/// Create a packet function for a set of `PacketDefinition`s and `FacetDefinition`s
///
/// # Errors
/// Returns an `eyre::Result` when something goes wrong.
#[tracing::instrument(level = "debug", skip(packets, facets, packager_factory))]
fn create_packager_with_factory(
    packets: &[PacketDefinition],
    facets: &[FacetDefinition],
    packager_factory: &PackagerFactory,
) -> eyre::Result<BoxedPackager> {
    tracing::debug!(
        "Creating packagers for {} packets and {} facets.",
        packets.len(),
        facets.len()
    );

    let children = packets
        .iter()
        .map(|p| {
            let packager = setup_faceted_action(p, facets, packager_factory)?;
            let filter = p.filter.clone();

            Ok(Box::new(filtered::FilteredPackager::new(
                p.name.to_string(),
                filter,
                packager,
            )) as BoxedPackager)
        })
        .collect::<eyre::Result<Vec<_>>>()?;

    tracing::trace!("Master packager created for {} packets.", children.len());
    Ok(Box::new(switching::SwitchingPackager::new(children)))
}
