// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2020 Tobias Hunger <tobias.hunger@gmail.com>

//! Hash related code

// ----------------------------------------------------------------------
// - Helper:
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

// ----------------------------------------------------------------------
// - Hash:
// ----------------------------------------------------------------------

/// A supported `Hash`
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
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

    /// Check whether there is no real hash.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None())
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

    /// Create a `SHA256` from a `sha3::Sha3_256`
    #[must_use]
    pub fn from_sha256(value: sha3::Sha3_256) -> Self {
        use sha3::Digest;

        let result = value.finalize();
        let mut v = [0_u8; 32];
        v.clone_from_slice(&result);

        Self::Sha256(v)
    }
    /// Create a `SHA256` hash by hashing the provided bytes
    #[must_use]
    pub fn calculate_sha256(value: &[u8]) -> Self {
        use sha3::{Digest, Sha3_256};

        let mut hasher = Sha3_256::default();
        hasher.update(&value);
        let result = hasher.finalize();

        let mut v = [0_u8; 32];
        v.clone_from_slice(&result);

        Self::Sha256(v)
    }

    /// Create a `SHA256` hash by hashing the provided bytes
    #[must_use]
    pub fn calculate_sha512(value: &[u8]) -> Self {
        use sha3::{Digest, Sha3_512};

        let mut hasher = Sha3_512::default();
        hasher.update(&value);
        let result = hasher.finalize();

        let mut v = [0_u8; 64];
        v.clone_from_slice(&result);

        Self::Sha512(v)
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

    /// The hash value
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::None() => &[],
            Self::Sha256(v) => &v[..],
            Self::Sha512(v) => &v[..],
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

impl Default for Hash {
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

impl std::cmp::PartialOrd for Hash {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Hash {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self {
            Self::None() => match other {
                Self::None() => std::cmp::Ordering::Equal,
                _ => std::cmp::Ordering::Greater,
            },
            Self::Sha256(sv) => match other {
                Self::None() => std::cmp::Ordering::Greater,
                Self::Sha256(ov) => sv.cmp(ov),
                Self::Sha512(_) => std::cmp::Ordering::Less,
            },
            Self::Sha512(sv) => match other {
                Self::Sha512(ov) => sv.cmp(ov),
                _ => std::cmp::Ordering::Greater,
            },
        }
    }
}

// ----------------------------------------------------------------------
// - HashWriter:
// ----------------------------------------------------------------------

/// Write data into an inner writer `I`, calculating a `Hash` of all the passing data.
pub struct HashedWriter<I>
where
    I: std::io::Write,
{
    inner: I,
    hash: sha3::Sha3_256,
}

impl<I> HashedWriter<I>
where
    I: std::io::Write,
{
    /// Create a new `HashedWriter`
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            hash: sha3::Sha3_256::default(),
        }
    }

    /// Finalize the stream.
    pub fn into_inner(self) -> (Hash, I) {
        (Hash::from_sha256(self.hash), self.inner)
    }
}

impl<I> std::io::Write for HashedWriter<I>
where
    I: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        use sha3::Digest;

        self.hash.update(buf);

        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}
