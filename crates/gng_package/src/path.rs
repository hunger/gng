// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

// spell-checker: ignore filemagic

// ----------------------------------------------------------------------
// - Helper:
// ----------------------------------------------------------------------

fn create_cookie() -> eyre::Result<filemagic::Magic> {
    let cookie = filemagic::Magic::open(filemagic::flags::Flags::default()).map_err(|e| {
        gng_core::Error::Runtime {
            message: format!("File type detection setup failed: {}", e),
        }
    })?;
    cookie
        .load::<String>(&[])
        .map_err(|e| gng_core::Error::Runtime {
            message: format!("File type detection database failed to load: {}", e),
        })?;

    Ok(cookie)
}

// ----------------------------------------------------------------------
// - Contents:
// ----------------------------------------------------------------------

/// The file contents
#[derive(Clone, Eq, PartialEq)]
pub enum FileContents {
    /// `FileContents` is taken from a buffer
    Buffer(Vec<u8>),
    /// `FileContents` is read from a file on disk
    OnDisk(std::path::PathBuf),
}

impl FileContents {
    fn magic(&self) -> eyre::Result<String> {
        thread_local! {
            static COOKIE: std::cell::RefCell<Option<filemagic::Magic>> = std::cell::RefCell::new(None);
        }
        COOKIE.with(|c| {
            if c.borrow().is_none() {
                *c.borrow_mut() = Some(create_cookie()?);
            };

            let c = c.borrow();
            let c = c
                .as_ref()
                .expect("COOKIE was set before, so this should not be None");

            Ok(match self {
                Self::OnDisk(p) => c.file(p).unwrap_or_default(),
                Self::Buffer(b) => c.buffer(b).unwrap_or_default(),
            })
        })
    }
}

// ----------------------------------------------------------------------
// - PathLeaf:
// ----------------------------------------------------------------------

/// Different types of paths
#[derive(Clone, PartialEq)]
enum PathLeaf {
    /// A `File`
    File {
        /// Source to the `File` on disk.
        contents: FileContents,
        /// The size of the `File` in bytes
        size: u64,
    },
    /// A `Link`
    Link {
        /// The `Link` target (complete with base directories, etc.!)
        target: std::path::PathBuf,
    },
    /// A `Directory`
    Directory {},
}

impl PathLeaf {
    const fn size(&self) -> u64 {
        match &self {
            Self::File {
                size: s,
                contents: _,
            } => *s,
            _ => 0,
        }
    }

    fn link_target(&self) -> Option<std::path::PathBuf> {
        match &self {
            Self::Link { target: t } => Some(t.clone()),
            _ => None,
        }
    }

    const fn leaf_type(&self) -> &'static str {
        match &self {
            Self::File {
                size: _,
                contents: _,
            } => "f",
            Self::Link { target: _ } => "l",
            Self::Directory {} => "d",
        }
    }

    const fn is_dir(&self) -> bool {
        matches!(&self, Self::Directory {})
    }

    const fn is_link(&self) -> bool {
        matches!(&self, Self::Link { target: _ })
    }

    const fn is_file(&self) -> bool {
        matches!(
            &self,
            Self::File {
                size: _,
                contents: _
            }
        )
    }

    fn magic(&self) -> eyre::Result<String> {
        match self {
            Self::File { contents, size: _ } => contents.magic(),
            Self::Link { target: _ } => Ok("link".to_string()),
            Self::Directory {} => Ok("directory".to_string()),
        }
    }

    const fn file_contents(&self) -> Option<&FileContents> {
        match self {
            Self::File {
                contents: c,
                size: _,
            } => Some(c),
            _ => None,
        }
    }
}

impl std::fmt::Debug for PathLeaf {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match &self {
            Self::File { contents, size } => match contents {
                FileContents::OnDisk(p) => {
                    write!(fmt, "FILE ({}bytes) from \"{}\"", size, p.to_string_lossy())
                }
                FileContents::Buffer(_) => write!(fmt, "FILE ({}bytes) from <BUFFER>", size),
            },
            Self::Link { target } => write!(fmt, "LINK to \"{}\"", target.to_string_lossy()),
            Self::Directory {} => write!(fmt, "DIR"),
        }
    }
}

// ----------------------------------------------------------------------
// - Path:
// ----------------------------------------------------------------------

/// A full path
#[derive(Clone, PartialEq)]
pub struct Path {
    /// The full path
    full_path: std::path::PathBuf,
    /// The permissions on the `File`
    mode: u32,
    /// The uid of the `File`
    user_id: u32,
    /// The gid of the `File`
    group_id: u32,
    /// The leaf node on the directory
    leaf_type: PathLeaf,
    /// The ***magic***
    magic: Option<String>,
}

