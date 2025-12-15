//! FFI Pointer types for zero-copy memory access.
//!
//! Implements two distinct pointer types following C semantics:
//! - `RawPointer` (void*): Byte-level arithmetic, no indexing
//! - `TypedPointer` (T*): Stride-based arithmetic, array indexing

use mlua::prelude::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::types::CType;

/// Unique ID generator for arena tracking
static ARENA_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Raw pointer (void*) - byte-level arithmetic only
#[derive(Debug, Clone, Copy)]
pub struct RawPointer {
    pub addr: *mut c_void,
    /// Optional arena ID for bounds checking (0 = unmanaged)
    pub arena_id: usize,
    /// Size hint for bounds checking (0 = unknown)
    pub size_hint: usize,
}

impl RawPointer {
    /// Create a new raw pointer from an address
    #[must_use]
    pub fn new(addr: *mut c_void) -> Self {
        Self {
            addr,
            arena_id: 0,
            size_hint: 0,
        }
    }

    /// Create a managed pointer with bounds info
    #[must_use]
    pub fn managed(addr: *mut c_void, arena_id: usize, size: usize) -> Self {
        Self {
            addr,
            arena_id,
            size_hint: size,
        }
    }

    /// Check if pointer is null
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.addr.is_null()
    }

    /// Get address as usize
    #[must_use]
    pub fn as_usize(&self) -> usize {
        self.addr as usize
    }

    /// Offset by bytes (void* arithmetic)
    #[must_use]
    pub fn offset_bytes(&self, offset: isize) -> Self {
        Self {
            addr: unsafe { self.addr.cast::<u8>().offset(offset).cast() },
            arena_id: self.arena_id,
            size_hint: if self.size_hint > 0 && offset >= 0 {
                self.size_hint.saturating_sub(offset as usize)
            } else {
                0
            },
        }
    }

    /// Read a value at offset with type
    pub fn read(&self, lua: &Lua, offset: usize, ctype: CType) -> LuaResult<LuaValue> {
        if self.addr.is_null() {
            return Err(LuaError::external("Cannot read from null pointer"));
        }

        // Bounds check for managed pointers
        if self.size_hint > 0 && offset + ctype.size() > self.size_hint {
            return Err(LuaError::external(format!(
                "Read out of bounds: offset {} + size {} > {}",
                offset,
                ctype.size(),
                self.size_hint
            )));
        }

        let ptr = unsafe { self.addr.cast::<u8>().add(offset) };
        read_value_at(lua, ptr, ctype)
    }

    /// Write a value at offset with type
    pub fn write(&self, lua: &Lua, offset: usize, ctype: CType, value: LuaValue) -> LuaResult<()> {
        if self.addr.is_null() {
            return Err(LuaError::external("Cannot write to null pointer"));
        }

        // Bounds check for managed pointers
        if self.size_hint > 0 && offset + ctype.size() > self.size_hint {
            return Err(LuaError::external(format!(
                "Write out of bounds: offset {} + size {} > {}",
                offset,
                ctype.size(),
                self.size_hint
            )));
        }

        let ptr = unsafe { self.addr.cast::<u8>().add(offset) };
        write_value_at(lua, ptr, ctype, value)
    }
}

impl LuaUserData for RawPointer {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("addr", |_, this| Ok(this.as_usize()));
        fields.add_field_method_get("isNull", |_, this| Ok(this.is_null()));
        fields.add_field_method_get("isManaged", |_, this| Ok(this.arena_id != 0));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Byte-level arithmetic: ptr + n adds n bytes
        methods.add_meta_method(LuaMetaMethod::Add, |_, this, offset: isize| {
            Ok(this.offset_bytes(offset))
        });

        methods.add_meta_method(LuaMetaMethod::Sub, |_, this, offset: isize| {
            Ok(this.offset_bytes(-offset))
        });

        // Equality check via AnyUserData
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaAnyUserData| {
            if let Ok(other_ptr) = other.borrow::<RawPointer>() {
                Ok(this.addr == other_ptr.addr)
            } else {
                Ok(false)
            }
        });

        // ToString
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("RawPointer(0x{:x})", this.as_usize()))
        });

        // Read at offset
        methods.add_method("read", |lua, this, (offset, ctype): (usize, CType)| {
            this.read(lua, offset, ctype)
        });

        // Write at offset
        methods.add_method(
            "write",
            |lua, this, (offset, ctype, value): (usize, CType, LuaValue)| {
                this.write(lua, offset, ctype, value)
            },
        );

        // Offset method (alternative to + operator)
        methods.add_method("offset", |_, this, bytes: isize| {
            Ok(this.offset_bytes(bytes))
        });

        // Get raw lightuserdata for C calls
        methods.add_method("toLightUserData", |_, this, ()| {
            Ok(LuaLightUserData(this.addr))
        });
    }
}

// ============================================================================
// TypedPointer - Stride-based arithmetic with array indexing
// ============================================================================

