// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Package configuration

use itertools::Itertools;
use std::convert::From;
use std::convert::TryFrom;

/// A 'Url'
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Url(String);

impl Url {
    /// Create a new `Url` from a string input
    pub fn new(value: &str) -> crate::Result<Url> {
        if !value.contains("://") {
            return Err(crate::Error::Conversion("A URL must contain ://."));
        }
        Ok(Url(value.to_string()))
    }
}

impl TryFrom<&str> for Url {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Url::new(value)
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", self.0)
    }
}

impl From<&Url> for String {
    fn from(url: &Url) -> Self {
        format!("{:}", &url)
    }
}

/// A GPG key id (16 hex values)
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpgKeyId(String);

impl GpgKeyId {
    /// Create a new `Url` from a string input
    pub fn new(value: &str) -> crate::Result<GpgKeyId> {
        let value = value.to_lowercase();
        if !value
            .chars()
            .all(|c| (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c == ' ') || (c == '-'))
        {
            return Err(crate::Error::Conversion(
                "A GPG Key ID must be hex with optional ' ' or '-' characters.",
            ));
        }
        let value = value
            .chars()
            .filter(|c| (*c >= '0' && *c <= '9') || (*c >= 'a' && *c <= 'f'))
            .chunks(4)
            .into_iter()
            .map(|c| c.format(""))
            .join(" ");
        if value.chars().count() != (16 + 3) {
            return Err(crate::Error::Conversion(
                "A GPG Key ID must contain 16 hex digits.",
            ));
        }
        Ok(GpgKeyId(value))
    }
}
impl TryFrom<&str> for GpgKeyId {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        GpgKeyId::new(value)
    }
}

impl std::fmt::Display for GpgKeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", self.0)
    }
}

impl From<&GpgKeyId> for String {
    fn from(key_id: &GpgKeyId) -> Self {
        format!("{:}", &key_id)
    }
}

/// A `Version` number
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Version {
    /// The distributions package version `epoch`
    epoch: u32,
    /// The upstream `version`
    version: String,
    /// The distributions package `release` version
    release: String,
}

impl Version {
    /// Create a package `Version` from an `epoch`, a `version` and an `release`
    pub fn new(epoch: u32, version: &str, release: &str) -> crate::Result<Version> {
        if version.is_empty() {
            return Err(crate::Error::Conversion(
                "Version part of a package version can not be empty.",
            ));
        }
        if !version
            .chars()
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || (c == '_') || (c == '.'))
        {
            return Err(crate::Error::Conversion(
                &"Package version must consist of numbers, lowercase letters, '.' or '_' characters only.",
            ));
        }
        if !version
            .chars()
            .take(1)
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9'))
        {
            return Err(crate::Error::Conversion(
                &"Package version must start with a numbers or lowercase letter.",
            ));
        }
        if !release
            .chars()
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || (c == '_') || (c == '.'))
        {
            return Err(crate::Error::Conversion(
                &"Package version release must consist of numbers, lowercase letters, '.' or '_' characters only.",
            ));
        }
        if !release
            .chars()
            .take(1)
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9'))
        {
            return Err(crate::Error::Conversion(
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
                .or(Err(crate::Error::Conversion(
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

impl From<&Version> for String {
    fn from(version: &Version) -> Self {
        format!("{:}", &version)
    }
}

/// A package `Name`
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Name(String);

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
            return Err(crate::Error::Conversion(&"Package name can not be empty."));
        }
        if !value
            .chars()
            .take(1)
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9'))
        {
            return Err(crate::Error::Conversion(
                &"Package name must start with a number or lowercase letter.",
            ));
        }
        if !value
            .chars()
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || (c == '_'))
        {
            return Err(crate::Error::Conversion(
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

/// Package `MetaData`
#[derive(derive_builder::Builder, Clone, Debug)]
#[builder(try_setter, setter(into))]
pub struct MetaData {
    /// The package `name`
    pub name: Name,
    /// The package `version`
    pub version: Version,
    /// A short description of the package
    pub description: String,
    /// The upstream `url`
    pub url: Url,
    /// The upstream bug tracker url
    #[builder(default = "None")]
    pub bug_url: Option<Url>,
    /// The upstream license
    pub license: String,

    /// The other packages this Package conflicts with
    #[builder(default = "vec!()")]
    pub conflicts: Vec<Name>,
    /// Abstract interfaces provided by this package
    #[builder(default = "vec!()")]
    pub provides: Vec<Name>,
}

/// A binary package in the package database
#[derive(Clone, Debug)]
pub struct Package {
    /// Package `MetaData`
    pub meta: MetaData,
}

#[cfg(test)]
mod tests {
    use std::convert::From;
    use std::convert::TryFrom;

    use super::GpgKeyId;
    use super::Name;
    use super::Url;
    use super::Version;

    #[test]
    fn test_package_url_ok() {
        let url = Url::new("https://foo.bar/").unwrap();
        assert_eq!(url.0, "https://foo.bar/");

        assert_eq!(
            Url::try_from("file:///some/where/").unwrap(),
            Url::new("file:///some/where/").unwrap()
        )
    }

    #[test]
    fn test_package_gpg_key_id_ok() {
        let key_id = GpgKeyId::new("aB-c D1---23   4EFAB5678").unwrap();
        assert_eq!(key_id.0, "abcd 1234 efab 5678");

        assert_eq!(
            GpgKeyId::try_from("aB-c D1---23   4EFAB5678").unwrap(),
            GpgKeyId::new("ABCD1234EFAB5678").unwrap()
        )
    }

    #[test]
    fn test_package_gpg_key_id_not_ok() {
        assert!(GpgKeyId::new("").is_err());
        assert!(GpgKeyId::new("aB-c D1---23   4EFA5G78").is_err());
        assert!(GpgKeyId::new("aB-c D1---23   4EFAB5678 0").is_err());
    }

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
        let name = Name::new("test").unwrap();
        assert_eq!(name, Name("test".to_string()));

        let name = Name::try_from("9_foobar__").unwrap();
        assert_eq!(name.0, "9_foobar__".to_string());
    }

    #[test]
    fn test_package_name_not_ok() {
        assert!(Name::new("").is_err());
        assert!(Name::new("töst").is_err());
        assert!(Name::new("teSt").is_err());
        assert!(Name::new("Test").is_err());
        assert!(Name::new("_foobar").is_err());
        assert!(Name::new("").is_err());
    }

    #[test]
    fn test_package_name_conversion() {
        let name = Name::try_from("9_foobar__").unwrap();
        assert_eq!(name.0, "9_foobar__".to_string());
        assert_eq!(String::from(&name), "9_foobar__".to_string());
    }
}
