// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! Handle packing/unpacking a packets.

// ----------------------------------------------------------------------
// - PacketWriter:
// ----------------------------------------------------------------------

/// Different types of paths
enum PathLeaf {
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
        /// The uid of the `File`
        uid: u32,
        /// The gid of the `File`
        gid: u32,
    },
    /// A `Directory`
    Directory {
        /// The `Dir` name with extension, etc. but without the base directory part
        name: std::ffi::OsString,
        /// The permissions on the `Dir`
        mode: u32,
        /// The uid of the `Dir`
        uid: u32,
        /// The gid of the `Dir`
        gid: u32,
    },
}

impl PathLeaf {
    fn leaf_name(&self) -> std::ffi::OsString {
        match &self {
            PathLeaf::File {
                name,
                mode: _,
                uid: _,
                gid: _,
                size: _,
            }
            | PathLeaf::Link {
                name,
                target: _,
                uid: _,
                gid: _,
            }
            | PathLeaf::Directory {
                name,
                mode: _,
                uid: _,
                gid: _,
            } => name.clone(),
        }
    }

    const fn mode(&self) -> u32 {
        match &self {
            PathLeaf::File {
                name: _,
                mode: m,
                uid: _,
                gid: _,
                size: _,
            }
            | PathLeaf::Directory {
                name: _,
                mode: m,
                uid: _,
                gid: _,
            } => *m,
            _ => 0o777,
        }
    }

    const fn uid(&self) -> u32 {
        match &self {
            PathLeaf::File {
                name: _,
                mode: _,
                uid: u,
                gid: _,
                size: _,
            }
            | PathLeaf::Link {
                name: _,
                target: _,
                uid: u,
                gid: _,
            }
            | PathLeaf::Directory {
                name: _,
                mode: _,
                uid: u,
                gid: _,
            } => *u,
        }
    }

    const fn gid(&self) -> u32 {
        match &self {
            PathLeaf::File {
                name: _,
                mode: _,
                uid: _,
                gid: g,
                size: _,
            }
            | PathLeaf::Link {
                name: _,
                target: _,
                uid: _,
                gid: g,
            }
            | PathLeaf::Directory {
                name: _,
                mode: _,
                uid: _,
                gid: g,
            } => *g,
        }
    }

    const fn size(&self) -> u64 {
        match &self {
            PathLeaf::File {
                name: _,
                mode: _,
                uid: _,
                gid: _,
                size: s,
            } => *s,
            _ => 0,
        }
    }

    fn link_target(&self) -> Option<std::path::PathBuf> {
        match &self {
            PathLeaf::Link {
                name: _,
                target: t,
                uid: _,
                gid: _,
            } => Some(t.clone()),
            _ => None,
        }
    }

    const fn leaf_type(&self) -> &'static str {
        match &self {
            PathLeaf::File {
                name: _,
                mode: _,
                uid: _,
                gid: _,
                size: _,
            } => "f",
            PathLeaf::Link {
                name: _,
                target: _,
                uid: _,
                gid: _,
            } => "l",
            PathLeaf::Directory {
                name: _,
                mode: _,
                uid: _,
                gid: _,
            } => "d",
        }
    }

    const fn is_dir(&self) -> bool {
        matches!(
            &self,
            PathLeaf::Directory {
                name: _,
                mode: _,
                uid: _,
                gid: _,
            }
        )
    }

    const fn is_link(&self) -> bool {
        matches!(
            &self,
            PathLeaf::Link {
                name: _,
                target: _,
                uid: _,
                gid: _,
            }
        )
    }

    const fn is_file(&self) -> bool {
        matches!(
            &self,
            PathLeaf::File {
                name: _,
                mode: _,
                uid: _,
                gid: _,
                size: _,
            }
        )
    }
}

/// A full path
pub struct Path {
    /// The directories up to the leaf
    directory: std::path::PathBuf,
    /// The leaf node on the directory
    leaf: PathLeaf,
}

impl Path {
    /// Create a new Path for a file.
    #[must_use]
    pub fn new_file(
        directory: &std::path::Path,
        name: &std::ffi::OsString,
        mode: u32,
        uid: u32,
        gid: u32,
        size: u64,
    ) -> Self {
        Self {
            directory: directory.to_path_buf(),
            leaf: PathLeaf::File {
                name: name.clone(),
                mode,
                uid,
                gid,
                size,
            },
        }
    }

    /// Create a new Path for a link.
    #[must_use]
    pub fn new_link(
        directory: &std::path::Path,
        name: &std::ffi::OsString,
        target: &std::path::Path,
        uid: u32,
        gid: u32,
    ) -> Self {
        Self {
            directory: directory.to_path_buf(),
            leaf: PathLeaf::Link {
                name: name.clone(),
                target: target.to_path_buf(),
                uid,
                gid,
            },
        }
    }

