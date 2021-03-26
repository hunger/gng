// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use std::os::unix::fs::MetadataExt;

// - Helper:
// ----------------------------------------------------------------------

fn collect_contents(directory: &std::path::Path) -> gng_shared::Result<Vec<std::fs::DirEntry>> {
    let mut contents = std::fs::read_dir(directory)?
        .map(|i| i.map_err(|e| e.into()))
        .filter(|i| {
            if let Ok(d) = i {
                (d.file_name() != ".") && (d.file_name() != "..")
            } else {
                true
            }
        })
        .collect::<gng_shared::Result<Vec<std::fs::DirEntry>>>()?;
    contents.sort_by_key(std::fs::DirEntry::file_name);
    contents.reverse(); // So that we can pop() in turn later!

    Ok(contents)
}

fn dir_entry_for_path(path: &std::path::Path) -> gng_shared::Result<std::fs::DirEntry> {
    let search_name = path.file_name().ok_or(gng_shared::Error::Runtime {
        message: format!(
            "\"{}\" does not exist: No file name part was found.",
            path.to_string_lossy()
        ),
    })?;

    let parent = path.parent().ok_or(gng_shared::Error::Runtime {
        message: format!(
            "\"{}\" does not exist: Parent is not valid.",
            path.to_string_lossy()
        ),
    })?;
    collect_contents(parent)?
        .into_iter()
        .find(|d| d.file_name() == search_name)
        .ok_or(gng_shared::Error::Runtime {
            message: format!("\"{}\" not found.", path.to_string_lossy()),
        })
}

// ----------------------------------------------------------------------
// - DeterministicDirectoryIterator:
// ----------------------------------------------------------------------

pub struct DeterministicDirectoryIterator {
    stack: Vec<(Vec<std::fs::DirEntry>, std::path::PathBuf)>,
}

impl DeterministicDirectoryIterator {
    pub fn new(directory: &std::path::Path) -> gng_shared::Result<Self> {
        let base_dir_entry = dir_entry_for_path(directory)?;

        if base_dir_entry.file_type()?.is_dir() {
            Ok(Self {
                stack: vec![(vec![base_dir_entry], std::path::PathBuf::new())],
            })
        } else {
            Err(gng_shared::Error::Runtime {
                message: format!("\"{}\" is not a directory.", directory.to_string_lossy()),
            })
        }
    }

    fn at_end(&self) -> bool {
        self.stack.is_empty()
    }

    fn find_iterator_value(&mut self) -> crate::packager::PackagingIteration {
        let stack_frame = self.stack.last_mut().expect("Can not be empty!");
        let entry = stack_frame.0.pop().expect("Can not be empty!");
        let directory = stack_frame.1.clone();

        let name = entry.file_name();
        let file_type = entry.file_type()?;
        let meta = entry.path().symlink_metadata()?;
        let mode = meta.mode() & 0o7777_u32;
        let user_id = meta.uid();
        let group_id = meta.gid();
        let size = meta.size();

        if file_type.is_symlink() {
            let target = entry.path().read_link()?;
            Ok(crate::packager::PacketPath {
                on_disk: entry.path(),
                in_packet: gng_shared::package::Path::new_link(
                    &directory, &name, &target, user_id, group_id,
                ),
                mime_type: String::new(),
            })
        } else if file_type.is_file() {
            Ok(crate::packager::PacketPath {
                on_disk: entry.path(),
                in_packet: gng_shared::package::Path::new_file(
                    &directory, &name, mode, user_id, group_id, size,
                ),
                mime_type: String::new(),
            })
        } else if file_type.is_dir() {
            let contents = collect_contents(&entry.path())?;
            let (new_directory_path, new_directory_name) = if directory.as_os_str().is_empty() {
                (std::path::PathBuf::from("."), std::ffi::OsString::from("."))
            } else {
                (directory.join(&name), name)
            };

            self.stack.push((contents, new_directory_path));

            Ok(crate::packager::PacketPath {
                on_disk: entry.path(),
                in_packet: gng_shared::package::Path::new_directory(
                    &directory,
                    &new_directory_name,
                    mode,
                    user_id,
                    group_id,
                ),
                mime_type: String::new(),
            })
        } else {
            Err(gng_shared::Error::Runtime {
                message: format!(
                    "Unsupported file type {:?} found in {}.",
                    &file_type,
                    &entry.path().to_string_lossy()
                ),
            })
        }
    }

    fn clean_up(&mut self) {
        loop {
            if let Some(v) = self.stack.last() {
                if v.0.is_empty() {
                    // The top element is empty: pop it and its corresponding directory!
                    self.stack.pop();
                    continue;
                }
            }
            break;
        }
    }
}

impl Iterator for DeterministicDirectoryIterator {
    type Item = crate::packager::PackagingIteration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at_end() {
            return None;
        }

        let result = self.find_iterator_value();
        self.clean_up();

        Some(result)
    }
}
