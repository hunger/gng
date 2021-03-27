// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

//! Handle packing/unpacking a packets.

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn create_header(packet_path: &Path) -> crate::Result<tar::Header> {
    let mut header = tar::Header::new_gnu();

    {
        let gnu = header
            .as_gnu_mut()
            .expect("Created this as GNU, so should work!");

        gnu.set_atime(0);
        gnu.set_ctime(0);
    }

    header.set_mtime(0);
    header.set_device_major(0)?;
    header.set_device_minor(0)?;
    header.set_size(packet_path.size());
    header.set_mode(packet_path.mode());
    header.set_uid(packet_path.user_id() as u64);
    header.set_gid(packet_path.group_id() as u64);

    if let Some(t) = packet_path.link_target() {
        header.set_link_name(&t)?;
    }

    if packet_path.is_dir() {
        header.set_entry_type(tar::EntryType::Directory)
    } else if packet_path.is_file() {
        header.set_entry_type(tar::EntryType::Regular)
    } else if packet_path.is_link() {
        header.set_entry_type(tar::EntryType::Symlink);
    } else {
        return Err(crate::Error::Runtime {
            message: "Unexpected entry in filesystem. Can not package.".to_string(),
        });
    }

    Ok(header)
}

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
        user_id: u32,
        /// The gid of the `File`
        group_id: u32,
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
        user_id: u32,
        /// The gid of the `File`
        group_id: u32,
    },
    /// A `Directory`
    Directory {
        /// The `Dir` name with extension, etc. but without the base directory part
        name: std::ffi::OsString,
        /// The permissions on the `Dir`
        mode: u32,
        /// The uid of the `Dir`
        user_id: u32,
        /// The gid of the `Dir`
        group_id: u32,
    },
}

impl PathLeaf {
    fn leaf_name(&self) -> std::ffi::OsString {
        match &self {
            PathLeaf::File {
                name,
                mode: _,
                user_id: _,
                group_id: _,
                size: _,
            }
            | PathLeaf::Link {
                name,
                target: _,
                user_id: _,
                group_id: _,
            }
            | PathLeaf::Directory {
                name,
                mode: _,
                user_id: _,
                group_id: _,
            } => name.clone(),
        }
    }

    const fn mode(&self) -> u32 {
        match &self {
            PathLeaf::File {
                name: _,
                mode: m,
                user_id: _,
                group_id: _,
                size: _,
            }
            | PathLeaf::Directory {
                name: _,
                mode: m,
                user_id: _,
                group_id: _,
            } => *m,
            _ => 0o777,
        }
    }

    const fn user_id(&self) -> u32 {
        match &self {
            PathLeaf::File {
                name: _,
                mode: _,
                user_id: u,
                group_id: _,
                size: _,
            }
            | PathLeaf::Link {
                name: _,
                target: _,
                user_id: u,
                group_id: _,
            }
            | PathLeaf::Directory {
                name: _,
                mode: _,
                user_id: u,
                group_id: _,
            } => *u,
        }
    }

    const fn group_id(&self) -> u32 {
        match &self {
            PathLeaf::File {
                name: _,
                mode: _,
                user_id: _,
                group_id: g,
                size: _,
            }
            | PathLeaf::Link {
                name: _,
                target: _,
                user_id: _,
                group_id: g,
            }
            | PathLeaf::Directory {
                name: _,
                mode: _,
                user_id: _,
                group_id: g,
            } => *g,
        }
    }

    const fn size(&self) -> u64 {
        match &self {
            PathLeaf::File {
                name: _,
                mode: _,
                user_id: _,
                group_id: _,
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
                user_id: _,
                group_id: _,
            } => Some(t.clone()),
            _ => None,
        }
    }

