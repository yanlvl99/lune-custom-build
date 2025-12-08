//! Hierarchical error types for Lune runtime.
//!
//! All errors follow zero-panic policy - no `.unwrap()` or `.expect()` in production.

use thiserror::Error;

/// Root error type for Lune runtime.
#[derive(Error, Debug)]
pub enum LuneError {
    #[error(transparent)]
    Network(#[from] NetworkError),

    #[error(transparent)]
    Install(#[from] InstallError),

    #[error(transparent)]
    Database(#[from] DatabaseError),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Lua error: {0}")]
    Lua(#[from] mlua::Error),
}

/// Network-related errors (UDP, TCP, HTTP).
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Failed to bind to {address}: {source}")]
    BindFailed {
        address: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Connection refused to {host}:{port}")]
    ConnectionRefused { host: String, port: u16 },

    #[error("Connection reset by peer")]
    ConnectionReset,

    #[error("DNS resolution failed for {domain}")]
    DnsResolutionFailed { domain: String },

    #[error("Timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("Send failed: {source}")]
    SendFailed {
        #[source]
        source: std::io::Error,
    },

    #[error("Receive failed: {source}")]
    ReceiveFailed {
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid address format: {0}")]
    InvalidAddress(String),

    #[error("HTTP error: {status} - {message}")]
    HttpError { status: u16, message: String },

    #[error("TLS error: {0}")]
    TlsError(String),
}

/// Package installation errors.
#[derive(Error, Debug)]
pub enum InstallError {
    #[error("Package '{name}' not found in registry")]
    PackageNotFound { name: String },

    #[error("Version '{version}' incompatible with constraint '{constraint}'")]
    VersionMismatch { version: String, constraint: String },

    #[error("No compatible version found for '{package}' with constraint '{constraint}'")]
    NoCompatibleVersion { package: String, constraint: String },

    #[error("Git clone failed for {url}: {message}")]
    GitCloneFailed { url: String, message: String },

    #[error("Invalid config at {path}: {reason}")]
    InvalidConfig { path: String, reason: String },

    #[error("Registry fetch failed: {0}")]
    RegistryFetchFailed(String),

    #[error("Checksum mismatch for {package}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    #[error("Transaction rollback: {reason}")]
    TransactionRollback { reason: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Database-related errors.
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Query execution failed: {0}")]
    QueryFailed(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Invalid query: parameters count mismatch (expected {expected}, got {actual})")]
    ParameterMismatch { expected: usize, actual: usize },

    #[error("Type conversion error: cannot convert {from_type} to {to_type}")]
    TypeConversion { from_type: String, to_type: String },

    #[error("SQLite error: {0}")]
    Sqlite(String),
}

/// Validation errors for newtypes.
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Invalid port: {0}")]
    InvalidPort(u16),

    #[error("Empty value not allowed for {field}")]
    EmptyValue { field: String },
}

/// Result type alias for Lune operations.
pub type LuneResult<T> = Result<T, LuneError>;

impl From<git2::Error> for InstallError {
    fn from(e: git2::Error) -> Self {
        Self::GitCloneFailed {
            url: String::new(),
            message: e.message().to_owned(),
        }
    }
}

impl From<reqwest::Error> for InstallError {
    fn from(e: reqwest::Error) -> Self {
        Self::RegistryFetchFailed(e.to_string())
    }
}

impl From<serde_json::Error> for InstallError {
    fn from(e: serde_json::Error) -> Self {
        Self::InvalidConfig {
            path: String::new(),
            reason: e.to_string(),
        }
    }
}
