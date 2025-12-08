//! SQL Standard Library for Lune.
//!
//! Provides safe SQLite database access with mandatory parameterized queries
//! to prevent SQL injection attacks.

#![allow(clippy::cargo_common_metadata)]

use lune_utils::TableBuilder;
use mlua::prelude::*;

mod connection;
mod statement;
mod value;

pub use connection::SqlConnection;

const TYPEDEFS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/types.d.luau"));

/// Returns type definitions for the `sql` standard library.
#[must_use]
pub fn typedefs() -> String {
    TYPEDEFS.to_string()
}

/// Creates the `sql` standard library module.
///
/// # Errors
///
/// Errors when out of memory.
pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    TableBuilder::new(lua)?
        .with_function("open", sql_open)?
        .with_function("memory", sql_memory)?
        .build_readonly()
}

fn sql_open(_: &Lua, path: String) -> LuaResult<SqlConnection> {
    SqlConnection::open(&path)
}

fn sql_memory(_: &Lua, (): ()) -> LuaResult<SqlConnection> {
    SqlConnection::memory()
}