/// Typed pointer (T*) - stride-based arithmetic and indexing
#[derive(Debug, Clone)]
pub struct TypedPointer {
    pub addr: *mut c_void,
    pub ctype: CType,
    pub stride: usize,
    /// Optional arena ID for safety
    pub arena_id: usize,
    /// Size hint in elements (0 = unknown)
    pub element_count: usize,
}

impl TypedPointer {
    /// Create a typed pointer from raw pointer and type
    #[must_use]
    pub fn new(raw: &RawPointer, ctype: CType) -> Self {
        let stride = ctype.size();
        let element_count = if raw.size_hint > 0 && stride > 0 {
            raw.size_hint / stride
        } else {
            0
        };

        Self {
            addr: raw.addr,
            ctype,
            stride,
            arena_id: raw.arena_id,
            element_count,
        }
    }

    /// Create from address and type
    #[must_use]
    pub fn from_addr(addr: *mut c_void, ctype: CType) -> Self {
        Self {
            addr,
            ctype,
            stride: ctype.size(),
            arena_id: 0,
            element_count: 0,
        }
    }

    /// Get address as usize
    #[must_use]
    pub fn as_usize(&self) -> usize {
        self.addr as usize
    }

    /// Offset by elements (T* arithmetic)
    #[must_use]
    pub fn offset_elements(&self, count: isize) -> Self {
        let byte_offset = count * self.stride as isize;
        Self {
            addr: unsafe { self.addr.cast::<u8>().offset(byte_offset).cast() },
            ctype: self.ctype,
            stride: self.stride,
            arena_id: self.arena_id,
            element_count: if self.element_count > 0 && count >= 0 {
                self.element_count.saturating_sub(count as usize)
            } else {
                0
            },
        }
    }

    /// Read value at index
    pub fn read_at(&self, lua: &Lua, index: usize) -> LuaResult<LuaValue> {
        if self.addr.is_null() {
            return Err(LuaError::external("Cannot read from null pointer"));
        }

        // Bounds check
        if self.element_count > 0 && index >= self.element_count {
            return Err(LuaError::external(format!(
                "Index {} out of bounds (count: {})",
                index, self.element_count
            )));
        }

        let offset = index * self.stride;
        let ptr = unsafe { self.addr.cast::<u8>().add(offset) };
        read_value_at(lua, ptr, self.ctype)
    }

    /// Write value at index
    pub fn write_at(&self, lua: &Lua, index: usize, value: LuaValue) -> LuaResult<()> {
        if self.addr.is_null() {
            return Err(LuaError::external("Cannot write to null pointer"));
        }

        // Bounds check
        if self.element_count > 0 && index >= self.element_count {
            return Err(LuaError::external(format!(
                "Index {} out of bounds (count: {})",
                index, self.element_count
            )));
        }

        let offset = index * self.stride;
        let ptr = unsafe { self.addr.cast::<u8>().add(offset) };
        write_value_at(lua, ptr, self.ctype, value)
    }
}

impl LuaUserData for TypedPointer {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("addr", |_, this| Ok(this.as_usize()));
        fields.add_field_method_get("stride", |_, this| Ok(this.stride));
        fields.add_field_method_get("isNull", |_, this| Ok(this.addr.is_null()));
        fields.add_field_method_get("count", |_, this| {
            if this.element_count > 0 {
                Ok(Some(this.element_count))
            } else {
                Ok(None)
            }
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Stride-based arithmetic: ptr + n adds n * stride bytes
        methods.add_meta_method(LuaMetaMethod::Add, |_, this, offset: isize| {
            Ok(this.offset_elements(offset))
        });

        methods.add_meta_method(LuaMetaMethod::Sub, |_, this, offset: isize| {
            Ok(this.offset_elements(-offset))
        });

        // Array indexing: ptr[i] reads at index
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            match key {
                LuaValue::Integer(i) => this.read_at(lua, i as usize),
                LuaValue::Number(n) => this.read_at(lua, n as usize),
                LuaValue::String(s) => {
                    // Handle named field access
                    let name = s.to_str()?;
                    let name_str: &str = &name;
                    match name_str {
                        "addr" => Ok(LuaValue::Integer(this.as_usize() as i64)),
                        "stride" => Ok(LuaValue::Integer(this.stride as i64)),
                        "isNull" => Ok(LuaValue::Boolean(this.addr.is_null())),
                        _ => Err(LuaError::external(format!("Unknown field: {}", name_str))),
                    }
                }
                _ => Err(LuaError::external("Index must be a number")),
            }
        });

        // Array assignment: ptr[i] = val writes at index
        methods.add_meta_method(
            LuaMetaMethod::NewIndex,
            |lua, this, (index, value): (usize, LuaValue)| this.write_at(lua, index, value),
        );

