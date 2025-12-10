//! FFI Callback support using libffi closures.
//!
//! Creates C-callable function pointers from Lua functions.

#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]

use std::ffi::c_void;
use std::ptr::{self, addr_of_mut};

use libffi::low::{
    CodePtr, closure_alloc, closure_free, ffi_cif, ffi_closure, ffi_type, prep_cif,
    prep_closure_mut,
};
use libffi::raw::ffi_abi_FFI_DEFAULT_ABI;
use mlua::prelude::*;

use crate::types::CType;

/// Convert CType to libffi ffi_type pointer
fn ctype_to_ffi_type(ctype: CType) -> *mut ffi_type {
    match ctype {
        CType::Void => addr_of_mut!(libffi::low::types::void),
        CType::Bool | CType::I8 => addr_of_mut!(libffi::low::types::sint8),
        CType::U8 => addr_of_mut!(libffi::low::types::uint8),
        CType::I16 => addr_of_mut!(libffi::low::types::sint16),
        CType::U16 => addr_of_mut!(libffi::low::types::uint16),
        CType::I32 => addr_of_mut!(libffi::low::types::sint32),
        CType::U32 => addr_of_mut!(libffi::low::types::uint32),
        CType::I64 => addr_of_mut!(libffi::low::types::sint64),
        CType::U64 => addr_of_mut!(libffi::low::types::uint64),
        // Platform-specific types for ARM compatibility
        #[cfg(target_pointer_width = "64")]
        CType::ISize => addr_of_mut!(libffi::low::types::sint64),
        #[cfg(target_pointer_width = "64")]
        CType::USize => addr_of_mut!(libffi::low::types::uint64),
        #[cfg(target_pointer_width = "32")]
        CType::ISize => addr_of_mut!(libffi::low::types::sint32),
        #[cfg(target_pointer_width = "32")]
        CType::USize => addr_of_mut!(libffi::low::types::uint32),
        CType::F32 => addr_of_mut!(libffi::low::types::float),
        CType::F64 => addr_of_mut!(libffi::low::types::double),
        CType::Pointer | CType::CString => addr_of_mut!(libffi::low::types::pointer),
    }
}

/// Userdata stored with each callback
struct CallbackData {
    func_key: LuaRegistryKey,
    lua_ptr: *const Lua,
    arg_types: Vec<CType>,
    ret_type: CType,
}

