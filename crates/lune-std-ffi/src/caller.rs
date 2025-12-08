//! Dynamic function caller using libffi for arbitrary function signatures.

use libffi::middle::{Arg, Builder, CodePtr, Type as FfiType};
use mlua::prelude::*;
use std::ffi::{CStr, CString, c_void};

use crate::types::{Buffer, CType};

/// Convert `CType` to libffi Type
fn ctype_to_ffi(ctype: CType) -> FfiType {
    match ctype {
        CType::Void => FfiType::void(),
        CType::Bool | CType::I8 => FfiType::i8(),
        CType::U8 => FfiType::u8(),
        CType::I16 => FfiType::i16(),
        CType::U16 => FfiType::u16(),
        CType::I32 => FfiType::i32(),
        CType::U32 => FfiType::u32(),
        CType::I64 => FfiType::i64(),
        CType::U64 => FfiType::u64(),
        CType::F32 => FfiType::f32(),
        CType::F64 => FfiType::f64(),
        CType::Pointer | CType::CString => FfiType::pointer(),
    }
}

/// Storage for argument values during a call
#[allow(dead_code)]
enum ArgValue {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Pointer(*mut c_void),
    CStringVal(CString),
}

impl ArgValue {
    fn as_arg(&self) -> Arg {
        match self {
            Self::Bool(v) => Arg::new(v),
            Self::I8(v) => Arg::new(v),
            Self::U8(v) => Arg::new(v),
            Self::I16(v) => Arg::new(v),
            Self::U16(v) => Arg::new(v),
            Self::I32(v) => Arg::new(v),
            Self::U32(v) => Arg::new(v),
            Self::I64(v) => Arg::new(v),
            Self::U64(v) => Arg::new(v),
            Self::F32(v) => Arg::new(v),
            Self::F64(v) => Arg::new(v),
            Self::Pointer(v) => Arg::new(v),
            Self::CStringVal(v) => Arg::new(&v.as_ptr()),
        }
    }
}

/// Convert a Lua value to `ArgValue` based on `CType`
fn lua_to_arg(lua: &Lua, value: LuaValue, ctype: CType) -> LuaResult<ArgValue> {
    Ok(match ctype {
        CType::Void => return Err(LuaError::external("Cannot pass void as argument")),
        CType::Bool => ArgValue::Bool(FromLua::from_lua(value, lua)?),
        CType::I8 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            ArgValue::I8(v as i8)
        }
        CType::U8 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            ArgValue::U8(v as u8)
        }
        CType::I16 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            ArgValue::I16(v as i16)
        }
        CType::U16 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            ArgValue::U16(v as u16)
        }
        CType::I32 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            ArgValue::I32(v as i32)
        }
        CType::U32 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            ArgValue::U32(v as u32)
        }
        CType::I64 => {
            let v: i64 = FromLua::from_lua(value, lua)?;
            ArgValue::I64(v)
        }
        CType::U64 => {
            let v: f64 = FromLua::from_lua(value, lua)?;
            ArgValue::U64(v as u64)
        }
        CType::F32 => {
            let v: f64 = FromLua::from_lua(value, lua)?;
            ArgValue::F32(v as f32)
        }
        CType::F64 => {
            let v: f64 = FromLua::from_lua(value, lua)?;
            ArgValue::F64(v)
        }
        CType::Pointer => match value {
            LuaValue::Nil => ArgValue::Pointer(std::ptr::null_mut()),
            LuaValue::LightUserData(ud) => ArgValue::Pointer(ud.0),
            LuaValue::UserData(ud) => {
                if let Ok(buf) = ud.borrow::<Buffer>() {
                    ArgValue::Pointer(buf.as_ptr().cast::<c_void>())
                } else {
                    return Err(LuaError::external("Expected pointer, buffer, or nil"));
                }
            }
            LuaValue::Integer(i) => ArgValue::Pointer(i as usize as *mut c_void),
            LuaValue::Number(n) => ArgValue::Pointer(n as usize as *mut c_void),
            _ => return Err(LuaError::external("Expected pointer, buffer, or nil")),
        },
        CType::CString => {
            let s: mlua::String = FromLua::from_lua(value, lua)?;
            let borrowed = s.as_bytes();
            let bytes: Vec<u8> = borrowed.to_vec();
            let cstr =
                CString::new(bytes).map_err(|_| LuaError::external("String contains null byte"))?;
            ArgValue::CStringVal(cstr)
        }
    })
}

