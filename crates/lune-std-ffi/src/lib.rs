//! FFI (Foreign Function Interface) module for Lune.
//!
//! Provides the ability to load native libraries (.dll, .so, .dylib)
//! and call functions from them.

use mlua::prelude::*;

mod library;

pub use library::NativeLibrary;

/// Creates the `ffi` module for Lune.
pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;

    // ffi.open(path: string) -> NativeLibrary
    exports.set(
        "open",
        lua.create_function(|_, path: String| NativeLibrary::open(&path))?,
    )?;

    // ffi.cdef - placeholder for future C definition parsing
    exports.set(
        "cdef",
        lua.create_function(|_, _def: String| -> LuaResult<()> {
            // Future: parse C definitions for type-safe FFI
            Ok(())
        })?,
    )?;

    Ok(exports)
}