/// The callback trampoline - signature must match libffi's expectation
unsafe extern "C" fn callback_trampoline(
    _cif: &ffi_cif,
    result: &mut c_void,
    args: *const *const c_void,
    userdata: &mut c_void,
) {
    // All operations here are inside an unsafe block since this is an unsafe fn
    unsafe {
        let data = &*(userdata as *const c_void as *const CallbackData);
        let lua = &*data.lua_ptr;

        // Get Lua function
        let func: LuaFunction = match lua.registry_value(&data.func_key) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[FFI CALLBACK ERROR] Failed to get Lua function: {}", e);
                return;
            }
        };

        // Convert C args to Lua values
        let mut lua_args = Vec::with_capacity(data.arg_types.len());
        for (i, arg_type) in data.arg_types.iter().enumerate() {
            let arg_ptr = *args.add(i);
            let lua_val = match arg_type {
                CType::Void => LuaValue::Nil,
                CType::Bool => LuaValue::Boolean(*(arg_ptr as *const i8) != 0),
                CType::I8 => LuaValue::Integer(i64::from(*(arg_ptr as *const i8))),
                CType::U8 => LuaValue::Integer(i64::from(*(arg_ptr as *const u8))),
                CType::I16 => LuaValue::Integer(i64::from(*(arg_ptr as *const i16))),
                CType::U16 => LuaValue::Integer(i64::from(*(arg_ptr as *const u16))),
                CType::I32 => LuaValue::Integer(i64::from(*(arg_ptr as *const i32))),
                CType::U32 => LuaValue::Integer(i64::from(*(arg_ptr as *const u32))),
                CType::I64 => LuaValue::Integer(*(arg_ptr as *const i64)),
                CType::U64 => LuaValue::Number(*(arg_ptr as *const u64) as f64),
                CType::ISize => LuaValue::Integer(*(arg_ptr as *const isize) as i64),
                CType::USize => LuaValue::Integer(*(arg_ptr as *const usize) as i64),
                CType::F32 => LuaValue::Number(f64::from(*(arg_ptr as *const f32))),
                CType::F64 => LuaValue::Number(*(arg_ptr as *const f64)),
                CType::Pointer => {
                    LuaValue::LightUserData(LuaLightUserData(*(arg_ptr as *const *mut c_void)))
                }
                CType::CString => {
                    let cptr = *(arg_ptr as *const *const i8);
                    if cptr.is_null() {
                        LuaValue::Nil
                    } else {
                        match std::ffi::CStr::from_ptr(cptr).to_str() {
                            Ok(s) => lua
                                .create_string(s)
                                .map(LuaValue::String)
                                .unwrap_or(LuaValue::Nil),
                            Err(_) => LuaValue::Nil,
                        }
                    }
                }
            };
            lua_args.push(lua_val);
        }

        // Call Lua function
        let call_result = func.call::<LuaMultiValue>(LuaMultiValue::from_iter(lua_args));

        // Convert result back to C
        let ret_ptr = result as *mut c_void;
        match call_result {
            Ok(values) => {
                let first = values.into_iter().next().unwrap_or(LuaValue::Nil);
                match data.ret_type {
                    CType::Void => {}
                    CType::Bool => {
                        *(ret_ptr as *mut i8) = i8::from(first.as_boolean().unwrap_or(false));
                    }
                    CType::I8 => {
                        *(ret_ptr as *mut i8) = first.as_integer().unwrap_or(0) as i8;
                    }
                    CType::U8 => {
                        *(ret_ptr as *mut u8) = first.as_integer().unwrap_or(0) as u8;
                    }
                    CType::I16 => {
                        *(ret_ptr as *mut i16) = first.as_integer().unwrap_or(0) as i16;
                    }
                    CType::U16 => {
                        *(ret_ptr as *mut u16) = first.as_integer().unwrap_or(0) as u16;
                    }
                    CType::I32 => {
                        *(ret_ptr as *mut i32) = first.as_integer().unwrap_or(0) as i32;
                    }
                    CType::U32 => {
                        *(ret_ptr as *mut u32) = first.as_integer().unwrap_or(0) as u32;
                    }
                    CType::I64 => {
                        *(ret_ptr as *mut i64) = first.as_integer().unwrap_or(0);
                    }
                    CType::U64 => {
                        *(ret_ptr as *mut u64) = first.as_number().unwrap_or(0.0) as u64;
                    }
                    CType::ISize => {
                        *(ret_ptr as *mut isize) = first.as_integer().unwrap_or(0) as isize;
                    }
                    CType::USize => {
                        *(ret_ptr as *mut usize) = first.as_integer().unwrap_or(0) as usize;
                    }
                    CType::F32 => {
                        *(ret_ptr as *mut f32) = first.as_number().unwrap_or(0.0) as f32;
                    }
                    CType::F64 => {
                        *(ret_ptr as *mut f64) = first.as_number().unwrap_or(0.0);
                    }
                    CType::Pointer => {
                        if let LuaValue::LightUserData(ud) = first {
                            *(ret_ptr as *mut *mut c_void) = ud.0;
                        } else {
                            *(ret_ptr as *mut *mut c_void) = ptr::null_mut();
                        }
                    }
                    CType::CString => {
                        *(ret_ptr as *mut *mut c_void) = ptr::null_mut();
                    }
                }
            }
            Err(e) => {
                eprintln!("[FFI CALLBACK ERROR] Lua function error: {}", e);
            }
        }
    }
}

