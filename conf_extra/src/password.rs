//! Helper type for representing passwords in command line arguments etc. that should not be logged.
//! Zeroizes on drop.
use core::{fmt, str::FromStr};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// A wrapper around a password string which does not log the password
/// in display or debug output.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(from = "String", into = "PasswordHelper"))]
pub struct Password(String);

impl Password {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for Password {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Password {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl FromStr for Password {
    type Err = std::convert::Infallible; // This should actually be !
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl fmt::Display for Password {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "***password***")
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "***password***")
    }
}

impl Drop for Password {
    fn drop(&mut self) {
        unsafe { self.0.as_mut_vec().zeroize() }
    }
}

// Helper to make serde serialize using display
// See serde (into =) attribute documentation.
// https://serde.rs/container-attrs.html
#[cfg(feature = "serde")]
#[derive(Serialize)]
#[serde(transparent)]
struct PasswordHelper(String);

#[cfg(feature = "serde")]
impl From<Password> for PasswordHelper {
    fn from(src: Password) -> PasswordHelper {
        PasswordHelper(src.to_string())
    }
}
