// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Package configuration

use itertools::Itertools;

// ----------------------------------------------------------------------
// - GpgKeyId:
// ----------------------------------------------------------------------

/// A GPG key id (16 hex values)
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct GpgKeyId(String);

impl GpgKeyId {
    /// Create a new `GpgKeyId` from a string input
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

impl std::convert::From<GpgKeyId> for String {
    fn from(key_id: GpgKeyId) -> Self {
        format!("{:}", &key_id)
    }
}

impl std::convert::TryFrom<String> for GpgKeyId {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        GpgKeyId::new(&value[..])
    }
}

impl std::fmt::Display for GpgKeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", self.0)
    }
}

// ----------------------------------------------------------------------
// - Hash:
// ----------------------------------------------------------------------

fn to_hex(input: &[u8]) -> String {
    let mut result = String::with_capacity(input.len() * 2);
    for c in input {
        result.push_str(&format!("{:02x}", c));
    }
    result
}

fn from_hex(input: &str, output: &mut [u8]) -> Result<(), crate::Error> {
    if input.len() != output.len() * 2 {
        return Err(crate::Error::Conversion("Hash value length is invalid."));
    }
    for i in 0..output.len() {
        output[i] = u8::from_str_radix(&input[(i * 2)..(i * 2) + 2], 16)
            .map_err(|_| crate::Error::Conversion("Hash value must be hex characters only."))?;
    }

    Ok(())
}

/// A supported `Hash`
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub enum Hash {
    /// No hash validation needed
    NONE(),
    /// SHA 256
    SHA256([u8; 32]),
    /// SHA 512
    SHA512([u8; 64]),
}

impl Hash {
    /// Create a 'Hash::NONE`
    pub fn none() -> crate::Result<Hash> {
        Ok(Hash::NONE())
    }

    /// Create a `Hash::SHA256`
    pub fn sha256(value: &str) -> crate::Result<Hash> {
        let mut v = [0_u8; 32];
        from_hex(&value, &mut v)?;

        Ok(Hash::SHA256(v))
    }

    /// Create a `Hash::SHA512`
    pub fn sha512(value: &str) -> crate::Result<Hash> {
        let mut v = [0_u8; 64];
        from_hex(&value, &mut v)?;

        Ok(Hash::SHA512(v))
    }

    /// The hash algorithm
    pub fn algorithm(&self) -> String {
        match self {
            Hash::NONE() => String::from("none"),
            Hash::SHA256(_) => String::from("sha256"),
            Hash::SHA512(_) => String::from("sha512"),
        }
    }

    /// The hash value
    pub fn value(&self) -> String {
        match self {
            Hash::NONE() => String::new(),
            Hash::SHA256(v) => to_hex(&v[..]),
            Hash::SHA512(v) => to_hex(&v[..]),
        }
    }
}

impl std::convert::From<Hash> for String {
    fn from(hash: Hash) -> Self {
        format!("{:}", &hash)
    }
}

impl std::convert::TryFrom<String> for Hash {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.to_lowercase();
        if value == "none" {
            Hash::none()
        } else if value.starts_with("sha256:") {
            Hash::sha256(&value[7..])
        } else if value.starts_with("sha512:") {
            Hash::sha512(&value[7..])
        } else {
            Err(crate::Error::Conversion("Unsupported hash type."))
        }
    }
}

impl std::default::Default for Hash {
    fn default() -> Self {
        Hash::NONE()
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Hash::NONE() => write!(f, "{}", self.algorithm()),
            _ => write!(f, "{}:{}", self.algorithm(), self.value()),
        }
    }
}

// ----------------------------------------------------------------------
// - Name:
// ----------------------------------------------------------------------

/// A package `Name`
#[derive(Clone, Debug, PartialOrd, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Name(String);

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

impl std::convert::From<Name> for String {
    fn from(name: Name) -> Self {
        name.0.clone()
    }
}

impl std::convert::TryFrom<String> for Name {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Name::new(&value[..])
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", &self.0)
    }
}

// ----------------------------------------------------------------------
// - Version:
// ----------------------------------------------------------------------

/// A `Version` number
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version {
    /// The distributions package version `epoch`
    epoch: u32,
    /// The upstream `version`
    upstream: String,
    /// The distributions package `release` version
    release: String,
}

