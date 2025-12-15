//! Smart library with pre-bound interface for zero-overhead FFI calls.
//!
//! Provides direct `lib.FunctionName()` access without per-call signature parsing.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_void};
use std::sync::Arc;

use libffi::middle::{Arg, Builder, Cif, CodePtr, Type as FfiType};
use libloading::Library;
use mlua::prelude::*;

use crate::pointer::RawPointer;
use crate::scratch_arena::SCRATCH_ARENA;
use crate::types::{Buffer, CType};

/// Convert CType to libffi Type
#[inline]
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
        CType::ISize => FfiType::isize(),
        CType::USize => FfiType::usize(),
        CType::F32 => FfiType::f32(),
        CType::F64 => FfiType::f64(),
        CType::Pointer | CType::CString => FfiType::pointer(),
    }
}

// ============================================================================
// SmartBoundFunction - Pre-compiled callable
// ============================================================================

/// A pre-compiled function binding with automatic type marshalling.
///
/// The CIF (Call Interface) is compiled once at bind time, not on every call.
/// String arguments are automatically converted via the thread-local scratch arena.
pub struct SmartBoundFunction {
    /// Keep library loaded
    #[allow(dead_code)]
    library: Arc<Library>,
    /// Function pointer
    fn_ptr: *const c_void,
    /// Return type
    ret_type: CType,
    /// Argument types
    arg_types: Vec<CType>,
    /// Pre-compiled libffi CIF
    cif: Cif,
}

// Safety: The function pointer and library handle are thread-safe
unsafe impl Send for SmartBoundFunction {}
unsafe impl Sync for SmartBoundFunction {}

impl Clone for SmartBoundFunction {
    fn clone(&self) -> Self {
        // Rebuild CIF since it's not Clone
        let ffi_args: Vec<FfiType> = self.arg_types.iter().map(|t| ctype_to_ffi(*t)).collect();
        let ffi_ret = ctype_to_ffi(self.ret_type);
        let cif = Builder::new().args(ffi_args).res(ffi_ret).into_cif();

        Self {
            library: Arc::clone(&self.library),
            fn_ptr: self.fn_ptr,
            ret_type: self.ret_type,
            arg_types: self.arg_types.clone(),
            cif,
        }
    }
}

impl SmartBoundFunction {
    /// Create a new smart bound function.
    pub fn new(
        library: Arc<Library>,
        fn_ptr: *const c_void,
        ret_type: CType,
        arg_types: Vec<CType>,
    ) -> LuaResult<Self> {
        // Pre-compile the CIF
        let ffi_args: Vec<FfiType> = arg_types.iter().map(|t| ctype_to_ffi(*t)).collect();
        let ffi_ret = ctype_to_ffi(ret_type);
        let cif = Builder::new().args(ffi_args).res(ffi_ret).into_cif();

        Ok(Self {
            library,
            fn_ptr,
            ret_type,
            arg_types,
            cif,
        })
    }

    /// Call the function with automatic marshalling.
    fn call_with_args(&self, lua: &Lua, args: LuaMultiValue) -> LuaResult<LuaValue> {
        let args_vec: Vec<LuaValue> = args.into_vec();

        if args_vec.len() != self.arg_types.len() {
            return Err(LuaError::external(format!(
                "Expected {} arguments, got {}",
                self.arg_types.len(),
                args_vec.len()
            )));
        }

        // Use scratch arena for string conversions
        SCRATCH_ARENA.with(|arena| {
            let mut arena = arena.borrow_mut();

            // Storage for argument values (keeps them alive during call)
            let mut storage = ArgStorage::new();

            // Convert each argument
            for (value, ctype) in args_vec.into_iter().zip(&self.arg_types) {
                storage.push(lua, value, *ctype, &mut arena)?;
            }

            // Build libffi args
            let ffi_args: Vec<Arg> = storage.as_args();

            // Perform the call
            let result = self.call_cif(lua, &ffi_args);

            // Reset scratch arena after call
            arena.reset();

            result
        })
    }