        // Equality via AnyUserData
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaAnyUserData| {
            if let Ok(other_ptr) = other.borrow::<TypedPointer>() {
                Ok(this.addr == other_ptr.addr && this.ctype == other_ptr.ctype)
            } else {
                Ok(false)
            }
        });

        // ToString
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "TypedPointer<{:?}>(0x{:x}, stride={})",
                this.ctype,
                this.as_usize(),
                this.stride
            ))
        });

        // Explicit read/write methods
        methods.add_method("get", |lua, this, index: usize| this.read_at(lua, index));

        methods.add_method("set", |lua, this, (index, value): (usize, LuaValue)| {
            this.write_at(lua, index, value)
        });

        // Convert back to raw
        methods.add_method("toRaw", |_, this, ()| {
            Ok(RawPointer {
                addr: this.addr,
                arena_id: this.arena_id,
                size_hint: this.element_count * this.stride,
            })
        });

        // Get raw lightuserdata
        methods.add_method("toLightUserData", |_, this, ()| {
            Ok(LuaLightUserData(this.addr))
        });
    }
}

// ============================================================================
// Helper functions for reading/writing values
// ============================================================================

/// Read a C value from memory
pub fn read_value_at(lua: &Lua, ptr: *mut u8, ctype: CType) -> LuaResult<LuaValue> {
    Ok(match ctype {
        CType::Void => LuaValue::Nil,
        CType::Bool => LuaValue::Boolean(unsafe { *(ptr as *const bool) }),
        CType::I8 => {
            let v = unsafe { *(ptr as *const i8) };
            LuaValue::Integer(i64::from(v))
        }
        CType::U8 => {
            let v = unsafe { *ptr };
            LuaValue::Integer(i64::from(v))
        }
        CType::I16 => {
            let v = unsafe { *(ptr as *const i16) };
            LuaValue::Integer(i64::from(v))
        }
        CType::U16 => {
            let v = unsafe { *(ptr as *const u16) };
            LuaValue::Integer(i64::from(v))
        }
        CType::I32 => {
            let v = unsafe { *(ptr as *const i32) };
            LuaValue::Integer(i64::from(v))
        }
        CType::U32 => {
            let v = unsafe { *(ptr as *const u32) };
            LuaValue::Integer(i64::from(v))
        }
        CType::I64 => {
            let v = unsafe { *(ptr as *const i64) };
            LuaValue::Integer(v)
        }
        CType::U64 => {
            let v = unsafe { *(ptr as *const u64) };
            LuaValue::Number(v as f64)
        }
        CType::ISize => {
            let v = unsafe { *(ptr as *const isize) };
            LuaValue::Integer(v as i64)
        }
        CType::USize => {
            let v = unsafe { *(ptr as *const usize) };
            LuaValue::Integer(v as i64)
        }
        CType::F32 => {
            let v = unsafe { *(ptr as *const f32) };
            LuaValue::Number(f64::from(v))
        }
        CType::F64 => {
            let v = unsafe { *(ptr as *const f64) };
            LuaValue::Number(v)
        }
        CType::Pointer => {
            let p = unsafe { *(ptr as *const *mut c_void) };
            if p.is_null() {
                LuaValue::Nil
            } else {
                LuaValue::LightUserData(LuaLightUserData(p))
            }
        }
        CType::CString => {
            let cptr = unsafe { *(ptr as *const *const i8) };
            if cptr.is_null() {
                LuaValue::Nil
            } else {
                let cstr = unsafe { std::ffi::CStr::from_ptr(cptr) };
                LuaValue::String(lua.create_string(cstr.to_bytes())?)
            }
        }
    })
}

/// Write a C value to memory
pub fn write_value_at(lua: &Lua, ptr: *mut u8, ctype: CType, value: LuaValue) -> LuaResult<()> {
    match ctype {
        CType::Void => {}
        CType::Bool => {
            let v: bool = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut bool) = v };
        }
        CType::I8 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut i8) = v as i8 };
        }
        CType::U8 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *ptr = v as u8 };
        }
        CType::I16 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut i16) = v as i16 };
        }
        CType::U16 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut u16) = v as u16 };
        }
        CType::I32 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut i32) = v as i32 };
        }
        CType::U32 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut u32) = v as u32 };
        }
        CType::I64 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut i64) = v };
        }
        CType::U64 => {
            let v: f64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut u64) = v as u64 };
        }
        CType::ISize => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut isize) = v as isize };
        }
        CType::USize => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut usize) = v as usize };
        }
        CType::F32 => {
            let v: f64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut f32) = v as f32 };
        }
        CType::F64 => {
            let v: f64 = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut f64) = v };
        }
        CType::Pointer => {
            let v: LuaLightUserData = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut *mut c_void) = v.0 };
        }
        CType::CString => {
            let v: LuaLightUserData = FromLua::from_lua(value, lua)?;
            unsafe { *(ptr as *mut *mut c_void) = v.0 };
        }
    }
    Ok(())
}

/// Generate a unique arena ID
pub fn next_arena_id() -> usize {
    ARENA_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}
