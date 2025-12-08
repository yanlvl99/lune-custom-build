//! Native library wrapper for loading DLLs/SOs with dynamic function calling.

use std::ffi::CString;
use std::sync::Arc;

use libloading::Library;
use mlua::prelude::*;

use crate::caller::dynamic_call;
use crate::types::CType;

/// A loaded native library with full dynamic calling capabilities.
pub struct NativeLibrary {
    library: Arc<Library>,
    path: String,
}

impl NativeLibrary {
    /// Open a native library by path.
    #[allow(clippy::missing_errors_doc)]
    pub fn open(path: &str) -> LuaResult<Self> {
        let library = unsafe { Library::new(path) }
            .map_err(|e| LuaError::external(format!("Failed to load library '{path}': {e}")))?;

        Ok(Self {
            library: Arc::new(library),
            path: path.to_owned(),
        })
    }

    /// Get a raw symbol pointer by name.
    fn get_symbol_ptr(&self, name: &str) -> LuaResult<*const std::ffi::c_void> {
        let cname = CString::new(name).map_err(|_| LuaError::external("Invalid symbol name"))?;

        unsafe {
            self.library
                .get::<*const std::ffi::c_void>(cname.as_bytes_with_nul())
                .map(|sym| *sym)
                .map_err(|e| LuaError::external(format!("Symbol '{name}' not found: {e}")))
        }
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
        // lib:getSymbol(name) -> pointer
        // Get raw symbol address
        methods.add_method("getSymbol", |_, this, name: String| {
            let ptr = this.get_symbol_ptr(&name)?;
            Ok(LuaLightUserData(ptr.cast_mut()))
        });

        // lib:hasSymbol(name) -> boolean
        methods.add_method("hasSymbol", |_, this, name: String| {
            Ok(this.get_symbol_ptr(&name).is_ok())
        });

        // lib:call(name, returnType, argTypes, ...args) -> result
        // Universal dynamic function call
        methods.add_method(
            "call",
            |lua, this, (name, ret_type, arg_types, args): (String, CType, LuaTable, LuaMultiValue)| {
                let fn_ptr = this.get_symbol_ptr(&name)?;

                // Parse argument types
                let arg_types: Vec<CType> = arg_types
                    .sequence_values::<CType>()
                    .collect::<LuaResult<Vec<_>>>()?;

                // Collect args
                let args: Vec<LuaValue> = args.into_vec();

                dynamic_call(lua, fn_ptr, ret_type, &arg_types, args)
            },
        );

        // lib:callPtr(ptr, returnType, argTypes, ...args) -> result
        // Call function by pointer
        methods.add_method(
            "callPtr",
            |lua,
             _,
             (ptr, ret_type, arg_types, args): (
                LuaLightUserData,
                CType,
                LuaTable,
                LuaMultiValue,
            )| {
                let arg_types: Vec<CType> = arg_types
                    .sequence_values::<CType>()
                    .collect::<LuaResult<Vec<_>>>()?;

                let args: Vec<LuaValue> = args.into_vec();

                dynamic_call(lua, ptr.0.cast_const(), ret_type, &arg_types, args)
            },
        );

        // Convenience methods for common signatures (backwards compatible)
        methods.add_method("callInt", |lua, this, name: String| {
            let fn_ptr = this.get_symbol_ptr(&name)?;
            dynamic_call(lua, fn_ptr, CType::I32, &[], vec![])
        });

        methods.add_method("callIntArg", |lua, this, (name, arg): (String, i64)| {
            let fn_ptr = this.get_symbol_ptr(&name)?;
            dynamic_call(
                lua,
                fn_ptr,
                CType::I32,
                &[CType::I32],
                vec![arg.into_lua(lua)?],
            )
        });

        methods.add_method("callDouble", |lua, this, name: String| {
            let fn_ptr = this.get_symbol_ptr(&name)?;
            dynamic_call(lua, fn_ptr, CType::F64, &[], vec![])
        });

        methods.add_method("callVoid", |lua, this, name: String| {
            let fn_ptr = this.get_symbol_ptr(&name)?;
            dynamic_call(lua, fn_ptr, CType::Void, &[], vec![])
        });

        methods.add_method("callString", |lua, this, name: String| {
            let fn_ptr = this.get_symbol_ptr(&name)?;
            dynamic_call(lua, fn_ptr, CType::CString, &[], vec![])
        });

        // lib:close() -> ()
        methods.add_method("close", |_, _, ()| Ok(()));
    }
}

/// Creates a bound function from a library and symbol name.
/// This allows storing the function reference and calling it multiple times efficiently.
pub struct BoundFunction {
    /// We keep the library alive to ensure the function pointer remains valid
    #[allow(dead_code)]
    library: Arc<Library>,
    fn_ptr: *const std::ffi::c_void,
    ret_type: CType,
    arg_types: Vec<CType>,
}

unsafe impl Send for BoundFunction {}
unsafe impl Sync for BoundFunction {}

impl BoundFunction {
    pub fn new(
        library: Arc<Library>,
        name: &str,
        ret_type: CType,
        arg_types: Vec<CType>,
    ) -> LuaResult<Self> {
        let cname = CString::new(name).map_err(|_| LuaError::external("Invalid symbol name"))?;

        let fn_ptr = unsafe {
            library
                .get::<*const std::ffi::c_void>(cname.as_bytes_with_nul())
                .map(|sym| *sym)
                .map_err(|e| LuaError::external(format!("Symbol '{name}' not found: {e}")))?
        };

        Ok(Self {
            library,
            fn_ptr,
            ret_type,
            arg_types,
        })
    }
}

impl LuaUserData for BoundFunction {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // func(...args) -> result
        methods.add_meta_method(LuaMetaMethod::Call, |lua, this, args: LuaMultiValue| {
            let args: Vec<LuaValue> = args.into_vec();
            dynamic_call(lua, this.fn_ptr, this.ret_type, &this.arg_types, args)
        });
    }
}
