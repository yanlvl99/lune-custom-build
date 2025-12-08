//! FFI type descriptors for dynamic function calls.

use mlua::prelude::*;
use std::alloc::{Layout, alloc, dealloc};
use std::ffi::c_void;
use std::ptr;

/// Represents a C type for FFI calls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CType {
    Void,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Pointer,
    CString,
}

impl CType {
    /// Parse a C type from a string
    #[allow(clippy::should_implement_trait)]
    #[must_use] 
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "void" => Some(Self::Void),
            "bool" => Some(Self::Bool),
            "i8" | "int8" | "char" => Some(Self::I8),
            "u8" | "uint8" | "uchar" | "byte" => Some(Self::U8),
            "i16" | "int16" | "short" => Some(Self::I16),
            "u16" | "uint16" | "ushort" => Some(Self::U16),
            "i32" | "int32" | "int" => Some(Self::I32),
            "u32" | "uint32" | "uint" => Some(Self::U32),
            "i64" | "int64" | "long" | "longlong" => Some(Self::I64),
            "u64" | "uint64" | "ulong" | "ulonglong" | "size_t" => Some(Self::U64),
            "f32" | "float" => Some(Self::F32),
            "f64" | "double" => Some(Self::F64),
            "ptr" | "pointer" | "void*" => Some(Self::Pointer),
            "string" | "cstring" | "char*" => Some(Self::CString),
            _ => None,
        }
    }

    #[must_use] 
    pub fn size(&self) -> usize {
        match self {
            Self::Void => 0,
            Self::Bool | Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::I64 | Self::U64 | Self::F64 | Self::Pointer | Self::CString => 8,
        }
    }

    #[must_use] 
    pub fn alignment(&self) -> usize {
        self.size().max(1)
    }
}

impl FromLua for CType {
    fn from_lua(value: LuaValue, _: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::String(s) => {
                let borrowed = s.to_str()?;
                let s: &str = &borrowed;
                Self::from_str(s)
                    .ok_or_else(|| LuaError::external(format!("Unknown C type: '{s}'")))
            }
            _ => Err(LuaError::external("Expected string for CType")),
        }
    }
}

impl IntoLua for CType {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let name = match self {
            Self::Void => "void",
            Self::Bool => "bool",
            Self::I8 => "i8",
            Self::U8 => "u8",
            Self::I16 => "i16",
            Self::U16 => "u16",
            Self::I32 => "i32",
            Self::U32 => "u32",
            Self::I64 => "i64",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Pointer => "pointer",
            Self::CString => "string",
        };
        Ok(LuaValue::String(lua.create_string(name)?))
    }
}

/// A raw memory buffer for FFI operations
pub struct Buffer {
    ptr: *mut u8,
    size: usize,
    owned: bool,
}

impl Buffer {
    /// Allocate a new buffer of the given size
    #[must_use] 
    pub fn new(size: usize) -> Self {
        let layout = Layout::from_size_align(size.max(1), 8).unwrap();
        let ptr = unsafe { alloc(layout) };
        unsafe { ptr::write_bytes(ptr, 0, size) };
        Self {
            ptr,
            size,
            owned: true,
        }
    }

    /// Create a buffer from an existing pointer (not owned)
    pub fn from_ptr(ptr: *mut u8, size: usize) -> Self {
        Self {
            ptr,
            size,
            owned: false,
        }
    }

    /// Get a pointer to the buffer
    #[must_use] 
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }

    /// Read a value of the given type at offset
    pub fn read(&self, lua: &Lua, offset: usize, ctype: CType) -> LuaResult<LuaValue> {
        if offset + ctype.size() > self.size {
            return Err(LuaError::external("Buffer read out of bounds"));
        }

        let ptr = unsafe { self.ptr.add(offset) };

        Ok(match ctype {
            CType::Void => LuaValue::Nil,
            CType::Bool => LuaValue::Boolean(unsafe { *(ptr as *const bool) }),
            CType::I8 => {
                let v = unsafe { *(ptr as *const i8) };
                i64::from(v).into_lua(lua)?
            }
            CType::U8 => {
                let v = unsafe { *ptr.cast_const() };
                i64::from(v).into_lua(lua)?
            }
            CType::I16 => {
                let v = unsafe { *(ptr as *const i16) };
                i64::from(v).into_lua(lua)?
            }
            CType::U16 => {
                let v = unsafe { *(ptr as *const u16) };
                i64::from(v).into_lua(lua)?
            }
            CType::I32 => {
                let v = unsafe { *(ptr as *const i32) };
                i64::from(v).into_lua(lua)?
            }
            CType::U32 => {
                let v = unsafe { *(ptr as *const u32) };
                i64::from(v).into_lua(lua)?
            }
            CType::I64 => {
                let v = unsafe { *(ptr as *const i64) };
                v.into_lua(lua)?
            }
            CType::U64 => {
                let v = unsafe { *(ptr as *const u64) };
                (v as f64).into_lua(lua)?
            }
            CType::F32 => {
                let v = unsafe { *(ptr as *const f32) };
                f64::from(v).into_lua(lua)?
            }
            CType::F64 => {
                let v = unsafe { *(ptr as *const f64) };
                v.into_lua(lua)?
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

    /// Write a value of the given type at offset
    pub fn write(
        &mut self,
        lua: &Lua,
        offset: usize,
        ctype: CType,
        value: LuaValue,
    ) -> LuaResult<()> {
        if offset + ctype.size() > self.size {
            return Err(LuaError::external("Buffer write out of bounds"));
        }

        let ptr = unsafe { self.ptr.add(offset) };

        match ctype {
            CType::Void => {}
            CType::Bool => {
                let v: bool = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<bool>() = v };
            }
            CType::I8 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<i8>() = v as i8 };
            }
            CType::U8 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<u8>() = v as u8 };
            }
            CType::I16 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<i16>() = v as i16 };
            }
            CType::U16 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<u16>() = v as u16 };
            }
            CType::I32 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<i32>() = v as i32 };
            }
            CType::U32 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<u32>() = v as u32 };
            }
            CType::I64 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<i64>() = v };
            }
            CType::U64 => {
                let v: f64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<u64>() = v as u64 };
            }
            CType::F32 => {
                let v: f64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<f32>() = v as f32 };
            }
            CType::F64 => {
                let v: f64 = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<f64>() = v };
            }
            CType::Pointer => {
                let v: LuaLightUserData = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<*mut c_void>() = v.0 };
            }
            CType::CString => {
                let v: LuaLightUserData = FromLua::from_lua(value, lua)?;
                unsafe { *ptr.cast::<*mut c_void>() = v.0 };
            }
        }
        Ok(())
    }

    /// Fill the buffer with zeros
    pub fn zero(&mut self) {
        unsafe { ptr::write_bytes(self.ptr, 0, self.size) };
    }

    /// Copy bytes into the buffer
    pub fn write_bytes(&mut self, offset: usize, bytes: &[u8]) -> LuaResult<()> {
        if offset + bytes.len() > self.size {
            return Err(LuaError::external("Buffer write out of bounds"));
        }
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.add(offset), bytes.len());
        }
        Ok(())
    }

    /// Read bytes from the buffer
    pub fn read_bytes(&self, offset: usize, len: usize) -> LuaResult<Vec<u8>> {
        if offset + len > self.size {
            return Err(LuaError::external("Buffer read out of bounds"));
        }
        let mut bytes = vec![0u8; len];
        unsafe {
            ptr::copy_nonoverlapping(self.ptr.add(offset), bytes.as_mut_ptr(), len);
        }
        Ok(bytes)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.owned && !self.ptr.is_null() {
            let layout = Layout::from_size_align(self.size.max(1), 8).unwrap();
            unsafe { dealloc(self.ptr, layout) };
        }
    }
}

