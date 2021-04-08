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

fn populate_directory_stack(
    directory: &std::path::Path,
    name: &std::path::Path,
) -> gng_shared::Result<(Vec<std::fs::DirEntry>, std::path::PathBuf)> {
    let contents = collect_contents(directory)?;
    Ok((contents, name.to_owned()))
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
            let stack_element = populate_directory_stack(directory, &std::path::PathBuf::new())?;
            Ok(Self {
                stack: vec![stack_element],
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
        let entry = stack_frame.0.pop().expect("Can not be empty");
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
                in_packet: gng_shared::packet::Path::new_link(
                    &directory, &name, &target, user_id, group_id,
                ),
                mime_type: String::new(),
            })
        } else if file_type.is_file() {
            Ok(crate::packager::PacketPath {
                in_packet: gng_shared::packet::Path::new_file_from_disk(
                    &entry.path(),
                    &directory,
                    &name,
                    mode,
                    user_id,
                    group_id,
                    size,
                )?,
                mime_type: String::new(),
            })
        } else if file_type.is_dir() {
            let new_directory = if directory.as_os_str().is_empty() {
                std::path::PathBuf::from(&name)
            } else {
                directory.join(&name)
            };

            self.stack
                .push(populate_directory_stack(&entry.path(), &new_directory)?);

            self.print_state("New directory");

            Ok(crate::packager::PacketPath {
                in_packet: gng_shared::packet::Path::new_directory(
                    &directory, &name, mode, user_id, group_id,
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

    fn print_state(&self, message: &str) {
        println!("{}", message);
        for s in &self.stack {
            println!(
                "    {} => with {} elements.",
                s.1.to_string_lossy(),
                s.0.len()
            );
        }
        println!("-----------------------");
    }
}

impl Iterator for DeterministicDirectoryIterator {
    type Item = crate::packager::PackagingIteration;

    fn next(&mut self) -> Option<Self::Item> {
        self.print_state("Before Cleanup");
        self.clean_up();
        if self.at_end() {
            return None;
        }

        self.print_state("When finding");
        let result = self.find_iterator_value();

        Some(result)
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use temp_dir::TempDir;

    fn touch(path: &std::path::Path) {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)
            .unwrap();
    }

    fn link(origin: &std::path::Path, target: &std::path::Path) {
        std::os::unix::fs::symlink(target, origin).unwrap()
    }

    #[test]
    fn test_deterministic_iterator_empty_top_dir() {
        let tmp = TempDir::new().unwrap();
        let mut it = DeterministicDirectoryIterator::new(tmp.path()).unwrap();
        assert!(it.next().is_none());
    }

    #[test]
    fn test_deterministic_iterator_sort_order() {
        let tmp = TempDir::new().unwrap();
        let tmp_meta = std::fs::metadata(tmp.path()).unwrap();

        std::fs::create_dir(tmp.path().join("bar_dir")).unwrap();
        touch(&tmp.path().join("aaa_foo.txt"));
        link(
            &tmp.path().join("zzz_link"),
            &std::path::PathBuf::from("bar_dir"),
        );
        touch(&tmp.path().join("bar_dir/aaa_bar.txt"));
        std::fs::create_dir(tmp.path().join("empty_dir")).unwrap();

        let mut it = DeterministicDirectoryIterator::new(tmp.path()).unwrap();
        assert_eq!(
            it.next().unwrap().unwrap(),
            crate::packager::PacketPath {
                in_packet: gng_shared::packet::Path::new_file_from_disk(
                    &tmp.path().join("aaa_foo.txt"),
                    &std::path::PathBuf::new(),
                    &std::ffi::OsString::from("aaa_foo.txt"),
                    0o644,
                    tmp_meta.uid(),
                    tmp_meta.gid(),
                    0,
                )
                .unwrap(),
                mime_type: String::new(),
            }
        );
        assert_eq!(
            it.next().unwrap().unwrap(),
            crate::packager::PacketPath {
                in_packet: gng_shared::packet::Path::new_directory(
                    &std::path::PathBuf::new(),
                    &std::ffi::OsString::from("bar_dir"),
                    0o755,
                    tmp_meta.uid(),
                    tmp_meta.gid(),
                ),
                mime_type: String::new(),
            }
        );
        assert_eq!(
            it.next().unwrap().unwrap(),
            crate::packager::PacketPath {
                in_packet: gng_shared::packet::Path::new_file_from_disk(
                    &tmp.path().join("bar_dir/aaa_bar.txt"),
                    &std::path::PathBuf::from("bar_dir"),
                    &std::ffi::OsString::from("aaa_bar.txt"),
                    0o644,
                    tmp_meta.uid(),
                    tmp_meta.gid(),
                    0,
                )
                .unwrap(),
                mime_type: String::new(),
            }
        );
        assert_eq!(
            it.next().unwrap().unwrap(),
            crate::packager::PacketPath {
                in_packet: gng_shared::packet::Path::new_directory(
                    &std::path::PathBuf::new(),
                    &std::ffi::OsString::from("empty_dir"),
                    0o755,
                    tmp_meta.uid(),
                    tmp_meta.gid(),
                ),
                mime_type: String::new(),
            }
        );
        assert_eq!(
            it.next().unwrap().unwrap(),
            crate::packager::PacketPath {
                in_packet: gng_shared::packet::Path::new_link(
                    &std::path::PathBuf::new(),
                    &std::ffi::OsString::from("zzz_link"),
                    &std::path::PathBuf::from("bar_dir"),
                    tmp_meta.uid(),
                    tmp_meta.gid(),
                ),
                mime_type: String::new(),
            }
        );
        assert!(it.next().is_none())
    }
}
