// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

// spell-checker: ignore dedup

use itertools::Itertools;

// ----------------------------------------------------------------------
// - Name:
// ----------------------------------------------------------------------

/// A packet `Name`
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct Name(String);

impl Name {
    /// Create a `Packet` `Name` from a `&str`
    ///
    /// # Errors
    /// * `Error::Conversion`: When the input string is not a valid `Name`
    pub fn new(value: &str) -> crate::Result<Self> {
        if value.is_empty() {
            return Err(crate::Error::Conversion {
                expression: value.to_string(),
                typename: "Name".to_string(),
                message: "Packet name can not be empty.".into(),
            });
        }
        if !crate::start_alphanumerical_char(value) {
            return Err(crate::Error::Conversion {
                expression: value.to_string(),
                typename: "Name".to_string(),
                message: "Packet name must start with a number or lowercase letter.".into(),
            });
        }
        if !crate::all_name_chars(value) {
            return Err(crate::Error::Conversion {
                expression: value.to_string(),
                typename: "Name".to_string(),
                message:
                    "Packet name must consist of numbers, lowercase letter or '_' characters only."
                        .into(),
            });
        }
        Ok(Self(value.to_string()))
    }

    /// Get a list of bytes from a `Name`
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Combine a `Name` with an optional other `Name`, e.g. a packet name and a facet
    #[must_use]
    pub fn combine(&self, other: &Option<Self>) -> String {
        format!(
            "{}{}",
            &self.0,
            (other.as_ref()).map_or_else(String::new, |n| { format!("-{}", &n) }),
        )
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

/// An implicitly sorted and de-duplicated vector of `Name`s
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct Names(Vec<Name>);

impl Names {
    /// Insert a name into the list of `Name`s
    pub fn insert(&mut self, name: Name) -> &mut Self {
        self.merge(&[name])
    }

    /// Merge one vector of `Name`s with another

    pub fn merge(&mut self, names: &[Name]) -> &mut Self {
        self.0.extend_from_slice(names);
        self.fix()
    }

    /// Check whether a `Name` is in this list
    #[must_use]
    pub fn contains(&self, name: &Name) -> bool {
        self.0.contains(name)
    }

    /// Check whether there is at least one `Name`
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the number of entries in the list of `Name`s
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    fn fix(&mut self) -> &mut Self {
        self.0.sort();
        self.0.dedup();
        self
    }
}

impl std::convert::From<Names> for Vec<String> {
    fn from(names: Names) -> Self {
        names.0.iter().map(Name::to_string).collect()
    }
}

impl std::convert::From<Name> for Names {
    fn from(name: Name) -> Self {
        Self(vec![name])
    }
}

impl std::convert::From<&[Name]> for Names {
    fn from(names: &[Name]) -> Self {
        let mut result = Self(names.to_vec());
        result.fix();
        result
    }
}

impl std::convert::TryFrom<Vec<&str>> for Names {
    type Error = crate::Error;

    fn try_from(values: Vec<&str>) -> Result<Self, Self::Error> {
        let mut result = Self(Vec::with_capacity(values.len()));
        for n in values {
            result.0.push(Name::try_from(n)?);
        }
        result.fix();
        Ok(result)
    }
}

impl std::convert::TryFrom<&[String]> for Names {
    type Error = crate::Error;

    fn try_from(values: &[String]) -> Result<Self, Self::Error> {
        let mut result = Self(Vec::with_capacity(values.len()));
        for n in values {
            result.0.push(Name::try_from(&n[..])?);
        }
        result.fix();
        Ok(result)
    }
}

impl std::convert::TryFrom<Vec<String>> for Names {
    type Error = crate::Error;

    fn try_from(values: Vec<String>) -> Result<Self, Self::Error> {
        Self::try_from(&values[..])
    }
}

impl std::fmt::Display for Names {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names_string = self.0.iter().sorted().map(Name::to_string).join(" ");
        write!(f, "{}", &names_string)
    }
}

impl<'a> IntoIterator for &'a Names {
    type Item = &'a Name;

    type IntoIter = std::slice::Iter<'a, Name>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// ----------------------------------------------------------------------
// - Tests:
// ----------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod tests {
    use std::convert::From;
    use std::convert::TryFrom;

    use super::Name;

    // Name:
    #[test]
    fn packet_name_ok() {
        let name = Name::new("test").unwrap();
        assert_eq!(name, Name("test".to_string()));

        let name = Name::try_from("9_foobar__").unwrap();
        assert_eq!(name.0, "9_foobar__");
    }

    #[test]
    fn packet_name_not_ok() {
        assert!(Name::new("").is_err());
        assert!(Name::new("tö_st").is_err());
        assert!(Name::new("teSt").is_err());
        assert!(Name::new("Test").is_err());
        assert!(Name::new("_foobar").is_err());
    }

    #[test]
    fn packet_name_conversion() {
        let name = Name::try_from("9_foobar__").unwrap();
        assert_eq!(name.0, "9_foobar__");
        assert_eq!(String::from(name), "9_foobar__".to_string());
    }
}
