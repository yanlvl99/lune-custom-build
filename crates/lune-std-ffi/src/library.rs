//! Native library wrapper for loading DLLs/SOs.

use std::ffi::CString;
use std::sync::Arc;

use libloading::{Library, Symbol};
use mlua::prelude::*;

/// A loaded native library.
pub struct NativeLibrary {
    library: Arc<Library>,
    path: String,
}

impl NativeLibrary {
    /// Open a native library by path.
    pub fn open(path: &str) -> LuaResult<Self> {
        let library = unsafe { Library::new(path) }
            .map_err(|e| LuaError::external(format!("Failed to load library '{}': {}", path, e)))?;

        Ok(Self {
            library: Arc::new(library),
            path: path.to_owned(),
        })
    }

    /// Get a function pointer from the library.
    ///
    /// # Safety
    /// The caller must ensure the function signature matches.
    unsafe fn get_symbol<T>(&self, name: &str) -> LuaResult<Symbol<T>> {
        let cname = CString::new(name).map_err(|_| LuaError::external("Invalid symbol name"))?;

        self.library
            .get(cname.as_bytes_with_nul())
            .map_err(|e| LuaError::external(format!("Symbol '{}' not found: {}", name, e)))
    }
}

impl Clone for NativeLibrary {
    fn clone(&self) -> Self {
        Self {
            library: Arc::clone(&self.library),
            path: self.path.clone(),
        }
    }
}

impl LuaUserData for NativeLibrary {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // lib:call(name: string, ...args) -> result
        // Calls a function that takes no arguments and returns an i32
        methods.add_method("callInt", |_, this, name: String| {
            type FnType = unsafe extern "C" fn() -> i32;

            let func: Symbol<FnType> = unsafe { this.get_symbol(&name)? };
            let result = unsafe { func() };

            Ok(result)
        });

        // lib:callIntArg(name: string, arg: number) -> number
        // Calls a function that takes one i32 and returns i32
        methods.add_method("callIntArg", |_, this, (name, arg): (String, i32)| {
            type FnType = unsafe extern "C" fn(i32) -> i32;

            let func: Symbol<FnType> = unsafe { this.get_symbol(&name)? };
            let result = unsafe { func(arg) };

            Ok(result)
        });

        // lib:callDouble(name: string) -> number
        // Calls a function that returns a double
        methods.add_method("callDouble", |_, this, name: String| {
            type FnType = unsafe extern "C" fn() -> f64;

            let func: Symbol<FnType> = unsafe { this.get_symbol(&name)? };
            let result = unsafe { func() };

            Ok(result)
        });

        // lib:callVoid(name: string) -> ()
        // Calls a void function with no arguments
        methods.add_method("callVoid", |_, this, name: String| {
            type FnType = unsafe extern "C" fn();

            let func: Symbol<FnType> = unsafe { this.get_symbol(&name)? };
            unsafe { func() };

            Ok(())
        });

        // lib:callString(name: string) -> string
        // Calls a function that returns a C string (const char*)
        methods.add_method("callString", |lua, this, name: String| {
            type FnType = unsafe extern "C" fn() -> *const std::ffi::c_char;

            let func: Symbol<FnType> = unsafe { this.get_symbol(&name)? };
            let ptr = unsafe { func() };

            if ptr.is_null() {
                return Ok(LuaValue::Nil);
            }

            let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
            let s = cstr
                .to_str()
                .map_err(|_| LuaError::external("Invalid UTF-8 in returned string"))?;

            Ok(LuaValue::String(lua.create_string(s)?))
        });

        // lib:hasSymbol(name: string) -> boolean
        // Checks if a symbol exists in the library
        methods.add_method("hasSymbol", |_, this, name: String| {
            let cname = CString::new(name.as_str())
                .map_err(|_| LuaError::external("Invalid symbol name"))?;

            let exists: Result<Symbol<*const ()>, _> =
                unsafe { this.library.get(cname.as_bytes_with_nul()) };

            Ok(exists.is_ok())
        });

        // lib:close() -> ()
        // Explicitly unload the library (also happens on GC)
        methods.add_method("close", |_, _, ()| {
            // Library is dropped when Arc count reaches 0
            Ok(())
        });
    }
}
