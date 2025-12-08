#![allow(clippy::cargo_common_metadata)]

mod table_builder;
mod version_string;

pub mod errors;
pub mod fmt;
pub mod globals;
pub mod newtypes;
pub mod path;
pub mod process;

pub use self::errors::{
    DatabaseError, InstallError, LuneError, LuneResult, NetworkError, ValidationError,
};
pub use self::newtypes::{
    AbsolutePath, NonEmptyString, PackageName, Port, SocketAddr, Url, Version, VersionReq,
};
pub use self::table_builder::TableBuilder;
pub use self::version_string::get_version_string;

// TODO: Remove this in the next major semver
pub mod jit {
    pub use super::process::ProcessJitEnablement as JitEnablement;
}