    /// Perform the actual FFI call.
    #[inline]
    fn call_cif(&self, lua: &Lua, args: &[Arg]) -> LuaResult<LuaValue> {
        let code_ptr = CodePtr::from_ptr(self.fn_ptr);

        Ok(match self.ret_type {
            CType::Void => {
                unsafe { self.cif.call::<()>(code_ptr, args) };
                LuaValue::Nil
            }
            CType::Bool => {
                let r: i8 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Boolean(r != 0)
            }
            CType::I8 => {
                let r: i8 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(i64::from(r))
            }
            CType::U8 => {
                let r: u8 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(i64::from(r))
            }
            CType::I16 => {
                let r: i16 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(i64::from(r))
            }
            CType::U16 => {
                let r: u16 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(i64::from(r))
            }
            CType::I32 => {
                let r: i32 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(i64::from(r))
            }
            CType::U32 => {
                let r: u32 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(i64::from(r))
            }
            CType::I64 => {
                let r: i64 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(r)
            }
            CType::U64 => {
                let r: u64 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Number(r as f64)
            }
            CType::ISize => {
                let r: isize = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(r as i64)
            }
            CType::USize => {
                let r: usize = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Integer(r as i64)
            }
            CType::F32 => {
                let r: f32 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Number(f64::from(r))
            }
            CType::F64 => {
                let r: f64 = unsafe { self.cif.call(code_ptr, args) };
                LuaValue::Number(r)
            }
            CType::Pointer => {
                let r: *mut c_void = unsafe { self.cif.call(code_ptr, args) };
                if r.is_null() {
                    LuaValue::Nil
                } else {
                    LuaValue::LightUserData(LuaLightUserData(r))
                }
            }
            CType::CString => {
                let r: *const i8 = unsafe { self.cif.call(code_ptr, args) };
                if r.is_null() {
                    LuaValue::Nil
                } else {
                    let cstr = unsafe { CStr::from_ptr(r) };
                    LuaValue::String(lua.create_string(cstr.to_bytes())?)
                }
            }
        })
    }
}

impl LuaUserData for SmartBoundFunction {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Call, |lua, this, args: LuaMultiValue| {
            this.call_with_args(lua, args)
        });

        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "SmartBoundFunction({} args -> {:?})",
                this.arg_types.len(),
                this.ret_type
            ))
        });
    }
}

// ============================================================================
// ArgStorage - Keeps argument values alive during FFI call
// ============================================================================

/// Temporary storage for argument values during a call.
///
/// This avoids allocations by using inline storage for common cases.
struct ArgStorage {
    // Inline storage for common numeric types
    i8s: Vec<i8>,
    u8s: Vec<u8>,
    i16s: Vec<i16>,
    u16s: Vec<u16>,
    i32s: Vec<i32>,
    u32s: Vec<u32>,
    i64s: Vec<i64>,
    u64s: Vec<u64>,
    f32s: Vec<f32>,
    f64s: Vec<f64>,
    ptrs: Vec<*mut c_void>,
    // For owned CStrings (when not using scratch arena)
    cstrings: Vec<CString>,
    // Argument indices mapping to storage
    args: Vec<ArgRef>,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum ArgRef {
    I8(usize),
    U8(usize),
    I16(usize),
    U16(usize),
    I32(usize),
    U32(usize),
    I64(usize),
    U64(usize),
    F32(usize),
    F64(usize),
    Ptr(usize),
    CStr(usize),
}

impl ArgStorage {
    fn new() -> Self {
        Self {
            i8s: Vec::new(),
            u8s: Vec::new(),
            i16s: Vec::new(),
            u16s: Vec::new(),
            i32s: Vec::new(),
            u32s: Vec::new(),
            i64s: Vec::new(),
            u64s: Vec::new(),
            f32s: Vec::new(),
            f64s: Vec::new(),
            ptrs: Vec::new(),
            cstrings: Vec::new(),
            args: Vec::new(),
        }
    }

    fn push(
        &mut self,
        lua: &Lua,
        value: LuaValue,
        ctype: CType,
        scratch: &mut crate::scratch_arena::ScratchArena,
    ) -> LuaResult<()> {
        let arg_ref = match ctype {
            CType::Void => return Err(LuaError::external("Cannot pass void as argument")),

            CType::Bool => {
                let v: bool = FromLua::from_lua(value, lua)?;
                let idx = self.i8s.len();
                self.i8s.push(if v { 1 } else { 0 });
                ArgRef::I8(idx)
            }

            CType::I8 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.i8s.len();
                self.i8s.push(v as i8);
                ArgRef::I8(idx)
            }

            CType::U8 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.u8s.len();
                self.u8s.push(v as u8);
                ArgRef::U8(idx)
            }

            CType::I16 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.i16s.len();
                self.i16s.push(v as i16);
                ArgRef::I16(idx)
            }

