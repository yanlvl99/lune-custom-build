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
//! - **Zero-Copy Access**: Direct memory read/write without allocations
//! - **Struct Mapper**: C-ABI compliant struct layout with field access
//! - **Memory Arenas**: Scoped allocators with automatic cleanup

#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]

use mlua::prelude::*;
use std::ffi::c_void;
use std::ptr;

mod arena;
mod callback;
mod caller;
mod library;
mod pointer;
mod struct_mapper;
mod types;

pub use arena::Arena;
pub use callback::FfiCallback;
pub use library::{BoundFunction, NativeLibrary};
pub use pointer::{RawPointer, TypedPointer};
pub use struct_mapper::{StructDefinition, StructView};
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

    // ========================================================================
    // Library Loading
    // ========================================================================

    // ffi.load(path: string) -> NativeLibrary (Modern API)
    exports.set(
        "load",
        lua.create_function(|_, path: String| NativeLibrary::open(&path))?,
    )?;

    // ffi.open(path: string) -> NativeLibrary (Legacy alias)
    exports.set(
        "open",
        lua.create_function(|_, path: String| NativeLibrary::open(&path))?,
    )?;

    // ========================================================================
    // Memory Allocation
    // ========================================================================

    // ffi.buffer(size: number) -> Buffer
    exports.set(
        "buffer",
        lua.create_function(|_, size: usize| Ok(Buffer::new(size)))?,
    )?;

    // ffi.arena() -> Arena
    exports.set("arena", lua.create_function(|_, ()| Ok(Arena::new()))?)?;

    // ========================================================================
    // Zero-Copy Memory Access (Core Primitives)
    // ========================================================================

    // ffi.read(ptr, offset, type) -> value
    // Direct memory read without buffer wrapper
    exports.set(
        "read",
        lua.create_function(
            |lua, (ptr, offset, ctype): (LuaAnyUserData, usize, CType)| {
                // Try RawPointer first
                if let Ok(raw) = ptr.borrow::<RawPointer>() {
                    return raw.read(lua, offset, ctype);
                }
                // Try TypedPointer
                if let Ok(typed) = ptr.borrow::<TypedPointer>() {
                    let byte_ptr = unsafe { typed.addr.cast::<u8>().add(offset) };
                    return pointer::read_value_at(lua, byte_ptr, ctype);
                }
                // Try Buffer
                if let Ok(buf) = ptr.borrow::<Buffer>() {
                    return buf.read(lua, offset, ctype);
                }
                Err(LuaError::external(
                    "Expected RawPointer, TypedPointer, or Buffer",
                ))
            },
        )?,
    )?;

    // ffi.write(ptr, offset, type, val) -> void
    // Direct memory write without buffer wrapper
    exports.set(
        "write",
        lua.create_function(
            |lua, (ptr, offset, ctype, value): (LuaAnyUserData, usize, CType, LuaValue)| {
                // Try RawPointer first
                if let Ok(raw) = ptr.borrow::<RawPointer>() {
                    return raw.write(lua, offset, ctype, value);
                }
                // Try TypedPointer
                if let Ok(typed) = ptr.borrow::<TypedPointer>() {
                    let byte_ptr = unsafe { typed.addr.cast::<u8>().add(offset) };
                    return pointer::write_value_at(lua, byte_ptr, ctype, value);
                }
                // Try Buffer
                if let Ok(mut buf) = ptr.borrow_mut::<Buffer>() {
                    return buf.write(lua, offset, ctype, value);
                }
                Err(LuaError::external(
                    "Expected RawPointer, TypedPointer, or Buffer",
                ))
            },
        )?,
    )?;

    // ffi.copy(dst, src, len) -> void
    // SIMD-optimized memcpy
    exports.set(
        "copy",
        lua.create_function(
            |_, (dst, src, len): (LuaAnyUserData, LuaAnyUserData, usize)| {
                let dst_ptr = get_raw_ptr(&dst)?;
                let src_ptr = get_raw_ptr(&src)?;

                if dst_ptr.is_null() || src_ptr.is_null() {
                    return Err(LuaError::external("Cannot copy to/from null pointer"));
                }

                unsafe {
                    ptr::copy_nonoverlapping(src_ptr.cast::<u8>(), dst_ptr.cast::<u8>(), len);
                }
                Ok(())
            },
        )?,
    )?;

    // ffi.fill(ptr, len, byte) -> void
    // SIMD-optimized memset
    exports.set(
        "fill",
        lua.create_function(|_, (ptr, len, byte): (LuaAnyUserData, usize, u8)| {
            let raw_ptr = get_raw_ptr(&ptr)?;

            if raw_ptr.is_null() {
                return Err(LuaError::external("Cannot fill null pointer"));
            }

            unsafe {
                ptr::write_bytes(raw_ptr.cast::<u8>(), byte, len);
            }
            Ok(())
        })?,
    )?;

    // ========================================================================
    // Pointer Operations
    // ========================================================================

    // ffi.ptr(addr) -> RawPointer
    // Create a raw pointer from an address (for interop)
    exports.set(
        "ptr",
        lua.create_function(|_, addr: usize| Ok(RawPointer::new(addr as *mut c_void)))?,
    )?;

    // ffi.cast(ptr, type) -> TypedPointer or StructView
    // Cast raw pointer to typed pointer for array indexing
    exports.set(
        "cast",
        lua.create_function(|lua, (ptr, type_val): (LuaValue, LuaValue)| {
            // Get the raw pointer
            let raw = match ptr {
                LuaValue::UserData(ud) => {
                    if let Ok(raw) = ud.borrow::<RawPointer>() {
                        *raw
                    } else if let Ok(typed) = ud.borrow::<TypedPointer>() {
                        RawPointer {
                            addr: typed.addr,
                            arena_id: typed.arena_id,
                            size_hint: typed.element_count * typed.stride,
                        }
                    } else if let Ok(buf) = ud.borrow::<Buffer>() {
                        RawPointer::new(buf.as_ptr().cast())
                    } else {
                        return Err(LuaError::external("Expected pointer or buffer"));
                    }
                }
                LuaValue::LightUserData(lud) => RawPointer::new(lud.0),
                _ => return Err(LuaError::external("Expected pointer")),
            };

            // Get the type
            match type_val {
                LuaValue::String(s) => {
                    let type_str = s.to_str()?;
                    let ctype = CType::from_str(&type_str)
                        .ok_or_else(|| LuaError::external(format!("Unknown type: {}", type_str)))?;
                    TypedPointer::new(&raw, ctype).into_lua(lua)
                }
                LuaValue::UserData(ud) => {
                    if let Ok(def) = ud.borrow::<StructDefinition>() {
                        StructView::new(&raw, def.clone()).into_lua(lua)
                    } else {
                        Err(LuaError::external(
                            "Expected type string or StructDefinition",
                        ))
                    }
                }
                _ => Err(LuaError::external(
                    "Expected type string or StructDefinition",
                )),
            }
        })?,
    )?;

    // ========================================================================
    // Struct System
    // ========================================================================

    // ffi.struct(schema) -> StructDefinition
    exports.set(
        "struct",
        lua.create_function(|lua, schema: LuaTable| StructDefinition::from_schema(lua, schema))?,
    )?;

    // ffi.view(ptr, structDef) -> StructView
    exports.set(
        "view",
        lua.create_function(|_, (ptr, def): (LuaAnyUserData, LuaAnyUserData)| {
            let raw = if let Ok(r) = ptr.borrow::<RawPointer>() {
                *r
            } else if let Ok(typed) = ptr.borrow::<TypedPointer>() {
                RawPointer {
                    addr: typed.addr,
                    arena_id: typed.arena_id,
                    size_hint: typed.element_count * typed.stride,
                }
            } else {
                return Err(LuaError::external("Expected pointer"));
            };

            let struct_def = def.borrow::<StructDefinition>()?;
            Ok(StructView::new(&raw, struct_def.clone()))
        })?,
    )?;

    // ========================================================================
    // String Operations
    // ========================================================================

    // ffi.string(ptr: lightuserdata, len?: number) -> string
    exports.set(
        "string",
        lua.create_function(|lua, (ptr, len): (LuaValue, Option<usize>)| {
            let raw_ptr = match ptr {
                LuaValue::LightUserData(lud) => lud.0,
                LuaValue::UserData(ud) => {
                    if let Ok(raw) = ud.borrow::<RawPointer>() {
                        raw.addr
                    } else if let Ok(typed) = ud.borrow::<TypedPointer>() {
                        typed.addr
                    } else {
                        return Err(LuaError::external("Expected pointer"));
                    }
                }
                _ => return Err(LuaError::external("Expected pointer")),
            };

            if raw_ptr.is_null() {
                return Ok(LuaValue::Nil);
            }

            let cptr = raw_ptr.cast::<i8>();

            if let Some(len) = len {
                let slice = unsafe { std::slice::from_raw_parts(cptr.cast::<u8>(), len) };
                Ok(LuaValue::String(lua.create_string(slice)?))
            } else {
                let cstr = unsafe { std::ffi::CStr::from_ptr(cptr as *const _) };
                Ok(LuaValue::String(lua.create_string(cstr.to_bytes())?))
            }
        })?,
    )?;

    // ========================================================================
    // Null Pointer
    // ========================================================================

    // ffi.null -> null pointer
    exports.set("null", LuaLightUserData(std::ptr::null_mut()))?;

    // ffi.nullPtr -> RawPointer(null)
    exports.set("nullPtr", RawPointer::new(std::ptr::null_mut()))?;

    // ffi.isNull(ptr) -> boolean
    exports.set(
        "isNull",
        lua.create_function(|_, ptr: LuaValue| match ptr {
            LuaValue::LightUserData(lud) => Ok(lud.0.is_null()),
            LuaValue::UserData(ud) => {
                if let Ok(raw) = ud.borrow::<RawPointer>() {
                    Ok(raw.is_null())
                } else if let Ok(typed) = ud.borrow::<TypedPointer>() {
                    Ok(typed.addr.is_null())
                } else {
                    Ok(false)
                }
            }
            LuaValue::Nil => Ok(true),
            _ => Ok(false),
        })?,
    )?;

    // ========================================================================
    // Type Information
    // ========================================================================

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

    // ========================================================================
    // Callbacks
    // ========================================================================

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

/// Helper to extract raw pointer from various userdata types
fn get_raw_ptr(ud: &LuaAnyUserData) -> LuaResult<*mut c_void> {
    if let Ok(raw) = ud.borrow::<RawPointer>() {
        return Ok(raw.addr);
    }
    if let Ok(typed) = ud.borrow::<TypedPointer>() {
        return Ok(typed.addr);
    }
    if let Ok(buf) = ud.borrow::<Buffer>() {
        return Ok(buf.as_ptr().cast());
    }
    Err(LuaError::external(
        "Expected RawPointer, TypedPointer, or Buffer",
    ))
}