/// Convert a return value based on `CType`
fn call_and_convert(
    lua: &Lua,
    cif: &libffi::middle::Cif,
    code_ptr: CodePtr,
    args: &[Arg],
    ret_type: CType,
) -> LuaResult<LuaValue> {
    Ok(match ret_type {
        CType::Void => {
            unsafe { cif.call::<()>(code_ptr, args) };
            LuaValue::Nil
        }
        CType::Bool => {
            let result: i8 = unsafe { cif.call(code_ptr, args) };
            LuaValue::Boolean(result != 0)
        }
        CType::I8 => {
            let result: i8 = unsafe { cif.call(code_ptr, args) };
            i64::from(result).into_lua(lua)?
        }
        CType::U8 => {
            let result: u8 = unsafe { cif.call(code_ptr, args) };
            i64::from(result).into_lua(lua)?
        }
        CType::I16 => {
            let result: i16 = unsafe { cif.call(code_ptr, args) };
            i64::from(result).into_lua(lua)?
        }
        CType::U16 => {
            let result: u16 = unsafe { cif.call(code_ptr, args) };
            i64::from(result).into_lua(lua)?
        }
        CType::I32 => {
            let result: i32 = unsafe { cif.call(code_ptr, args) };
            i64::from(result).into_lua(lua)?
        }
        CType::U32 => {
            let result: u32 = unsafe { cif.call(code_ptr, args) };
            i64::from(result).into_lua(lua)?
        }
        CType::I64 => {
            let result: i64 = unsafe { cif.call(code_ptr, args) };
            result.into_lua(lua)?
        }
        CType::U64 => {
            let result: u64 = unsafe { cif.call(code_ptr, args) };
            (result as f64).into_lua(lua)?
        }
        CType::F32 => {
            let result: f32 = unsafe { cif.call(code_ptr, args) };
            f64::from(result).into_lua(lua)?
        }
        CType::F64 => {
            let result: f64 = unsafe { cif.call(code_ptr, args) };
            result.into_lua(lua)?
        }
        CType::Pointer => {
            let result: *mut c_void = unsafe { cif.call(code_ptr, args) };
            if result.is_null() {
                LuaValue::Nil
            } else {
                LuaValue::LightUserData(LuaLightUserData(result))
            }
        }
        CType::CString => {
            let result: *const i8 = unsafe { cif.call(code_ptr, args) };
            if result.is_null() {
                LuaValue::Nil
            } else {
                let cstr = unsafe { CStr::from_ptr(result) };
                LuaValue::String(lua.create_string(cstr.to_bytes())?)
            }
        }
    })
}

/// Perform a dynamic function call
pub fn dynamic_call(
    lua: &Lua,
    fn_ptr: *const c_void,
    ret_type: CType,
    arg_types: &[CType],
    args: Vec<LuaValue>,
) -> LuaResult<LuaValue> {
    if args.len() != arg_types.len() {
        return Err(LuaError::external(format!(
            "Expected {} arguments, got {}",
            arg_types.len(),
            args.len()
        )));
    }

    // Convert types
    let ffi_arg_types: Vec<FfiType> = arg_types.iter().map(|t| ctype_to_ffi(*t)).collect();
    let ffi_ret_type = ctype_to_ffi(ret_type);

    // Build CIF
    let cif = Builder::new()
        .args(ffi_arg_types)
        .res(ffi_ret_type)
        .into_cif();

    // Convert arguments
    let arg_values: Vec<ArgValue> = args
        .into_iter()
        .zip(arg_types.iter())
        .map(|(v, t)| lua_to_arg(lua, v, *t))
        .collect::<LuaResult<Vec<_>>>()?;

    let ffi_args: Vec<Arg> = arg_values.iter().map(ArgValue::as_arg).collect();

    // Call
    let code_ptr = CodePtr::from_ptr(fn_ptr);
    call_and_convert(lua, &cif, code_ptr, &ffi_args, ret_type)
}
