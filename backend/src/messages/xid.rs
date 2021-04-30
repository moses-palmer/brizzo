use std::fmt;
use std::num;
use std::str;
use std::time;

use std::ops::Add;

use serde;

/// An identifier parse error.
pub enum Error {
    /// The string format is invalid.
    Format,

    /// The timestamp is invalid.
    Timestamp,

    /// The cookie has expired.
    Expired,

    /// The cookie is missing.
    Missing,
}

/// A room identifier.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Identifier(u64);

impl From<u64> for Identifier {
    /// Converts a number to an identifier.
    ///
    /// # Arguments
    /// *  `source` - The source number.
    fn from(source: u64) -> Self {
        Identifier(source)
    }
}

impl str::FromStr for Identifier {
    type Err = num::ParseIntError;

    /// Parses a hex number to a room identifier.
    ///
    /// # Arguments
    /// *  `source` - The hex string to parse.
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        u64::from_str_radix(source, 16).map(Identifier)
    }
}

impl fmt::Display for Identifier {
    /// Displays this identifier as a zero padded hex string.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:016X}", self.0)
    }
}

impl<'a> serde::de::Deserialize<'a> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl serde::ser::Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<IdentifierCookie> for Identifier {
    fn from(source: IdentifierCookie) -> Self {
        source.0
    }
}

/// An identifier wrapped in a cookie value.
///
/// Identifiers wrapped in cookie values provide timestamps that are checked
/// when parsing and generated when stringified. Parsing will fail for
/// timestamps generated too far in the past.
pub struct IdentifierCookie(Identifier);

impl IdentifierCookie {
    /// The separator used in the cookie value.
    const SEPARATOR: char = ':';

    /// The maximum age of a cookie.
    const MAX_AGE: time::Duration = time::Duration::from_secs(10);
}

impl str::FromStr for IdentifierCookie {
    type Err = Error;

    /// Parses an identifier cookie value.
    ///
    /// Parsing will fail if the timestamp is too far in the past.
    ///
    /// # Arguments
    /// *  `source` - The string to parse.
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut parts = source.split(Self::SEPARATOR);
        let xid = parts
            .next()
            .and_then(|s| Identifier::from_str(s).ok())
            .ok_or(Error::Format)?;
        let then = time::UNIX_EPOCH.add(
            parts
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .map(time::Duration::from_millis)
                .ok_or(Error::Timestamp)?,
        );
        match time::SystemTime::now().duration_since(then) {
            Ok(d) if d < Self::MAX_AGE => Ok(xid.into()),
            _ => Err(Error::Expired),
        }
    }
}

impl fmt::Display for IdentifierCookie {
    /// Displays this identifier as a zero padded hex string with a timestamp
    /// appended.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{:?}",
            self.0,
            Self::SEPARATOR,
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
    }
}

impl From<Identifier> for IdentifierCookie {
    fn from(source: Identifier) -> Self {
        IdentifierCookie(source)
    }
}
