// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Package configuration

use itertools::Itertools;

// ----------------------------------------------------------------------
// - GpgKeyId:
// ----------------------------------------------------------------------

/// A GPG key id (16 hex values)
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "&str", into = "String")]
pub struct GpgKeyId(String);

impl GpgKeyId {
    /// Create a new `GpgKeyId` from a string input
    ///
    /// # Errors
    /// * `Error::Conversion`: When the input string is not a valid GPG Key ID
    pub fn new(value: &str) -> crate::Result<Self> {
        let value = value.to_lowercase();
        if !crate::all_hex_or_separator(&value) {
            return Err(crate::Error::Conversion {
                expression: value,
                typename: "GpgKeyId".to_string(),
                message: "A GPG Key ID must be hex with optional ' ' or '-' characters.".into(),
            });
        }
        let value = value
            .chars()
            .filter(|c| (*c >= '0' && *c <= '9') || (*c >= 'a' && *c <= 'f'))
            .chunks(4)
            .into_iter()
            .map(|c| c.format(""))
            .join(" ");
        if value.chars().count() != (16 + 3) {
            return Err(crate::Error::Conversion {
                expression: value,
                typename: "GpgKeyId".to_string(),
                message: "A GPG Key ID must contain 16 hex digits.".into(),
            });
        }
        Ok(Self(value))
    }
}

impl std::convert::From<GpgKeyId> for String {
    fn from(key_id: GpgKeyId) -> Self {
        format!("{:}", &key_id)
    }
}

impl std::convert::TryFrom<&str> for GpgKeyId {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::convert::TryFrom<String> for GpgKeyId {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
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

fn from_hex(input: &str, output: &mut [u8]) -> crate::Result<()> {
    if input.len() != output.len() * 2 {
        return Err(crate::Error::Conversion {
            expression: input.to_string(),
            typename: "Hash".to_string(),
            message: "Hash value length is invalid.".into(),
        });
    }
    for i in 0..output.len() {
        output[i] = u8::from_str_radix(&input[(i * 2)..(i * 2) + 2], 16).map_err(|e| {
            crate::Error::Conversion {
                expression: input.to_string(),
                typename: "Hash".to_string(),
                message: format!("Hex conversion failed: {}", e.to_string()),
            }
        })?;
    }

    Ok(())
}

/// A supported `Hash`
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "&str", into = "String")]
pub enum Hash {
    /// No hash validation needed
    None(),
    /// SHA 256
    Sha256([u8; 32]),
    /// SHA 512
    Sha512([u8; 64]),
}

impl Hash {
    /// Create a `NONE` hash
    #[must_use]
    pub const fn none() -> Self {
        Self::None()
    }

    /// Create a `SHA256` hash
    ///
    /// # Errors
    /// * `Error::Conversion`: When the input string is not a valid `Hash`
    pub fn sha256(value: &str) -> crate::Result<Self> {
        let mut v = [0_u8; 32];
        from_hex(value, &mut v)?;

        Ok(Self::Sha256(v))
    }

    /// Create a `SHA512` hash
    ///
    /// # Errors
    /// * `Error::Conversion`: When the input string is not a valid `Hash`
    pub fn sha512(value: &str) -> crate::Result<Self> {
        let mut v = [0_u8; 64];
        from_hex(value, &mut v)?;

        Ok(Self::Sha512(v))
    }

    /// The hash algorithm
    #[must_use]
    pub const fn algorithm(&self) -> &'static str {
        match self {
            Self::None() => "none",
            Self::Sha256(_) => "sha256",
            Self::Sha512(_) => "sha512",
        }
    }

    /// The hash value
    #[must_use]
    pub fn value(&self) -> String {
        match self {
            Self::None() => String::new(),
            Self::Sha256(v) => to_hex(&v[..]),
            Self::Sha512(v) => to_hex(&v[..]),
        }
    }
}

impl std::convert::From<Hash> for String {
    fn from(hash: Hash) -> Self {
        format!("{:}", &hash)
    }
}

impl std::convert::TryFrom<&str> for Hash {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.to_lowercase();
        if value == "none" {
            return Ok(Self::none());
        }
        if let Some(v) = value.strip_prefix("sha256:") {
            return Self::sha256(v);
        }
        if let Some(v) = value.strip_prefix("sha512:") {
            return Self::sha512(v);
        }
        Err(crate::Error::Conversion {
            expression: value,
            typename: "Hash".to_string(),
            message: "Unsupported hash type.".into(),
        })
    }
}