impl Version {
    /// Create a package `Version` from an `epoch`, a `version` and an `release`
    pub fn new(epoch: u32, upstream: &str, release: &str) -> crate::Result<Version> {
        if upstream.is_empty() {
            return Err(crate::Error::Conversion(
                "Version part of a package version can not be empty.",
            ));
        }
        if !upstream
            .chars()
            .all(|c| (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || (c == '_') || (c == '.'))
        {
            return Err(crate::Error::Conversion(
                &"Package version must consist of numbers, lowercase letters, '.' or '_' characters only.",
            ));
        }
        if !upstream
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
            upstream: upstream.to_string(),
            release: release.to_string(),
        })
    }

    /// Return the epoch of a `Version`
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Return the epoch of a `Version`
    pub fn upstream(&self) -> String {
        self.upstream.clone()
    }
    /// Return the epoch of a `Version`
    pub fn release(&self) -> String {
        self.release.clone()
    }
}

impl std::convert::From<Version> for String {
    fn from(version: Version) -> Self {
        format!("{:}", &version)
    }
}

impl std::convert::TryFrom<String> for Version {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
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
            !self.upstream.is_empty(),
            !self.release.is_empty(),
        ) {
            (false, true, false) => write!(f, "{:}", self.upstream),
            (false, true, true) => write!(f, "{:}-{:}", self.upstream, self.release),
            (true, true, false) => write!(f, "{:}:{:}", self.epoch, self.upstream),
            (true, true, true) => write!(f, "{:}:{:}-{:}", self.epoch, self.upstream, self.release),
            (_, false, _) => unreachable!("Version was invalid during Display!"),
        }
    }
}

// ----------------------------------------------------------------------
// - Packet:
// ----------------------------------------------------------------------

/// `Package` meta data
#[derive(derive_builder::Builder, Clone, Debug)]
#[builder(try_setter, setter(into))]
pub struct Packet {
    /// The package `name`
    pub name: Name,
    /// The package `version`
    pub version: Version,
    /// A short description of the package
    pub description: String,
    /// The upstream `url`
    pub url: Option<String>,
    /// The upstream bug tracker url
    #[builder(default = "None")]
    pub bug_url: Option<String>,
    /// The upstream license
    pub license: String,

    /// The other packages this Package conflicts with
    #[builder(default = "vec!()")]
    pub conflicts: Vec<Name>,
    /// Abstract interfaces provided by this package
    #[builder(default = "vec!()")]
    pub provides: Vec<Name>,

    /// The other packages this Package conflicts with
    #[builder(default = "vec!()")]
    pub dependencies: Vec<Name>,
    /// Abstract interfaces provided by this package
    #[builder(default = "vec!()")]
    pub optional_dependencies: Vec<Name>,
}

impl std::cmp::PartialEq for Packet {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::convert::From;
    use std::convert::TryFrom;

    use super::{GpgKeyId, Hash, Name, Version};

