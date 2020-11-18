// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Package configuration

use std::convert::From;
use std::convert::TryFrom;

/// A `Version` number
#[derive(Debug, Eq, PartialEq)]
pub struct Version {
    epoch: u32,
    version: String,
    release: String,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (
            self.epoch != 0,
            !self.version.is_empty(),
            !self.release.is_empty(),
        ) {
            (false, true, false) => write!(f, "{:}", self.version),
            (false, true, true) => write!(f, "{:}-{:}", self.version, self.release),
            (true, true, false) => write!(f, "{:}:{:}", self.epoch, self.version),
            (true, true, true) => write!(f, "{:}:{:}-{:}", self.epoch, self.version, self.release),
            (_, false, _) => unreachable!("Version was invalid during Display!"),
        }
    }
}

impl TryFrom<&str> for Version {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let epoch;
        let version;
        let release;

        let input = &value[..];

        let mut index = input.chars().position(|c| c == ':').unwrap_or(0);
        if index > 0 {
            epoch = input[..index]
                .parse::<u32>()
                .or(Err(crate::Error::ConversionError(
                    "Invalid epoch value in version string found.",
                )))?;
            index += 1;
        } else {
            epoch = 0;
        }

        let input = &value[index..];
        let index = input.chars().position(|c| c == '-').unwrap_or(0);
        if index > 0 {
            version = &input[..index];
            release = &input[(index + 1)..];
        } else {
            version = input;
            release = "";
        }

        Version::new(epoch, version, release)
    }
}

impl From<&Version> for String {
    fn from(version: &Version) -> Self {
        format!("{:}", &version)
    }
}

impl Version {
    /// Create a package `Version` from an `epoch`, a `version` and an `release`
    pub fn new(epoch: u32, version: &str, release: &str) -> crate::Result<Version> {
        dbg!("Version::new({:?} : {:?} - {:?})", epoch, version, release);

        if version.is_empty() {
            return Err(crate::Error::ConversionError(
                "Version part of a package version can not be empty.",
            ));
        }
        if !version
            .chars()
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || (c == '_') || (c == '.'))
        {
            return Err(crate::Error::ConversionError(
                &"Package version must consist of numbers, lowercase letters, '.' or '_' characters only.",
            ));
        }
        if !version
            .chars()
            .take(1)
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9'))
        {
            return Err(crate::Error::ConversionError(
                &"Package version must start with a numbers or lowercase letter.",
            ));
        }
        if !release
            .chars()
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || (c == '_') || (c == '.'))
        {
            return Err(crate::Error::ConversionError(
                &"Package version release must consist of numbers, lowercase letters, '.' or '_' characters only.",
            ));
        }
        if !release
            .chars()
            .take(1)
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9'))
        {
            return Err(crate::Error::ConversionError(
                &"Package version release must start with a numbers or lowercase letter.",
            ));
        }

        Ok(Version {
            epoch,
            version: version.to_string(),
            release: release.to_string(),
        })
    }
}

/*
/// A `Hash` used to validate a file
#[derive(Debug)]
pub struct Hash {
    algorithm: String,
    value: String,
}

/// A `Source`
#[derive(Debug)]
pub struct Source {
    url: String,
    hash: Option<Hash>,
    signing_keys: Vec<String>,
    extract: bool,
}
*/

/// A package `Name`
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
struct Name(String);

impl TryFrom<&str> for Name {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Name::new(value)
    }
}

impl From<&Name> for String {
    fn from(name: &Name) -> Self {
        format!("{:}", &name)
    }
}

impl Name {
    /// Create a package 'Name' from a '&str'
    pub fn new(value: &str) -> crate::Result<Name> {
        if value.is_empty() {
            return Err(crate::Error::ConversionError(
                &"Package name can not be empty.",
            ));
        }
        if !value
            .chars()
            .take(1)
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9'))
        {
            return Err(crate::Error::ConversionError(
                &"Package name must start with a number or lowercase letter.",
            ));
        }
        if !value
            .chars()
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || (c == '_'))
        {
            return Err(crate::Error::ConversionError(
                &"Package name must consist of numbers, lowercase letter or '_' characters only.",
            ));
        }
        Ok(Name(value.to_string()))
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", self.0)
    }
}

/*
/// Package `MetaData`
pub struct MetaData {
    name: Name,
    version: Version,
    description: String,
    url: Option<String>,
    bug_url: Option<String>,
    license: String,

    conflicts: Vec<String>,
    provides: Vec<String>,
}

/// Configuration for a Source package
pub struct SourcePackage {
    meta: MetaData,

    sources: Vec<Source>,

    dependencies: Vec<String>,
    build_dependencies: Vec<String>,
    check_dependencies: Vec<String>,
    optional_dependencies: Vec<String>,

    packaging_options: Vec<String>,
}
*/

