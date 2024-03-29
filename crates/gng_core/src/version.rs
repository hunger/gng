// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

#![allow(clippy::default_trait_access)] // To work around a warning in code generated by derive_builder!

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
        if !crate::start_alphanumerical_char(upstream) {
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
        if !crate::start_alphanumerical_char(release) {
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

impl From<Version> for String {
    fn from(version: Version) -> Self {
        format!("{:}", &version)
    }
}

impl TryFrom<&str> for Version {
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

impl TryFrom<String> for Version {
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

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let epoch_cmp = self.epoch.cmp(&other.epoch);
        if epoch_cmp == std::cmp::Ordering::Equal {
            let upstream_cmp = self.upstream.cmp(&other.upstream);
            if upstream_cmp == std::cmp::Ordering::Equal {
                self.release.cmp(&other.release)
            } else {
                upstream_cmp
            }
        } else {
            epoch_cmp
        }
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod tests {
    use super::Version;

    // Version:
    #[test]
    fn package_version_ok() {
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
    fn package_version_not_ok() {
        assert!(Version::try_from("").is_err());

        assert!(Version::try_from("2.4.5!").is_err());
        assert!(Version::try_from("2.4.5!-arch1").is_err());
        assert!(Version::try_from("54:2.4.5!-arch1").is_err());
        assert!(Version::try_from("54:2.4.5-är1").is_err());

        assert!(Version::try_from("_2.4.5").is_err());
        assert!(Version::try_from("_2.4.5-arch1").is_err());
        assert!(Version::try_from("2.4.5-_arch1").is_err());
        assert!(Version::try_from("54:2.4.5-_arch1").is_err());
        assert!(Version::try_from("_54:2.4.5-arch1").is_err());

        assert!(Version::try_from("-1:2.4.5-arch1").is_err());
        assert!(Version::try_from("9999999999999999999:2.4.5-arch1").is_err());
    }

    #[test]
    fn package_version_conversion() {
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