impl std::convert::TryFrom<String> for Hash {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl std::default::Default for Hash {
    fn default() -> Self {
        Self::None()
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::None() => write!(f, "{}", self.algorithm()),
            _ => write!(f, "{}:{}", self.algorithm(), self.value()),
        }
    }
}

// ----------------------------------------------------------------------
// - Name:
// ----------------------------------------------------------------------

/// A package `Name`
#[derive(Clone, Debug, PartialOrd, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "&str", into = "String")]
pub struct Name(String);

impl Name {
    /// Create a package 'Name' from a '&str'
    ///
    /// # Errors
    /// * `Error::Conversion`: When the input string is not a valid `Name`
    pub fn new(value: &str) -> crate::Result<Self> {
        if value.is_empty() {
            return Err(crate::Error::Conversion {
                expression: value.to_string(),
                typename: "Name".to_string(),
                message: "Package name can not be empty.".into(),
            });
        }
        if !crate::start_alnum_char(value) {
            return Err(crate::Error::Conversion {
                expression: value.to_string(),
                typename: "Name".to_string(),
                message: "Package name must start with a number or lowercase letter.".into(),
            });
        }
        if !crate::all_name_chars(value) {
            return Err(crate::Error::Conversion {
                expression: value.to_string(),
                typename: "Name".to_string(),
                message:
                    "Package name must consist of numbers, lowercase letter or '_' characters only."
                        .into(),
            });
        }
        Ok(Self(value.to_string()))
    }
}

impl std::convert::From<Name> for String {
    fn from(name: Name) -> Self {
        name.0
    }
}

impl std::convert::TryFrom<&str> for Name {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::convert::TryFrom<String> for Name {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:}", &self.0)
    }
}

// ----------------------------------------------------------------------
// - Packet:
// ----------------------------------------------------------------------

/// `Packet` meta data
#[derive(derive_builder::Builder, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[builder(try_setter, setter(into))]
pub struct Packet {
    /// The source package `name`
    #[builder(try_setter)]
    pub source_name: Name,
    /// The package `version`
    #[builder(try_setter)]
    pub version: Version,
    /// `license`
    pub license: String,

    /// The package `name`
    #[builder(try_setter)]
    pub name: Name,

    /// A short description of the package
    pub description: String,
    /// The upstream `url`
    #[builder(default = "None")]
    pub url: Option<String>,
    /// The upstream bug tracker url
    #[builder(default = "None")]
    pub bug_url: Option<String>,

    /// The other packages this Package conflicts with
    #[builder(default = "vec!()")]
    pub conflicts: Vec<Name>,
    /// Abstract interfaces provided by this package
    #[builder(default = "vec!()")]
    pub provides: Vec<Name>,

    /// `Packet`s this `Packet` depends on.
    #[builder(default = "vec!()")]
    pub dependencies: Vec<Name>,
}

impl std::cmp::PartialEq for Packet {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version
    }
}

impl Packet {
    /// Create a simple `Packet` with all necessary fields set to "unknown"
    #[must_use]
    pub fn unknown_packet() -> Self {
        PacketBuilder::default()
            .try_source_name("unknown")
            .expect("Name was valid")
            .try_version("unknown")
            .expect("Version was valid")
            .license("unknown")
            .try_name("unknown")
            .expect("Name was valid")
            .description("unknown")
            .build()
            .expect("This should return a valid `Packet`.")
    }
}

// ----------------------------------------------------------------------
// - Version:
// ----------------------------------------------------------------------

/// A `Version` number
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "&str", into = "String")]
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
    ///
    /// # Errors
    /// * `Error::Conversion`: When the input string is not a valid `Version`
    pub fn new(epoch: u32, upstream: &str, release: &str) -> crate::Result<Self> {
        if upstream.is_empty() {
            return Err(crate::Error::Conversion {
                expression: upstream.to_string(),
                typename: "Version".to_string(),
                message: "Version part of a package version can not be empty.".into(),
            });
        }
        if !crate::all_version_chars(upstream) {
            return Err(crate::Error::Conversion{
                expression: upstream.to_string(),
                typename: "Version".to_string(),
                message: "Package version must consist of numbers, lowercase letters, '.' or '_' characters only.".into(),
            });
        }
        if !crate::start_alnum_char(upstream) {
            return Err(crate::Error::Conversion {
                expression: upstream.to_string(),
                typename: "Version".to_string(),
                message: "Package version must start with a numbers or lowercase letter.".into(),
            });
        }
        if !crate::all_version_chars(release) {
            return Err(crate::Error::Conversion{
                expression: release.to_string(),
                typename: "Version".to_string(),
                message: "Package version release must consist of numbers, lowercase letters, '.' or '_' characters only.".into(),
            });
        }
        if !crate::start_alnum_char(release) {
            return Err(crate::Error::Conversion {
                expression: release.to_string(),
                typename: "Version".to_string(),
                message: "Package version release must start with a numbers or lowercase letter."
                    .into(),
            });
        }

        Ok(Self {
            epoch,
            upstream: upstream.to_string(),
            release: release.to_string(),
        })
    }

    /// Return the epoch of a `Version`
    #[must_use]
    pub const fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Return the epoch of a `Version`
    #[must_use]
    pub fn upstream(&self) -> String {
        self.upstream.clone()
    }

    /// Return the epoch of a `Version`
    #[must_use]
    pub fn release(&self) -> String {
        self.release.clone()
    }
}

