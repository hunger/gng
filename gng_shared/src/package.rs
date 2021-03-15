// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! Handle packing/unpacking a packets.

// - Helper:
// ----------------------------------------------------------------------

fn tar_path<'b, 'p>(
    base: &'b std::path::Path,
    path: &'p std::path::Path,
) -> crate::Result<&'p std::path::Path> {
    if path.is_absolute() {
        path.strip_prefix(base).map_err(|e| e.into())
    } else {
        Err(crate::Error::Unknown)
    }
}

// ∙----------------------------------------------------------------------
// - PacketWriter:
// ----------------------------------------------------------------------

/// An interface to create different kinds of Packets
pub trait PacketWriter {
    /// Add a directory into the packet.
    ///
    /// # Errors
    /// Returns mostly `Error::Io`
    fn add_dir(&mut self, path: &std::path::Path) -> crate::Result<()>;

    /// Add a file into the packet.
    ///
    /// # Errors
    /// Returns mostly `Error::Io`
    fn add_file(&mut self, path: &std::path::Path) -> crate::Result<()>;

    /// finish writing the packet.
    ///
    /// # Errors
    /// Depends on the actual Writer being used.
    fn finish(self) -> crate::Result<()>;
}

/// A type for factories of `PacketWriter`
pub type PacketWriterFactory =
    dyn Fn(&std::path::Path, &std::path::Path) -> crate::Result<Box<dyn PacketWriter>>;

/// Create the full packet name from the base name.
fn full_packet_path(packet_path: &std::path::Path) -> std::path::PathBuf {
    let mut result = packet_path.to_owned();
    result.set_extension("gng");
    result
}

/// Create a default packet writer
///
/// # Errors
/// Depends on the actual `PacketWriter` being created.
pub fn create_packet_writer(
    packet_path: &std::path::Path,
    base_dir: &std::path::Path,
) -> crate::Result<Box<dyn PacketWriter>> {
    // TODO: Make this configurable?
    let writer = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&full_packet_path(packet_path))?;
    let writer = zstd::Encoder::new(writer, 21)?;

    let mut tarball = PacketWriterImpl::new(base_dir, writer)?;

    tarball.set_cleanup_function(Box::new(|encoder: zstd::Encoder<_>| -> crate::Result<()> {
        match encoder.finish() {
            Err(e) => Err(e.into()),
            Ok(_) => Ok(()),
        }
    }));

    Ok(Box::new(tarball))
}

//  ----------------------------------------------------------------------
// - PacketWriterImpl:
// ----------------------------------------------------------------------

type CleanUpFunction<T> = Box<dyn FnOnce(T) -> crate::Result<()>>;

/// Write files and directories into a packet file
struct PacketWriterImpl<T>
where
    T: std::io::Write,
{
    base_dir: std::path::PathBuf,
    tarball: tar::Builder<T>,
    cleanup_function: CleanUpFunction<T>,
}

impl<T> PacketWriterImpl<T>
where
    T: std::io::Write,
{
    fn new(base_dir: &std::path::Path, packet_writer: T) -> crate::Result<Self> {
        if !base_dir.is_absolute() {
            return Err(crate::Error::Unknown);
        }

        let mut tarball = tar::Builder::new(packet_writer);
        tarball.follow_symlinks(false);

        Ok(Self {
            base_dir: base_dir.to_owned(),
            tarball,
            cleanup_function: Box::new(|_| Ok(())),
        })
    }

    fn set_cleanup_function(&mut self, function: CleanUpFunction<T>) {
        self.cleanup_function = function;
    }
}

impl<T> PacketWriter for PacketWriterImpl<T>
where
    T: std::io::Write,
{
    fn add_dir(&mut self, path: &std::path::Path) -> crate::Result<()> {
        let internal_path = tar_path(&self.base_dir, path)?;

        self.tarball
            .append_dir(path, internal_path)
            .map_err(|e| e.into())
    }

    fn add_file(&mut self, path: &std::path::Path) -> crate::Result<()> {
        let internal_path = tar_path(&self.base_dir, path)?;

        self.tarball
            .append_path_with_name(path, internal_path)
            .map_err(|e| e.into())
    }

    fn finish(self) -> crate::Result<()> {
        let inner = self.tarball.into_inner()?;
        (self.cleanup_function)(inner)
    }
}
