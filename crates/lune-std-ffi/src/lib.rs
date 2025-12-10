//! FFI (Foreign Function Interface) module for Lune.
//!
//! Provides the ability to load native libraries (.dll, .so, .dylib)
//! and call functions from them with arbitrary signatures.
//!
//! # Features
//!
//! - **Dynamic Loading**: Load any native library at runtime
//! - **Dynamic Calls**: Call functions with any signature using libffi
//! - **Type System**: Full C type support (integers, floats, pointers, strings)
//! - **Memory Buffers**: Allocate and manipulate raw memory
//! - **Pointer Support**: Pass and receive pointers seamlessly

#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]

use mlua::prelude::*;

mod callback;
mod caller;
mod library;
mod types;

pub use callback::FfiCallback;
pub use library::{BoundFunction, NativeLibrary};
pub use types::{Buffer, CType};

/// Returns the type definitions for the FFI module.
#[must_use]
pub fn typedefs() -> String {
    include_str!("../types.d.luau").to_string()
}

/// Creates the `ffi` module for Lune.
#[allow(clippy::missing_errors_doc)]
pub fn module(lua: Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;

    // ffi.open(path: string) -> NativeLibrary
    exports.set(
        "open",
        lua.create_function(|_, path: String| NativeLibrary::open(&path))?,
    )?;

    // ffi.buffer(size: number) -> Buffer
    exports.set(
        "buffer",
        lua.create_function(|_, size: usize| Ok(Buffer::new(size)))?,
    )?;

    // ffi.cast(ptr: lightuserdata, type: string) -> value
    exports.set(
        "cast",
        lua.create_function(|lua, (ptr, ctype): (LuaLightUserData, CType)| {
            let buf = Buffer::from_ptr(ptr.0.cast::<u8>(), ctype.size());
            buf.read(lua, 0, ctype)
        })?,
    )?;

    // ffi.string(ptr: lightuserdata, len?: number) -> string
    exports.set(
        "string",
        lua.create_function(|lua, (ptr, len): (LuaLightUserData, Option<usize>)| {
            if ptr.0.is_null() {
                return Ok(LuaValue::Nil);
            }

            let cptr = ptr.0.cast::<i8>();

            if let Some(len) = len {
                let slice = unsafe { std::slice::from_raw_parts(cptr.cast::<u8>(), len) };
                Ok(LuaValue::String(lua.create_string(slice)?))
            } else {
                let cstr = unsafe { std::ffi::CStr::from_ptr(cptr as *const _) };
                Ok(LuaValue::String(lua.create_string(cstr.to_bytes())?))
            }
        })?,
    )?;

    // ffi.null -> null pointer
    exports.set("null", LuaLightUserData(std::ptr::null_mut()))?;

    // ffi.isNull(ptr) -> boolean
    exports.set(
        "isNull",
        lua.create_function(|_, ptr: LuaLightUserData| Ok(ptr.0.is_null()))?,
    )?;

    // ffi.sizeof(type: string) -> number
    exports.set(
        "sizeof",
        lua.create_function(|_, ctype: CType| Ok(ctype.size()))?,
    )?;

    // ffi.alignof(type: string) -> number
    exports.set(
        "alignof",
        lua.create_function(|_, ctype: CType| Ok(ctype.alignment()))?,
    )?;

    // ffi.types - type constants and utilities
    exports.set("types", types::create_types_table(&lua)?)?;

    // ffi.cdef - placeholder for C definition parsing
    exports.set(
        "cdef",
        lua.create_function(|_, _def: String| -> LuaResult<()> { Ok(()) })?,
    )?;

    // ffi.callback(fn, retType, argTypes) -> FfiCallback
    exports.set(
        "callback",
        lua.create_function(
            |lua, (func, ret_type, arg_types): (LuaFunction, CType, LuaTable)| {
                let arg_types: Vec<CType> = arg_types
                    .sequence_values::<CType>()
                    .collect::<LuaResult<Vec<_>>>()?;
                callback::create_callback(lua, func, ret_type, arg_types)
            },
        )?,
    )?;

    Ok(exports)
}