            CType::U16 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.u16s.len();
                self.u16s.push(v as u16);
                ArgRef::U16(idx)
            }

            CType::I32 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.i32s.len();
                self.i32s.push(v as i32);
                ArgRef::I32(idx)
            }

            CType::U32 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.u32s.len();
                self.u32s.push(v as u32);
                ArgRef::U32(idx)
            }

            CType::I64 => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.i64s.len();
                self.i64s.push(v);
                ArgRef::I64(idx)
            }

            CType::U64 => {
                let v: f64 = FromLua::from_lua(value, lua)?;
                let idx = self.u64s.len();
                self.u64s.push(v as u64);
                ArgRef::U64(idx)
            }

            CType::ISize => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.i64s.len();
                self.i64s.push(v);
                ArgRef::I64(idx)
            }

            CType::USize => {
                let v: i64 = FromLua::from_lua(value, lua)?;
                let idx = self.u64s.len();
                self.u64s.push(v as u64);
                ArgRef::U64(idx)
            }

            CType::F32 => {
                let v: f64 = FromLua::from_lua(value, lua)?;
                let idx = self.f32s.len();
                self.f32s.push(v as f32);
                ArgRef::F32(idx)
            }

            CType::F64 => {
                let v: f64 = FromLua::from_lua(value, lua)?;
                let idx = self.f64s.len();
                self.f64s.push(v);
                ArgRef::F64(idx)
            }

            CType::Pointer => {
                let ptr = match value {
                    LuaValue::Nil => std::ptr::null_mut(),
                    LuaValue::LightUserData(ud) => ud.0,
                    LuaValue::UserData(ud) => {
                        if let Ok(raw) = ud.borrow::<RawPointer>() {
                            raw.addr
                        } else if let Ok(buf) = ud.borrow::<Buffer>() {
                            buf.as_ptr().cast()
                        } else {
                            return Err(LuaError::external("Expected pointer, buffer, or nil"));
                        }
                    }
                    LuaValue::Integer(i) => i as usize as *mut c_void,
                    LuaValue::Number(n) => n as usize as *mut c_void,
                    LuaValue::String(s) => {
                        // Auto-convert string to char* via scratch arena
                        let borrowed = s.as_bytes();
                        let bytes: &[u8] = &*borrowed;
                        scratch.alloc_cstring(bytes).ok_or_else(|| {
                            LuaError::external("Scratch arena overflow for string argument")
                        })? as *mut c_void
                    }
                    _ => {
                        return Err(LuaError::external(
                            "Expected pointer, buffer, string, or nil",
                        ));
                    }
                };
                let idx = self.ptrs.len();
                self.ptrs.push(ptr);
                ArgRef::Ptr(idx)
            }

            CType::CString => {
                match value {
                    LuaValue::String(s) => {
                        // Use scratch arena for zero-GC conversion
                        let borrowed = s.as_bytes();
                        let bytes: &[u8] = &*borrowed;
                        let ptr = scratch.alloc_cstring(bytes).ok_or_else(|| {
                            LuaError::external("Scratch arena overflow for string argument")
                        })?;
                        let idx = self.ptrs.len();
                        self.ptrs.push(ptr as *mut c_void);
                        ArgRef::Ptr(idx)
                    }
                    LuaValue::Nil => {
                        let idx = self.ptrs.len();
                        self.ptrs.push(std::ptr::null_mut());
                        ArgRef::Ptr(idx)
                    }
                    LuaValue::LightUserData(ud) => {
                        let idx = self.ptrs.len();
                        self.ptrs.push(ud.0);
                        ArgRef::Ptr(idx)
                    }
                    _ => return Err(LuaError::external("Expected string, pointer, or nil")),
                }
            }
        };

        self.args.push(arg_ref);
        Ok(())
    }

    fn as_args(&self) -> Vec<Arg> {
        self.args
            .iter()
            .map(|r| match r {
                ArgRef::I8(i) => Arg::new(&self.i8s[*i]),
                ArgRef::U8(i) => Arg::new(&self.u8s[*i]),
                ArgRef::I16(i) => Arg::new(&self.i16s[*i]),
                ArgRef::U16(i) => Arg::new(&self.u16s[*i]),
                ArgRef::I32(i) => Arg::new(&self.i32s[*i]),
                ArgRef::U32(i) => Arg::new(&self.u32s[*i]),
                ArgRef::I64(i) => Arg::new(&self.i64s[*i]),
                ArgRef::U64(i) => Arg::new(&self.u64s[*i]),
                ArgRef::F32(i) => Arg::new(&self.f32s[*i]),
                ArgRef::F64(i) => Arg::new(&self.f64s[*i]),
                ArgRef::Ptr(i) => Arg::new(&self.ptrs[*i]),
                ArgRef::CStr(i) => Arg::new(&self.cstrings[*i].as_ptr()),
            })
            .collect()
    }
}