impl LuaUserData for Buffer {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("size", |_, this| Ok(this.size));
        fields.add_field_method_get("ptr", |_, this| {
            Ok(LuaLightUserData(this.ptr.cast::<c_void>()))
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("read", |lua, this, (offset, ctype): (usize, CType)| {
            this.read(lua, offset, ctype)
        });

        methods.add_method_mut(
            "write",
            |lua, this, (offset, ctype, value): (usize, CType, LuaValue)| {
                this.write(lua, offset, ctype, value)
            },
        );

        methods.add_method_mut("zero", |_, this, ()| {
            this.zero();
            Ok(())
        });

        methods.add_method_mut(
            "writeBytes",
            |_, this, (offset, bytes): (usize, mlua::String)| {
                let borrowed = bytes.as_bytes();
                let slice: &[u8] = &borrowed;
                this.write_bytes(offset, slice)
            },
        );

        methods.add_method("readBytes", |lua, this, (offset, len): (usize, usize)| {
            let bytes = this.read_bytes(offset, len)?;
            lua.create_string(&bytes)
        });

        methods.add_method("readString", |lua, this, offset: Option<usize>| {
            let offset = offset.unwrap_or(0);
            let mut len = 0;
            while offset + len < this.size {
                if unsafe { *this.ptr.add(offset + len) } == 0 {
                    break;
                }
                len += 1;
            }
            let bytes = this.read_bytes(offset, len)?;
            let s = String::from_utf8_lossy(&bytes);
            lua.create_string(s.as_ref())
        });

        methods.add_method_mut(
            "writeString",
            |_, this, (offset, s): (usize, mlua::String)| {
                let borrowed = s.as_bytes();
                let bytes: &[u8] = &borrowed;
                this.write_bytes(offset, bytes)?;
                if offset + bytes.len() < this.size {
                    unsafe { *this.ptr.add(offset + bytes.len()) = 0 };
                }
                Ok(())
            },
        );

        methods.add_method("slice", |_, this, (offset, size): (usize, usize)| {
            if offset + size > this.size {
                return Err(LuaError::external("Slice out of bounds"));
            }
            Ok(Buffer::from_ptr(unsafe { this.ptr.add(offset) }, size))
        });
    }
}

/// Create the types submodule
pub fn create_types_table(lua: &Lua) -> LuaResult<LuaTable> {
    let types = lua.create_table()?;

    types.set("void", "void")?;
    types.set("bool", "bool")?;
    types.set("i8", "i8")?;
    types.set("u8", "u8")?;
    types.set("i16", "i16")?;
    types.set("u16", "u16")?;
    types.set("i32", "i32")?;
    types.set("u32", "u32")?;
    types.set("i64", "i64")?;
    types.set("u64", "u64")?;
    types.set("f32", "f32")?;
    types.set("f64", "f64")?;
    types.set("pointer", "pointer")?;
    types.set("string", "string")?;

    types.set("int", "i32")?;
    types.set("uint", "u32")?;
    types.set("long", "i64")?;
    types.set("ulong", "u64")?;
    types.set("float", "f32")?;
    types.set("double", "f64")?;
    types.set("char", "i8")?;
    types.set("uchar", "u8")?;
    types.set("short", "i16")?;
    types.set("ushort", "u16")?;
    types.set("size_t", "u64")?;
    types.set("ptr", "pointer")?;

    types.set(
        "sizeof",
        lua.create_function(|_, ctype: CType| Ok(ctype.size()))?,
    )?;

    types.set(
        "alignof",
        lua.create_function(|_, ctype: CType| Ok(ctype.alignment()))?,
    )?;

    Ok(types)
}
