//! NewType wrappers to avoid primitive obsession.
//!
//! These types provide compile-time validation and semantic meaning.

use std::fmt;
use std::path::PathBuf;

use crate::errors::ValidationError;

/// Validated URL (must start with http:// or https://).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Url(String);

impl Url {
    /// Parse and validate a URL string.
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ValidationError> {
        let s = s.as_ref().trim();
        if s.is_empty() {
            return Err(ValidationError::EmptyValue {
                field: "url".to_owned(),
            });
        }
        if s.starts_with("http://") || s.starts_with("https://") {
            Ok(Self(s.to_owned()))
        } else {
            Err(ValidationError::InvalidUrl(s.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Package name (alphanumeric, hyphens, underscores; must start with letter).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageName(String);

impl PackageName {
    /// Parse and validate a package name.
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ValidationError> {
        let s = s.as_ref().trim();
        if s.is_empty() {
            return Err(ValidationError::EmptyValue {
                field: "package_name".to_owned(),
            });
        }

        let first = s.chars().next().unwrap_or('0');
        if !first.is_ascii_alphabetic() {
            return Err(ValidationError::InvalidPackageName(format!(
                "must start with a letter: {s}"
            )));
        }

        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ValidationError::InvalidPackageName(format!(
                "invalid characters: {s}"
            )));
        }

        Ok(Self(s.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Semantic version wrapper with validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version(semver::Version);

impl Version {
    /// Parse a semantic version string.
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ValidationError> {
        let s = s.as_ref().trim().trim_start_matches('v');
        semver::Version::parse(s)
            .map(Self)
            .map_err(|e| ValidationError::InvalidVersion(e.to_string()))
    }

    #[must_use]
    pub fn inner(&self) -> &semver::Version {
        &self.0
    }

    #[must_use]
    pub fn matches(&self, req: &semver::VersionReq) -> bool {
        req.matches(&self.0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

/// Version requirement/constraint wrapper.
#[derive(Debug, Clone)]
pub struct VersionReq(semver::VersionReq);

impl VersionReq {
    /// Parse a version requirement (e.g., "^1.0.0", ">=2.0", "~1.2").
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ValidationError> {
        semver::VersionReq::parse(s.as_ref())
            .map(Self)
            .map_err(|e| ValidationError::InvalidVersion(e.to_string()))
    }

    #[must_use]
    pub fn matches(&self, version: &Version) -> bool {
        self.0.matches(version.inner())
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Absolute filesystem path (validated to be absolute).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbsolutePath(PathBuf);

impl AbsolutePath {
    /// Create from a path, validating it is absolute.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, ValidationError> {
        let path = path.into();
        if path.is_absolute() {
            Ok(Self(path))
        } else {
            Err(ValidationError::InvalidPath(format!(
                "path must be absolute: {}",
                path.display()
            )))
        }
    }

    /// Create from current dir + relative path.
    pub fn from_relative(relative: impl AsRef<std::path::Path>) -> Result<Self, ValidationError> {
        let cwd =
            std::env::current_dir().map_err(|e| ValidationError::InvalidPath(e.to_string()))?;
        Ok(Self(cwd.join(relative)))
    }

    #[must_use]
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> PathBuf {
        self.0
    }

    #[must_use]
    pub fn join(&self, path: impl AsRef<std::path::Path>) -> Self {
        Self(self.0.join(path))
    }
}

impl fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl AsRef<std::path::Path> for AbsolutePath {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

/// Network port (1-65535).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Port(u16);

impl Port {
    /// Create a validated port number.
    pub fn new(port: u16) -> Result<Self, ValidationError> {
        if port == 0 {
            Err(ValidationError::InvalidPort(port))
        } else {
            Ok(Self(port))
        }
    }

    #[must_use]
    pub fn value(&self) -> u16 {
        self.0
    }
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Socket address combining host and port.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SocketAddr {
    host: String,
    port: Port,
}

impl SocketAddr {
    /// Parse a socket address string (e.g., "127.0.0.1:8080").
    pub fn parse(s: impl AsRef<str>) -> Result<Self, ValidationError> {
        let s = s.as_ref();
        let parts: Vec<&str> = s.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ValidationError::InvalidPath(format!(
                "invalid socket address: {s}"
            )));
        }

        let port: u16 = parts[0]
            .parse()
            .map_err(|_| ValidationError::InvalidPort(0))?;
        let port = Port::new(port)?;
        let host = parts[1].to_owned();

        Ok(Self { host, port })
    }

    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        self.port.value()
    }
}

impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// Non-empty string wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(s: impl AsRef<str>, field: &str) -> Result<Self, ValidationError> {
        let s = s.as_ref().trim();
        if s.is_empty() {
            Err(ValidationError::EmptyValue {
                field: field.to_owned(),
            })
        } else {
            Ok(Self(s.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for NonEmptyString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