// ============================================================================
// SmartLibrary - Library with pre-bound interface
// ============================================================================

/// A library with pre-bound interface for direct function access.
///
/// Created via `ffi.load(path, interface)`.
///
/// # Example
/// ```lua
/// local User32 = ffi.load("user32.dll", {
///     MessageBoxA = { args = {"u64", "string", "string", "u32"}, ret = "i32" },
///     MB_OK = 0,
/// })
///
/// User32.MessageBoxA(0, "Hello!", "Title", User32.MB_OK)
/// ```
pub struct SmartLibrary {
    /// Keep library loaded
    #[allow(dead_code)]
    library: Arc<Library>,
    /// Library path
    path: String,
    /// Pre-bound functions
    functions: HashMap<String, SmartBoundFunction>,
    /// Constants
    constants: HashMap<String, ConstantValue>,
}

/// A constant value in the interface
#[derive(Clone)]
pub enum ConstantValue {
    Integer(i64),
    Number(f64),
    String(String),
    Boolean(bool),
}

impl SmartLibrary {
    /// Create a SmartLibrary from an interface definition.
    pub fn from_interface(
        library: Arc<Library>,
        path: String,
        interface: LuaTable,
    ) -> LuaResult<Self> {
        let mut functions = HashMap::new();
        let mut constants = HashMap::new();

        for pair in interface.pairs::<String, LuaValue>() {
            let (name, value) = pair?;

            match value {
                // Function definition: { args = {...}, ret = "..." }
                LuaValue::Table(sig) => {
                    let ret_type: CType = sig.get("ret").unwrap_or(CType::Void);

                    let arg_types: Vec<CType> = match sig.get::<LuaTable>("args") {
                        Ok(args_tbl) => args_tbl
                            .sequence_values::<CType>()
                            .collect::<LuaResult<Vec<_>>>()?,
                        Err(_) => Vec::new(),
                    };

                    // Get symbol pointer
                    let cname = CString::new(name.as_str())
                        .map_err(|_| LuaError::external("Invalid symbol name"))?;

                    let fn_ptr = unsafe {
                        library
                            .get::<*const c_void>(cname.as_bytes_with_nul())
                            .map(|sym| *sym)
                            .map_err(|e| {
                                LuaError::external(format!("Symbol '{}' not found: {}", name, e))
                            })?
                    };

                    let bound =
                        SmartBoundFunction::new(Arc::clone(&library), fn_ptr, ret_type, arg_types)?;

                    functions.insert(name, bound);
                }

                // Integer constant
                LuaValue::Integer(n) => {
                    constants.insert(name, ConstantValue::Integer(n));
                }

                // Number constant
                LuaValue::Number(n) => {
                    constants.insert(name, ConstantValue::Number(n));
                }

                // String constant
                LuaValue::String(s) => {
                    constants.insert(name, ConstantValue::String(s.to_str()?.to_string()));
                }

                // Boolean constant
                LuaValue::Boolean(b) => {
                    constants.insert(name, ConstantValue::Boolean(b));
                }

                _ => {} // Ignore other types
            }
        }

        Ok(Self {
            library,
            path,
            functions,
            constants,
        })
    }
}

impl LuaUserData for SmartLibrary {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // lib.FunctionName or lib.CONSTANT via __index
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            // Check constants first (faster lookup)
            if let Some(constant) = this.constants.get(&key) {
                return match constant {
                    ConstantValue::Integer(n) => Ok(LuaValue::Integer(*n)),
                    ConstantValue::Number(n) => Ok(LuaValue::Number(*n)),
                    ConstantValue::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
                    ConstantValue::Boolean(b) => Ok(LuaValue::Boolean(*b)),
                };
            }

            // Check bound functions
            if let Some(func) = this.functions.get(&key) {
                return func.clone().into_lua(lua);
            }

            // Not found
            Ok(LuaValue::Nil)
        });

        // close() method
        methods.add_method("close", |_, _, ()| Ok(()));

        // toString
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "SmartLibrary('{}', {} functions, {} constants)",
                this.path,
                this.functions.len(),
                this.constants.len()
            ))
        });
    }
}