    #[test]
    fn test_package_gpg_key_id_ok() {
        let key_id = GpgKeyId::new("aB-c D1---23   4EFAB5678").unwrap();
        assert_eq!(key_id.0, "abcd 1234 efab 5678");

        assert_eq!(
            GpgKeyId::try_from(String::from("aB-c D1---23   4EFAB5678")).unwrap(),
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
    fn test_package_hash_ok() {
        assert_eq!(Hash::none().unwrap(), Hash::NONE());

        assert_eq!(
            Hash::sha256("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
                .unwrap()
                .to_string(),
            "sha256:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f".to_string()
        );

        assert_eq!(
            Hash::sha512("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
                .unwrap()
                .to_string(),
            "sha512:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f".to_string()
        );

        assert_eq!(
            Hash::try_from(String::from(
                "sha256:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
            ))
            .unwrap()
            .to_string(),
            "sha256:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f".to_string()
        )
    }

    #[test]
    fn test_package_hash_not_ok() {
        assert!(Hash::try_from(String::from("foobar")).is_err()); // unsupported hash
        assert!(Hash::try_from(String::from("foobar:baz")).is_err()); // unsupported hash
        assert!(Hash::try_from(String::from("sha256:")).is_err()); // No hex
        assert!(Hash::try_from(String::from("sha256:0123424")).is_err()); // too short
        assert!(Hash::try_from(String::from(
            "sha256:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0"
        ))
        .is_err()); // too long
        assert!(Hash::try_from(String::from(
            "sha256:000102030405060708090a0b0cXd0e0f101112131415161718191a1b1c1d1e1f"
        ))
        .is_err()); // not hex
    }

    // Name:
    #[test]
    fn test_package_name_ok() {
        let name = Name::new("test").unwrap();
        assert_eq!(name, Name(String::from("test")));

        let name = Name::try_from(String::from("9_foobar__")).unwrap();
        assert_eq!(name.0, String::from("9_foobar__"));
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
        let name = Name::try_from(String::from("9_foobar__")).unwrap();
        assert_eq!(name.0, String::from("9_foobar__"));
        assert_eq!(String::from(name), String::from("9_foobar__"));
    }

    #[test]
    fn test_package_version_ok() {
        let version = Version::new(43, "test", "foo").unwrap();
        assert_eq!(version.epoch, 43);
        assert_eq!(version.upstream, "test");
        assert_eq!(version.release, "foo");

        assert_eq!(
            Version::try_from(String::from("1")).unwrap(),
            Version::new(0, "1", "").unwrap()
        );
        assert_eq!(
            Version::try_from(String::from("42")).unwrap(),
            Version::new(0, "42", "").unwrap()
        );
        assert_eq!(
            Version::try_from(String::from("42.0")).unwrap(),
            Version::new(0, "42.0", "").unwrap()
        );
        assert_eq!(
            Version::try_from(String::from("42.0_alpha")).unwrap(),
            Version::new(0, "42.0_alpha", "").unwrap()
        );
        assert_eq!(
            Version::try_from(String::from("0:42.0_alpha")).unwrap(),
            Version::new(0, "42.0_alpha", "").unwrap()
        );
        assert_eq!(
            Version::try_from(String::from("23:42.0_alpha")).unwrap(),
            Version::new(23, "42.0_alpha", "").unwrap()
        );
        assert_eq!(
            Version::try_from(String::from("23:42.0_alpha-x")).unwrap(),
            Version::new(23, "42.0_alpha", "x").unwrap()
        );
        assert_eq!(
            Version::try_from(String::from("54:x-42.0_alpha")).unwrap(),
            Version::new(54, "x", "42.0_alpha").unwrap()
        );
        assert_eq!(
            Version::try_from(String::from("54:2.4.5-arch1")).unwrap(),
            Version::new(54, "2.4.5", "arch1").unwrap()
        );
    }

    #[test]
    fn test_package_version_not_ok() {
        assert!(Version::try_from(String::from("")).is_err());

        assert!(Version::try_from(String::from("2.4.5!")).is_err());
        assert!(Version::try_from(String::from("2.4.5!-arch1")).is_err());
        assert!(Version::try_from(String::from("54:2.4.5!-arch1")).is_err());
        assert!(Version::try_from(String::from("54:2.4.5-ärch1")).is_err());

        assert!(Version::try_from(String::from("_2.4.5")).is_err());
        assert!(Version::try_from(String::from("_2.4.5-arch1")).is_err());
        assert!(Version::try_from(String::from("2.4.5-_arch1")).is_err());
        assert!(Version::try_from(String::from("54:2.4.5-_arch1")).is_err());
        assert!(Version::try_from(String::from("_54:2.4.5-arch1")).is_err());

        assert!(Version::try_from(String::from("-1:2.4.5-arch1")).is_err());
        assert!(Version::try_from(String::from("9999999999999999999:2.4.5-arch1")).is_err());
    }

    #[test]
    fn test_package_version_conversion() {
        let version = Version::try_from(String::from("42:foobar-baz")).unwrap();
        assert_eq!(version.epoch, 42);
        assert_eq!(version.upstream, "foobar".to_string());
        assert_eq!(version.release, "baz".to_string());
        assert_eq!(String::from(version), String::from("42:foobar-baz"));

        assert_eq!(
            String::from(Version::new(0, "test", "baz").unwrap()),
            String::from("test-baz")
        );
        assert_eq!(
            String::from(Version::new(1, "test", "baz").unwrap()),
            String::from("1:test-baz")
        );
        assert_eq!(
            String::from(Version::new(0, "test", "baz").unwrap()),
            String::from("test-baz")
        );
        assert_eq!(
            String::from(Version::new(0, "test", "").unwrap()),
            String::from("test")
        );
        assert_eq!(
            String::from(Version::new(1, "test", "").unwrap()),
            String::from("1:test")
        );
    }
}