#[cfg(test)]
mod tests {
    use std::convert::From;
    use std::convert::TryFrom;

    use super::Version;

    #[test]
    fn test_package_version_ok() {
        let version = Version::new(43, "test", "foo").unwrap();
        assert_eq!(version.epoch, 43);
        assert_eq!(version.version, "test");
        assert_eq!(version.release, "foo");

        assert_eq!(
            Version::try_from("1").unwrap(),
            Version::new(0, "1", "").unwrap()
        );
        assert_eq!(
            Version::try_from("42").unwrap(),
            Version::new(0, "42", "").unwrap()
        );
        assert_eq!(
            Version::try_from("42.0").unwrap(),
            Version::new(0, "42.0", "").unwrap()
        );
        assert_eq!(
            Version::try_from("42.0_alpha").unwrap(),
            Version::new(0, "42.0_alpha", "").unwrap()
        );
        assert_eq!(
            Version::try_from("0:42.0_alpha").unwrap(),
            Version::new(0, "42.0_alpha", "").unwrap()
        );
        assert_eq!(
            Version::try_from("23:42.0_alpha").unwrap(),
            Version::new(23, "42.0_alpha", "").unwrap()
        );
        assert_eq!(
            Version::try_from("23:42.0_alpha-x").unwrap(),
            Version::new(23, "42.0_alpha", "x").unwrap()
        );
        assert_eq!(
            Version::try_from("54:x-42.0_alpha").unwrap(),
            Version::new(54, "x", "42.0_alpha").unwrap()
        );
        assert_eq!(
            Version::try_from("54:2.4.5-arch1").unwrap(),
            Version::new(54, "2.4.5", "arch1").unwrap()
        );
    }

    #[test]
    fn test_package_version_not_ok() {
        assert!(Version::try_from("").is_err());

        assert!(Version::try_from("2.4.5!").is_err());
        assert!(Version::try_from("2.4.5!-arch1").is_err());
        assert!(Version::try_from("54:2.4.5!-arch1").is_err());
        assert!(Version::try_from("54:2.4.5-ärch1").is_err());

        assert!(Version::try_from("_2.4.5").is_err());
        assert!(Version::try_from("_2.4.5-arch1").is_err());
        assert!(Version::try_from("2.4.5-_arch1").is_err());
        assert!(Version::try_from("54:2.4.5-_arch1").is_err());
        assert!(Version::try_from("_54:2.4.5-arch1").is_err());

        assert!(Version::try_from("-1:2.4.5-arch1").is_err());
        assert!(Version::try_from("9999999999999999999:2.4.5-arch1").is_err());
    }

    #[test]
    fn test_package_version_conversion() {
        let version = Version::try_from("42:foobar-baz").unwrap();
        assert_eq!(version.epoch, 42);
        assert_eq!(version.version, "foobar".to_string());
        assert_eq!(version.release, "baz".to_string());
        assert_eq!(String::from(&version), "42:foobar-baz".to_string());

        assert_eq!(
            String::from(&Version::new(0, "test", "baz").unwrap()),
            String::from("test-baz")
        );
        assert_eq!(
            String::from(&Version::new(1, "test", "baz").unwrap()),
            String::from("1:test-baz")
        );
        assert_eq!(
            String::from(&Version::new(0, "test", "baz").unwrap()),
            String::from("test-baz")
        );
        assert_eq!(
            String::from(&Version::new(0, "test", "").unwrap()),
            String::from("test")
        );
        assert_eq!(
            String::from(&Version::new(1, "test", "").unwrap()),
            String::from("1:test")
        );
    }

    // Name:
    #[test]
    fn test_package_name_ok() {
        let name = super::Name::new("test").unwrap();
        assert_eq!(name, super::Name("test".to_string()));

        let name = super::Name::try_from("9_foobar__").unwrap();
        assert_eq!(name.0, "9_foobar__".to_string());
    }

    #[test]
    fn test_package_name_not_ok() {
        assert!(super::Name::new("").is_err());
        assert!(super::Name::new("töst").is_err());
        assert!(super::Name::new("teSt").is_err());
        assert!(super::Name::new("Test").is_err());
        assert!(super::Name::new("_foobar").is_err());
        assert!(super::Name::new("").is_err());
    }

    #[test]
    fn test_package_name_conversion() {
        let name = super::Name::try_from("9_foobar__").unwrap();
        assert_eq!(name.0, "9_foobar__".to_string());
        assert_eq!(String::from(&name), "9_foobar__".to_string());
    }
}
