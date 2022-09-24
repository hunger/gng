// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2021 Tobias Hunger <tobias.hunger@gmail.com>

use super::path::Path;

use std::os::unix::fs::MetadataExt;

// - Helper:
// ----------------------------------------------------------------------

fn collect_contents(directory: &std::path::Path) -> eyre::Result<Vec<std::fs::DirEntry>> {
    let mut contents = std::fs::read_dir(directory)?
        .map(|i| i.map_err(Into::into))
        .filter(|i| {
            i.as_ref()
                .map_or(true, |d| (d.file_name() != ".") && (d.file_name() != ".."))
        })
        .collect::<eyre::Result<Vec<std::fs::DirEntry>>>()?;
    contents.sort_by_key(std::fs::DirEntry::file_name);
    contents.reverse(); // So that we can pop() in turn later!

    Ok(contents)
}

fn dir_entry_for_path(path: &std::path::Path) -> eyre::Result<std::fs::DirEntry> {
    let search_name = path.file_name().ok_or_else(|| {
        eyre::eyre!(
            "\"{}\" does not exist: No file name part was found.",
            path.to_string_lossy()
        )
    })?;

    let parent = path.parent().ok_or_else(|| {
        eyre::eyre!(
            "\"{}\" does not exist: Parent is not valid.",
            path.to_string_lossy()
        )
    })?;
    collect_contents(parent)?
        .into_iter()
        .find(|d| d.file_name() == search_name)
        .ok_or_else(|| eyre::eyre!("\"{}\" not found.", path.to_string_lossy()))
}

fn populate_directory_stack(
    directory: &std::path::Path,
    name: &std::path::Path,
) -> eyre::Result<(Vec<std::fs::DirEntry>, std::path::PathBuf)> {
    let contents = collect_contents(directory)?;
    Ok((contents, name.to_owned()))
}

// ----------------------------------------------------------------------
// - Helper Types:
// ----------------------------------------------------------------------

pub type PackagingIteration = eyre::Result<Path>;

// ----------------------------------------------------------------------
// - DeterministicDirectoryIterator:
// ----------------------------------------------------------------------

pub struct DeterministicDirectoryIterator {
    stack: Vec<(Vec<std::fs::DirEntry>, std::path::PathBuf)>,
}

impl DeterministicDirectoryIterator {
    /// Constructor
    pub fn new(directory: &std::path::Path) -> eyre::Result<Self> {
        let base_dir_entry = dir_entry_for_path(directory)?;

        if base_dir_entry.file_type()?.is_dir() {
            let stack_element = populate_directory_stack(directory, &std::path::PathBuf::new())?;
            Ok(Self {
                stack: vec![stack_element],
            })
        } else {
            Err(eyre::eyre!(
                "\"{}\" is not a directory.",
                directory.to_string_lossy()
            ))
        }
    }

    fn at_end(&self) -> bool {
        self.stack.is_empty()
    }

    fn find_iterator_value(&mut self) -> PackagingIteration {
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

            Ok(Path::new_link(
                &directory.join(name),
                &target,
                user_id,
                group_id,
            ))
        } else if file_type.is_file() {
            Ok(Path::new_file_from_disk(
                &entry.path(),
                &directory.join(name),
                mode,
                user_id,
                group_id,
                size,
            ))
        } else if file_type.is_dir() {
            let new_directory = if directory.as_os_str().is_empty() {
                std::path::PathBuf::from(&name)
            } else {
                directory.join(&name)
            };

            self.stack
                .push(populate_directory_stack(&entry.path(), &new_directory)?);

            Ok(Path::new_directory(
                &directory.join(name),
                mode,
                user_id,
                group_id,
            ))
        } else {
            Err(eyre::eyre!(
                "Unsupported file type {:?} found in {}.",
                &file_type,
                &entry.path().to_string_lossy()
            ))
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
    type Item = PackagingIteration;

    fn next(&mut self) -> Option<Self::Item> {
        self.clean_up();
        if self.at_end() {
            return None;
        }

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

    fn touch(path: &std::path::Path) {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)
            .unwrap();
    }

    fn link(origin: &std::path::Path, target: &std::path::Path) {
        std::os::unix::fs::symlink(target, origin).unwrap();
    }

    #[test]
    fn deterministic_iterator_empty_top_dir() {
        let tmp = tempfile::Builder::new()
            .prefix("dir-it-et-")
            .rand_bytes(8)
            .tempdir()
            .expect("Failed to create temporary directory.");
        let mut it = DeterministicDirectoryIterator::new(tmp.path()).unwrap();
        assert!(it.next().is_none());
    }

    #[test]
    fn deterministic_iterator_sort_order() {
        let tmp = tempfile::Builder::new()
            .prefix("dir-it-so-")
            .rand_bytes(8)
            .tempdir()
            .expect("Failed to create temporary directory.");
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
            Path::new_file_from_disk(
                &tmp.path().join("aaa_foo.txt"),
                &std::path::PathBuf::from("aaa_foo.txt"),
                0o644,
                tmp_meta.uid(),
                tmp_meta.gid(),
                0,
            )
        );
        assert_eq!(
            it.next().unwrap().unwrap(),
            Path::new_directory(
                &std::path::PathBuf::from("bar_dir"),
                0o755,
                tmp_meta.uid(),
                tmp_meta.gid(),
            )
        );
        assert_eq!(
            it.next().unwrap().unwrap(),
            Path::new_file_from_disk(
                &tmp.path().join("bar_dir/aaa_bar.txt"),
                &std::path::PathBuf::from("bar_dir/aaa_bar.txt"),
                0o644,
                tmp_meta.uid(),
                tmp_meta.gid(),
                0,
            )
        );
        assert_eq!(
            it.next().unwrap().unwrap(),
            Path::new_directory(
                &std::path::PathBuf::from("empty_dir"),
                0o755,
                tmp_meta.uid(),
                tmp_meta.gid(),
            )
        );
        assert_eq!(
            it.next().unwrap().unwrap(),
            Path::new_link(
                &std::path::PathBuf::from("zzz_link"),
                &std::path::PathBuf::from("bar_dir"),
                tmp_meta.uid(),
                tmp_meta.gid(),
            )
        );
        assert!(it.next().is_none());
    }
}
