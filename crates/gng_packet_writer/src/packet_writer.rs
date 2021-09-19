// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! Trait to handle package writing.

use gng_core::{Name, Version};

// ----------------------------------------------------------------------
// - PacketWriter:
// ----------------------------------------------------------------------

/// An interface to create different kinds of Packets
pub trait PacketWriter {
    /// Add a directory into the packet.
    ///
    /// # Errors
    /// Returns mostly `Error::Io`
    fn add_directory(
        &mut self,
        packet_path: &std::path::Path,
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()>;

    /// Add a buffer into the packet.
    ///
    /// # Errors
    /// Returns mostly `Error::Io`
    fn add_buffer(
        &mut self,
        packet_path: &std::path::Path,
        data: &[u8],
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()>;

    /// Add a file into the packet.
    ///
    /// # Errors
    /// Returns mostly `Error::Io`
    fn add_file(
        &mut self,
        packet_path: &std::path::Path,
        on_disk_path: &std::path::Path,
        size: u64,
        mode: u32,
        user_id: u64,
        group_id: u64,
    ) -> eyre::Result<()>;

    /// Add a link into the packet.
    ///
    /// # Errors
    /// Returns mostly `Error::Io`
    fn add_link(
        &mut self,
        packet_path: &std::path::Path,
        target_path: &std::path::Path,
    ) -> eyre::Result<()>;

    /// finish writing the packet.
    ///
    /// # Errors
    /// Depends on the actual Writer being used.
    fn finish(&mut self) -> eyre::Result<Option<std::path::PathBuf>>;
}

/// The product of a `PacketWriterFactory`
pub type BoxedPacketWriter = Box<dyn PacketWriter>;
/// A type for factories of `PacketWriter`
pub type PacketWriterFactory =
    dyn Fn(&std::path::Path, &Name, &Option<Name>, &Version) -> eyre::Result<Box<dyn PacketWriter>>;

/// Create a default packet writer
///
/// # Errors
/// Depends on the actual `PacketWriter` being created.
pub fn create_packet_writer(
    packet_path: &std::path::Path,
    packet_name: &Name,
    facet_name: &Option<Name>,
    version: &Version,
    metadata: Vec<u8>,
    policy: crate::PacketPolicy,
) -> eyre::Result<BoxedPacketWriter> {
    Ok(Box::new(crate::packet_writer_impl::PacketWriterImpl::new(
        packet_path,
        packet_name,
        facet_name,
        version,
        metadata,
        policy,
    )))
}
