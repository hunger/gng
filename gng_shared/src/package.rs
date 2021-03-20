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

// ----------------------------------------------------------------------
// - PacketWriter:
// ----------------------------------------------------------------------

/// Different types of paths
pub enum PathLeaf {
    /// A `File`
    File {
        /// The `File` name with extension, etc. but without the base directory part
        name: std::ffi::OsString,
        /// The permissions on the `File`
        mode: u32,
        /// The uid of the `File`
        uid: u32,
        /// The gid of the `File`
        gid: u32,
        /// The size of the `File` in bytes
        size: u64,
    },
    /// A `Link`
    Link {
        /// The `Link` name with extension, etc. but without the base directory part
        name: std::ffi::OsString,
        /// The `Link` target (complete with base directories, etc.!)
        target: std::path::PathBuf,
    },
    /// Just the directory, nothing in it:-)
    None,
}

/// A `Dir`
#[derive(Clone)]
pub struct Dir {
    /// The `Dir` name with extension, etc. but without the base directory part
    pub name: std::ffi::OsString,
    /// The permissions on the `Dir`
    pub mode: u32,
    /// The uid of the `Dir`
    pub uid: u32,
    /// The gid of the `Dir`
    pub gid: u32,
}

/// A full path
pub struct Path {
    /// Is this an absolute `Path`?
    pub is_absolute: bool,
    /// The directories up to the leaf
    pub directory: Vec<Dir>,
    /// The leaf node on the directory
    pub leaf: PathLeaf,
}

impl Path {
    /// Get the full path (abs or relative) stored in `Path`
    #[must_use]
    pub fn path(&self) -> std::path::PathBuf {
        let leaf_part = match &self.leaf {
            PathLeaf::File {
                name,
                mode: _,
                uid: _,
                gid: _,
                size: _,
            }
            | PathLeaf::Link { name, target: _ } => name.clone(),
            PathLeaf::None => std::ffi::OsString::new(),
        };
        let base_path = if self.is_absolute {
            std::path::PathBuf::from("/")
        } else {
            std::path::PathBuf::new()
        };

        let mut rel_path = self.directory.iter().fold(base_path, |a, b| {
            let mut result = a;
            result.push(&b.name);
            result
        });
        if !leaf_part.is_empty() {
            rel_path.push(&leaf_part);
        }

        rel_path
    }
}

/// An interface to create different kinds of Packets
pub trait PacketWriter {
    /// Add a directory into the packet.
    ///
    /// # Errors
    /// Returns mostly `Error::Io`
    fn add_path(&mut self, packet_path: &Path, on_disk_path: &std::path::Path)
        -> crate::Result<()>;

    /// finish writing the packet.
    ///
    /// # Errors
    /// Depends on the actual Writer being used.
    fn finish(self) -> crate::Result<()>;
}

/// A type for factories of `PacketWriter`
pub type PacketWriterFactory =
    dyn Fn(&std::path::Path) -> crate::Result<(Box<dyn PacketWriter>, std::path::PathBuf)>;

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
) -> crate::Result<(Box<dyn PacketWriter>, std::path::PathBuf)> {
    // TODO: Make this configurable?
    let full_name = full_packet_path(packet_path);

    let writer = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&full_name)?;
    let writer = zstd::Encoder::new(writer, 21)?;

    let mut tarball = PacketWriterImpl::new(writer)?;

    tarball.set_cleanup_function(Box::new(|encoder: zstd::Encoder<_>| -> crate::Result<()> {
        match encoder.finish() {
            Err(e) => Err(e.into()),
            Ok(_) => Ok(()),
        }
    }));

    Ok((Box::new(tarball), full_name))
}

// ----------------------------------------------------------------------
// - PacketWriterImpl:
// ----------------------------------------------------------------------

type CleanUpFunction<T> = Box<dyn FnOnce(T) -> crate::Result<()>>;

/// Write files and directories into a packet file
struct PacketWriterImpl<T>
where
    T: std::io::Write,
{
    tarball: tar::Builder<T>,
    cleanup_function: CleanUpFunction<T>,
}

impl<T> PacketWriterImpl<T>
where
    T: std::io::Write,
{
    fn new(packet_writer: T) -> crate::Result<Self> {
        let mut tarball = tar::Builder::new(packet_writer);
        tarball.follow_symlinks(false);

        Ok(Self {
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
    fn add_path(
        &mut self,
        _packet_path: &Path,
        _on_disk_path: &std::path::Path,
    ) -> crate::Result<()> {
        todo!()
    }

    fn finish(self) -> crate::Result<()> {
        let inner = self.tarball.into_inner()?;
        (self.cleanup_function)(inner)
    }
}