impl std::convert::From<Version> for String {
    fn from(version: Version) -> Self {
        format!("{:}", &version)
    }
}

impl std::convert::TryFrom<&str> for Version {
    type Error = crate::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let epoch;
        let upstream;
        let release;

        let epoch_upstream_release = value;
        let mut colon_index = epoch_upstream_release
            .chars()
            .position(|c| c == ':')
            .unwrap_or(0);
        if colon_index > 0 {
            epoch = epoch_upstream_release[..colon_index]
                .parse::<u32>()
                .map_err(|e| crate::Error::Conversion {
                    expression: e.to_string(),
                    typename: "Version".to_string(),
                    message: "Invalid epoch value".into(),
                })?;
            colon_index += 1;
        } else {
            epoch = 0;
        }

        let upstream_and_release = &value[colon_index..];
        let dash_index = upstream_and_release
            .chars()
            .position(|c| c == '-')
            .unwrap_or(0);
        if dash_index > 0 {
            upstream = &upstream_and_release[..dash_index];
            release = &upstream_and_release[(dash_index + 1)..];
        } else {
            upstream = upstream_and_release;
            release = "";
        }

        Self::new(epoch, upstream, release)
    }
}

impl std::convert::TryFrom<String> for Version {
    type Error = crate::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
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
    fn test_package_hash_ok() {
        assert_eq!(Hash::none(), Hash::None());

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
            Hash::try_from(
                "sha256:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
            )
            .unwrap()
            .to_string(),
            "sha256:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f".to_string()
        )
    }

    #[test]
    fn test_package_hash_not_ok() {
        assert!(Hash::try_from("foobar").is_err()); // unsupported hash
        assert!(Hash::try_from("foobar:baz").is_err()); // unsupported hash
        assert!(Hash::try_from("sha256:").is_err()); // No hex
        assert!(Hash::try_from("sha256:0123424").is_err()); // too short
        assert!(Hash::try_from(
            "sha256:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0"
        )
        .is_err()); // too long
        assert!(Hash::try_from(
            "sha256:000102030405060708090a0b0cXd0e0f101112131415161718191a1b1c1d1e1f"
        )
        .is_err()); // not hex
    }

    // Name:
    #[test]
    fn test_package_name_ok() {
        let name = Name::new("test").unwrap();
        assert_eq!(name, Name("test".to_string()));

        let name = Name::try_from("9_foobar__").unwrap();
        assert_eq!(name.0, "9_foobar__");
    }

    #[test]
    fn test_package_name_not_ok() {
        assert!(Name::new("").is_err());
        assert!(Name::new("töst").is_err());
        assert!(Name::new("teSt").is_err());
        assert!(Name::new("Test").is_err());
        assert!(Name::new("_foobar").is_err());
    }

    #[test]
    fn test_package_name_conversion() {
        let name = Name::try_from("9_foobar__").unwrap();
        assert_eq!(name.0, "9_foobar__");
        assert_eq!(String::from(name), "9_foobar__".to_string());
    }

    // Version:
    #[test]
    fn test_package_version_ok() {
        let version = Version::new(43, "test", "foo").unwrap();
        assert_eq!(version.epoch, 43);
        assert_eq!(version.upstream, "test");
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
        assert_eq!(version.upstream, "foobar".to_string());
        assert_eq!(version.release, "baz".to_string());
        assert_eq!(String::from(version), "42:foobar-baz".to_string());

        assert_eq!(
            Version::new(0, "test", "baz").unwrap().to_string(),
            "test-baz"
        );
        assert_eq!(
            Version::new(1, "test", "baz").unwrap().to_string(),
            "1:test-baz"
        );
        assert_eq!(
            Version::new(0, "test", "baz").unwrap().to_string(),
            "test-baz"
        );
        assert_eq!(Version::new(0, "test", "").unwrap().to_string(), "test");
        assert_eq!(Version::new(1, "test", "").unwrap().to_string(), "1:test");
    }
}