/// A callback that can be passed to C functions.
pub struct FfiCallback {
    closure: *mut ffi_closure,
    code_ptr: CodePtr,
    _cif: Box<ffi_cif>,
    _arg_types_ffi: Vec<*mut ffi_type>,
    _data: Box<CallbackData>,
    ret_type: CType,
    arg_count: usize,
}

unsafe impl Send for FfiCallback {}
unsafe impl Sync for FfiCallback {}

impl FfiCallback {
    /// Create a new callback from a Lua function.
    pub fn new(
        lua: &Lua,
        func: LuaFunction,
        ret_type: CType,
        arg_types: Vec<CType>,
    ) -> LuaResult<Self> {
        if arg_types.len() > 16 {
            eprintln!("[FFI ERROR] Callbacks with more than 16 arguments not supported");
            return Err(LuaError::external("Callbacks with >16 args not supported"));
        }

        let func_key = lua.create_registry_value(func)?;

        let arg_types_ffi: Vec<*mut ffi_type> =
            arg_types.iter().map(|t| ctype_to_ffi_type(*t)).collect();

        let ret_type_ffi = ctype_to_ffi_type(ret_type);

        let mut cif = Box::new(unsafe { std::mem::zeroed::<ffi_cif>() });

        let status = unsafe {
            prep_cif(
                cif.as_mut(),
                ffi_abi_FFI_DEFAULT_ABI,
                arg_types_ffi.len(),
                ret_type_ffi,
                if arg_types_ffi.is_empty() {
                    ptr::null_mut()
                } else {
                    arg_types_ffi.as_ptr() as *mut _
                },
            )
        };

        if status.is_err() {
            eprintln!("[FFI ERROR] Failed to prepare CIF for callback");
            return Err(LuaError::external("Failed to prepare callback CIF"));
        }

        let (closure, code_ptr) = closure_alloc();

        if closure.is_null() {
            eprintln!("[FFI ERROR] Failed to allocate closure");
            return Err(LuaError::external("Failed to allocate closure"));
        }

        let data = Box::new(CallbackData {
            func_key,
            lua_ptr: lua as *const Lua,
            arg_types: arg_types.clone(),
            ret_type,
        });

        let arg_count = arg_types.len();

        let status = unsafe {
            prep_closure_mut(
                closure,
                cif.as_mut(),
                callback_trampoline,
                data.as_ref() as *const CallbackData as *mut c_void,
                code_ptr,
            )
        };

        if status.is_err() {
            unsafe { closure_free(closure) };
            eprintln!("[FFI ERROR] Failed to prepare closure");
            return Err(LuaError::external("Failed to prepare closure"));
        }

        Ok(Self {
            closure,
            code_ptr,
            _cif: cif,
            _arg_types_ffi: arg_types_ffi,
            _data: data,
            ret_type,
            arg_count,
        })
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.code_ptr.as_ptr() as *mut c_void
    }
}

impl Drop for FfiCallback {
    fn drop(&mut self) {
        if !self.closure.is_null() {
            unsafe { closure_free(self.closure) };
        }
    }
}

impl LuaUserData for FfiCallback {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("ptr", |_, this| Ok(LuaLightUserData(this.as_ptr())));
        fields.add_field_method_get("retType", |lua, this| this.ret_type.into_lua(lua));
        fields.add_field_method_get("argCount", |_, this| Ok(this.arg_count));
        fields.add_field_method_get("isValid", |_, this| Ok(!this.closure.is_null()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getPtr", |_, this, ()| Ok(LuaLightUserData(this.as_ptr())));
        methods.add_method("isValid", |_, this, ()| Ok(!this.closure.is_null()));
    }
}

/// Create a callback from a Lua function.
pub fn create_callback(
    lua: &Lua,
    func: LuaFunction,
    ret_type: CType,
    arg_types: Vec<CType>,
) -> LuaResult<FfiCallback> {
    FfiCallback::new(lua, func, ret_type, arg_types)
}