impl Path {
    /// Create a new Path for a file.
    pub fn new_file_from_disk(
        on_disk: &std::path::Path,
        full_path: &std::path::Path,
        mode: u32,
        user_id: u32,
        group_id: u32,
        size: u64,
    ) -> Self {
        Self {
            full_path: full_path.to_path_buf(),
            mode,
            user_id,
            group_id,
            leaf_type: PathLeaf::File {
                size,
                contents: FileContents::OnDisk(on_disk.to_path_buf()),
            },
            magic: None,
        }
    }

    /// Create a new Path for a file.
    #[must_use]
    pub fn new_file_from_buffer(
        buffer: Vec<u8>,
        full_path: &std::path::Path,
        mode: u32,
        user_id: u32,
        group_id: u32,
    ) -> Self {
        let size = buffer.len() as u64;
        Self {
            full_path: full_path.to_path_buf(),
            mode,
            user_id,
            group_id,
            leaf_type: PathLeaf::File {
                size,
                contents: FileContents::Buffer(buffer),
            },
            magic: None,
        }
    }

    /// Create a new Path for a link.
    #[must_use]
    pub fn new_link(
        full_path: &std::path::Path,
        target: &std::path::Path,
        user_id: u32,
        group_id: u32,
    ) -> Self {
        Self {
            full_path: full_path.to_path_buf(),
            user_id,
            group_id,
            mode: 0x777,
            leaf_type: PathLeaf::Link {
                target: target.to_path_buf(),
            },
            magic: None,
        }
    }

    /// Create a new Path for a file.
    #[must_use]
    pub fn new_directory(
        full_path: &std::path::Path,
        mode: u32,
        user_id: u32,
        group_id: u32,
    ) -> Self {
        Self {
            full_path: full_path.to_path_buf(),
            mode,
            user_id,
            group_id,
            leaf_type: PathLeaf::Directory {},
            magic: None,
        }
    }

    /// The last part of the `Path`
    #[must_use]
    pub fn leaf_name(&self) -> &std::ffi::OsStr {
        self.full_path
            .file_name()
            .expect("Path was invalid and had no leaf_name")
    }

    /// The last part of the `Path`
    #[must_use]
    pub fn directory_path(&self) -> &std::path::Path {
        lazy_static::lazy_static! {
            static ref DEFAULT_PATH: std::path::PathBuf = std::path::PathBuf::from(".");

        }
        self.full_path.parent().unwrap_or(&DEFAULT_PATH)
    }

    /// A `&'static str` describing the type of `Path`
    #[must_use]
    pub const fn leaf_type(&self) -> &'static str {
        self.leaf_type.leaf_type()
    }

    /// The `mode` of the leaf
    #[must_use]
    pub const fn mode(&self) -> u32 {
        self.mode
    }

    /// The `user_id` of the leaf
    #[must_use]
    pub const fn user_id(&self) -> u32 {
        self.user_id
    }

    /// The `group_id` of the leaf
    #[must_use]
    pub const fn group_id(&self) -> u32 {
        self.group_id
    }

    /// The `size` of the leaf. Will be 0 for anything but normal files.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.leaf_type.size()
    }

    /// The target this leaf is pointing to (if it is a symlink).
    #[must_use]
    pub fn link_target(&self) -> Option<std::path::PathBuf> {
        self.leaf_type.link_target()
    }

    /// Is the leaf a directory?
    #[must_use]
    pub const fn is_dir(&self) -> bool {
        self.leaf_type.is_dir()
    }

    /// Is the leaf a link?
    #[must_use]
    pub const fn is_link(&self) -> bool {
        self.leaf_type.is_link()
    }

    /// Is the leaf a file?
    #[must_use]
    pub const fn is_file(&self) -> bool {
        self.leaf_type.is_file()
    }

    /// Get the file data source
    #[must_use]
    pub const fn file_contents(&self) -> Option<&FileContents> {
        self.leaf_type.file_contents()
    }

    /// Get the ***magic***
    pub fn magic(&mut self) -> eyre::Result<String> {
        if self.magic.is_none() {
            self.magic = Some(self.leaf_type.magic()?);
        }
        Ok(self.magic.as_ref().expect("Magic was just set.").clone())
    }

    /// Turn the `Path` into a String
    pub fn as_path(&self) -> &std::path::Path {
        &self.full_path
    }
}

impl std::fmt::Debug for Path {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            fmt,
            "{} [m:{:#o},u:{},g{}], {:?}",
            self.as_path().to_string_lossy(),
            self.mode(),
            self.user_id(),
            self.group_id(),
            self.leaf_type,
        )
    }
}