    const fn leaf_type(&self) -> &'static str {
        match &self {
            PathLeaf::File {
                name: _,
                mode: _,
                user_id: _,
                group_id: _,
                size: _,
            } => "f",
            PathLeaf::Link {
                name: _,
                target: _,
                user_id: _,
                group_id: _,
            } => "l",
            PathLeaf::Directory {
                name: _,
                mode: _,
                user_id: _,
                group_id: _,
            } => "d",
        }
    }

    const fn is_dir(&self) -> bool {
        matches!(
            &self,
            PathLeaf::Directory {
                name: _,
                mode: _,
                user_id: _,
                group_id: _,
            }
        )
    }

    const fn is_link(&self) -> bool {
        matches!(
            &self,
            PathLeaf::Link {
                name: _,
                target: _,
                user_id: _,
                group_id: _,
            }
        )
    }

    const fn is_file(&self) -> bool {
        matches!(
            &self,
            PathLeaf::File {
                name: _,
                mode: _,
                user_id: _,
                group_id: _,
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
        user_id: u32,
        group_id: u32,
        size: u64,
    ) -> Self {
        Self {
            directory: directory.to_path_buf(),
            leaf: PathLeaf::File {
                name: name.clone(),
                mode,
                user_id,
                group_id,
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
        user_id: u32,
        group_id: u32,
    ) -> Self {
        Self {
            directory: directory.to_path_buf(),
            leaf: PathLeaf::Link {
                name: name.clone(),
                target: target.to_path_buf(),
                user_id,
                group_id,
            },
        }
    }

    /// Create a new Path for a file.
    #[must_use]
    pub fn new_directory(
        directory: &std::path::Path,
        name: &std::ffi::OsString,
        mode: u32,
        user_id: u32,
        group_id: u32,
    ) -> Self {
        Self {
            directory: directory.to_path_buf(),
            leaf: PathLeaf::Directory {
                name: name.clone(),
                mode,
                user_id,
                group_id,
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

    /// The last part of the `Path`
    #[must_use]
    pub fn leaf_name(&self) -> std::ffi::OsString {
        self.leaf.leaf_name()
    }

    /// A `&'static str` describing the type of `Path`
    #[must_use]
    pub const fn leaf_type(&self) -> &'static str {
        self.leaf.leaf_type()
    }

    /// The `mode` of the leaf
    #[must_use]
    pub const fn mode(&self) -> u32 {
        self.leaf.mode()
    }

    /// The `user_id` of the leaf
    #[must_use]
    pub const fn user_id(&self) -> u32 {
        self.leaf.user_id()
    }

    /// The `group_id` of the leaf
    #[must_use]
    pub const fn group_id(&self) -> u32 {
        self.leaf.group_id()
    }

    /// The `size` of the leaf. Will be 0 for anything but normal files.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.leaf.size()
    }

    /// The target this leaf is pointing to (if it is a symlink).
    #[must_use]
    pub fn link_target(&self) -> Option<std::path::PathBuf> {
        self.leaf.link_target()
    }

    /// Is the leaf a directory?
    #[must_use]
    pub const fn is_dir(&self) -> bool {
        self.leaf.is_dir()
    }

    /// Is the leaf a link?
    #[must_use]
    pub const fn is_link(&self) -> bool {
        self.leaf.is_link()
    }

    /// Is the leaf a file?
    #[must_use]
    pub const fn is_file(&self) -> bool {
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
            self.user_id(),
            self.group_id(),
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

    /// Add a directory into the packet.
    ///
    /// # Errors
    /// Returns mostly `Error::Io`
    fn add_data(&mut self, packet_path: &Path, data: &[u8]) -> crate::Result<()>;

    /// finish writing the packet.
    ///
    /// # Errors
    /// Depends on the actual Writer being used.
    fn finish(&mut self) -> crate::Result<std::path::PathBuf>;
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
    tarball: Option<tar::Builder<T>>,
    cleanup_function: Option<CleanUpFunction<T>>,
}

impl<T> PacketWriterImpl<T>
where
    T: std::io::Write,
{
    fn new(packet_writer: T) -> Self {
        let mut tarball = tar::Builder::new(packet_writer);
        tarball.follow_symlinks(false);

        Self {
            tarball: Some(tarball),
            cleanup_function: None,
        }
    }

    fn set_cleanup_function(&mut self, function: CleanUpFunction<T>) {
        self.cleanup_function = Some(function);
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
        let tb = self.tarball.as_mut().ok_or(crate::Error::Runtime {
            message: "Writer has finished already.".to_string(),
        })?;

        let mut header = create_header(packet_path)?;

        let path = packet_path.path();

        if packet_path.is_file() {
            let file = std::fs::OpenOptions::new().read(true).open(&on_disk_path)?;
            tb.append_data(&mut header, &path, std::io::BufReader::new(file))?;
        } else {
            tb.append_data(&mut header, &path, std::io::empty())?;
        };

        Ok(())
    }

    fn add_data(&mut self, packet_path: &Path, data: &[u8]) -> crate::Result<()> {
        if packet_path.is_file() {
            let tb = self.tarball.as_mut().ok_or(crate::Error::Runtime {
                message: "Writer has finished already.".to_string(),
            })?;

            let mut header = create_header(packet_path)?;
            tb.append_data(&mut header, packet_path.path(), data)
                .map_err(|e| e.into())
        } else {
            Err(crate::Error::Runtime {
                message: "Need a file path to store a buffer in.".to_string(),
            })
        }
    }

    fn finish(&mut self) -> crate::Result<std::path::PathBuf> {
        let tb = self.tarball.take().ok_or(crate::Error::Runtime {
            message: "Writer has finished already.".to_string(),
        })?;
        let inner = tb.into_inner()?;
        (self
            .cleanup_function
            .take()
            .unwrap_or_else(|| Box::new(|_| Ok(std::path::PathBuf::new()))))(inner)
    }
}
