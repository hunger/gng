// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! A object representing a `Repository`

// ----------------------------------------------------------------------
// - Repository:
// ----------------------------------------------------------------------

/// A `Repository` of gng `Packet`s and related information
pub trait Repository {
    /// Get the Schema version
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn version(&self) -> crate::Result<u32>;

    /// Get the repository UUID
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn uuid(&self) -> crate::Result<Vec<u8>>;

    /// Print out the metadata stored about this repository.
    ///
    /// # Errors
    /// And of the crate's `Error`s can be returned from here.
    fn dump_metadata(&mut self) -> crate::Result<()>;
}

// ----------------------------------------------------------------------
// - RepositoryImpl:
// ----------------------------------------------------------------------

pub(crate) struct RepositoryImpl {
    db: sled::Db,
}

impl RepositoryImpl {
    const MAGIC: &'static str = "\"1e925d91-3294-4676-add6-917376d89d58\"";
    const MAGIC_KEY: &'static str = "magic";

    const VERSION: u32 = 1;
    const VERSION_KEY: &'static str = "schema_version";

    const REPOSITORY_UUID_KEY: &'static str = "repository_uuid";

    pub(crate) fn new(path: &std::path::Path) -> crate::Result<Self> {
        let config = sled::Config::default().path(path.to_owned());

        let mut result = Self { db: config.open()? };

        match result.db.get(Self::MAGIC_KEY)? {
            Some(v) if v == Self::MAGIC => result.db_validate()?,
            Some(_) => return Err(crate::Error::WrongMagic),
            None => result.db_setup()?,
        };

        Ok(result)
    }

    fn db_setup(&mut self) -> crate::Result<()> {
        self.db.insert(
            Self::REPOSITORY_UUID_KEY,
            &serde_json::to_vec(&uuid::Uuid::new_v4()).expect("UUID should be valid!")[..],
        )?;
        self.db.insert(
            Self::VERSION_KEY,
            &serde_json::to_vec(&Self::VERSION).map_err(|_| crate::Error::WrongSchema)?[..],
        )?;

        let repositories_tree = self.db.open_tree("repositories")?;
        repositories_tree.insert(&self.uuid()?[..], b"local")?;

        let _packages_tree = self.db.open_tree("packages")?;
        let _sources_tree = self.db.open_tree("sources")?;
        let _files_tree = self.db.open_tree("files")?;

        self.db.insert(Self::MAGIC_KEY, Self::MAGIC)?;

        self.db.flush()?;
        Ok(())
    }

    fn db_validate(&mut self) -> crate::Result<()> {
        match self.version()? {
            v if v < Self::VERSION => Err(crate::Error::WrongSchema),
            v if v == Self::VERSION => Ok(()),
            _ => Err(crate::Error::WrongSchema),
        }
    }

    fn repository_uuid(&self) -> crate::Result<uuid::Uuid> {
        let uuid = self
            .db
            .get(Self::REPOSITORY_UUID_KEY)?
            .expect("Repository UUID must be set!");
        let uuid: uuid::Uuid =
            serde_json::from_slice(&uuid[..]).map_err(|_| crate::Error::WrongSchema)?;
        Ok(uuid)
    }
}

impl Repository for RepositoryImpl {
    fn version(&self) -> crate::Result<u32> {
        let version = self
            .db
            .get(Self::VERSION_KEY)?
            .expect("Version must be set!");
        serde_json::from_slice(&version[..]).map_err(|_| crate::Error::WrongSchema)
    }

    fn uuid(&self) -> crate::Result<Vec<u8>> {
        let uuid = self.repository_uuid()?;
        Ok(uuid.as_bytes().to_vec())
    }

    fn dump_metadata(&mut self) -> crate::Result<()> {
        println!("Metadata:");
        for data in self.db.iter() {
            match data {
                Err(e) => println!("Error: {}", e),
                Ok((k, v)) => {
                    let key = std::string::String::from_utf8((&k[..]).to_vec())
                        .unwrap_or(format!("{:?}", k));
                    let value = if v.is_ascii() {
                        std::string::String::from_utf8((&v[..]).to_vec())
                            .expect("I though this is ASCII?!")
                    } else {
                        format!("{:?}", v)
                    };
                    println!("    \"{}\" => {}.", key, value,)
                }
            }
        }

        println!("\nKnown trees:");
        for n in &self.db.tree_names() {
            let n = std::string::String::from_utf8((&n[..]).to_vec()).unwrap_or(format!("{:?}", n));
            if n.starts_with("__") {
                continue;
            }
            println!("    \"{}\"", &n)
        }
        Ok(())
    }
}