    /// Create a new Path for a file.
    #[must_use]
    pub fn new_directory(
        directory: &std::path::Path,
        name: &std::ffi::OsString,
        mode: u32,
        uid: u32,
        gid: u32,
    ) -> Self {
        Self {
            directory: directory.to_path_buf(),
            leaf: PathLeaf::Directory {
                name: name.clone(),
                mode,
                uid,
                gid,
            },
        }
    }

    /// Get the full path (abs or relative) stored in `Path`
    #[must_use]
    pub fn path(&self) -> std::path::PathBuf {
        let mut path = self.directory.clone();
        path.push(self.leaf.leaf_name());
        path
    }

    fn leaf_name(&self) -> std::ffi::OsString {
        self.leaf.leaf_name()
    }

    const fn leaf_type(&self) -> &'static str {
        self.leaf.leaf_type()
    }

    const fn mode(&self) -> u32 {
        self.leaf.mode()
    }

    const fn uid(&self) -> u32 {
        self.leaf.uid()
    }

    const fn gid(&self) -> u32 {
        self.leaf.gid()
    }

    const fn size(&self) -> u64 {
        self.leaf.size()
    }

    fn link_target(&self) -> Option<std::path::PathBuf> {
        self.leaf.link_target()
    }

    const fn is_dir(&self) -> bool {
        self.leaf.is_dir()
    }
    const fn is_link(&self) -> bool {
        self.leaf.is_link()
    }
    const fn is_file(&self) -> bool {
        self.leaf.is_file()
    }
}

impl std::fmt::Debug for Path {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let pp = self.path().to_string_lossy().to_string();
        let target = if let Some(t) = self.link_target() {
            format!(" -> {}", t.to_string_lossy())
        } else {
            String::new()
        };

        write!(
            fmt,
            "{}:{} [m:{:#o},u:{},g{}], {}bytes{}",
            self.leaf_type(),
            pp,
            self.mode(),
            self.uid(),
            self.gid(),
            self.size(),
            target
        )
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
    fn finish(self) -> crate::Result<std::path::PathBuf>;
}

/// The product of a `PacketWriterFactory`
pub type PacketWriterProduct = crate::Result<Box<dyn PacketWriter>>;
/// A type for factories of `PacketWriter`
pub type PacketWriterFactory =
    dyn Fn(&std::path::Path, &crate::Name) -> crate::Result<Box<dyn PacketWriter>>;

/// Create the full packet name from the base name.
fn full_packet_path(
    packet_path: &std::path::Path,
    packet_name: &crate::Name,
) -> std::path::PathBuf {
    packet_path.join(format!("{}.gng", packet_name))
}

/// Create a default packet writer
///
/// # Errors
/// Depends on the actual `PacketWriter` being created.
pub fn create_packet_writer(
    packet_path: &std::path::Path,
    packet_name: &crate::Name,
) -> PacketWriterProduct {
    // TODO: Make this configurable?
    let full_name = full_packet_path(packet_path, packet_name);

    let writer = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&full_name)?;
    let writer = zstd::Encoder::new(writer, 21)?;

    let mut tarball = PacketWriterImpl::new(writer);

    tarball.set_cleanup_function(Box::new(
        move |encoder: zstd::Encoder<_>| -> crate::Result<std::path::PathBuf> {
            match encoder.finish() {
                Err(e) => Err(e.into()),
                Ok(_) => Ok(full_name),
            }
        },
    ));

    Ok(Box::new(tarball))
}

// ----------------------------------------------------------------------
// - PacketWriterImpl:
// ----------------------------------------------------------------------

type CleanUpFunction<T> = Box<dyn FnOnce(T) -> crate::Result<std::path::PathBuf>>;

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
    fn new(packet_writer: T) -> Self {
        let mut tarball = tar::Builder::new(packet_writer);
        tarball.follow_symlinks(false);

        Self {
            tarball,
            cleanup_function: Box::new(|_| Ok(std::path::PathBuf::new())),
        }
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
        packet_path: &Path,
        on_disk_path: &std::path::Path,
    ) -> crate::Result<()> {
        println!(
            "        PACKAGED: {} as {}.",
            on_disk_path.to_string_lossy(),
            packet_path.path().to_string_lossy()
        );
        Ok(())
    }

    fn finish(self) -> crate::Result<std::path::PathBuf> {
        let inner = self.tarball.into_inner()?;
        (self.cleanup_function)(inner)
    }
}
