//! Struct layout mapper for C-ABI compliant memory access.
//!
//! Parses field definitions and calculates proper offsets with padding.

use mlua::prelude::*;
use std::collections::HashMap;
use std::ffi::c_void;

use crate::pointer::RawPointer;
use crate::types::CType;

/// A field in a struct definition
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ctype: CType,
    pub offset: usize,
    pub size: usize,
    /// For fixed arrays: [u8; 32] has array_len = 32
    pub array_len: Option<usize>,
}

/// A compiled struct definition with layout info
#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub name: Option<String>,
    pub fields: Vec<StructField>,
    pub field_map: HashMap<String, usize>,
    pub size: usize,
    pub alignment: usize,
}

impl StructDefinition {
    /// Parse a schema table into a struct definition
    ///
    /// Schema format: { {"name", "type"}, {"name2", "type2"}, ... }
    /// Or with arrays: { {"name", "u8", 32}, ... } for fixed arrays
    pub fn from_schema(_lua: &Lua, schema: LuaTable) -> LuaResult<Self> {
        let mut fields = Vec::new();
        let mut field_map = HashMap::new();
        let mut offset = 0usize;
        let mut max_align = 1usize;

        for pair in schema.sequence_values::<LuaTable>() {
            let field_def = pair?;

            // Get field name
            let name: String = field_def.get(1)?;

            // Get field type
            let type_val: LuaValue = field_def.get(2)?;
            let ctype = match type_val {
                LuaValue::String(s) => {
                    let type_str = s.to_str()?;
                    CType::from_str(&type_str)
                        .ok_or_else(|| LuaError::external(format!("Unknown type: {}", type_str)))?
                }
                _ => return Err(LuaError::external("Field type must be a string")),
            };

            // Check for array length (optional 3rd element)
            let array_len: Option<usize> = field_def.get(3).ok();

            let field_size = ctype.size();
            let field_align = ctype.alignment();

            // Calculate actual size (considering arrays)
            let actual_size = if let Some(len) = array_len {
                field_size * len
            } else {
                field_size
            };

            // Align offset
            let padding = (field_align - (offset % field_align)) % field_align;
            offset += padding;

            // Store field
            field_map.insert(name.clone(), fields.len());
            fields.push(StructField {
                name,
                ctype,
                offset,
                size: actual_size,
                array_len,
            });

            offset += actual_size;
            max_align = max_align.max(field_align);
        }

        // Final struct size with trailing padding
        let trailing_padding = (max_align - (offset % max_align)) % max_align;
        let total_size = offset + trailing_padding;

        Ok(Self {
            name: None,
            fields,
            field_map,
            size: total_size,
            alignment: max_align,
        })
    }

    /// Get field by name
    pub fn get_field(&self, name: &str) -> Option<&StructField> {
        self.field_map.get(name).map(|&i| &self.fields[i])
    }

    /// Get field by index
    pub fn get_field_by_index(&self, index: usize) -> Option<&StructField> {
        self.fields.get(index)
    }
}

impl LuaUserData for StructDefinition {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("size", |_, this| Ok(this.size));
        fields.add_field_method_get("alignment", |_, this| Ok(this.alignment));
        fields.add_field_method_get("fieldCount", |_, this| Ok(this.fields.len()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Get offset of a field
        methods.add_method("offsetOf", |_, this, name: String| {
            this.get_field(&name)
                .map(|f| f.offset)
                .ok_or_else(|| LuaError::external(format!("Unknown field: {}", name)))
        });

        // Get size of a field
        methods.add_method("sizeOf", |_, this, name: String| {
            this.get_field(&name)
                .map(|f| f.size)
                .ok_or_else(|| LuaError::external(format!("Unknown field: {}", name)))
        });

        // Get all field names
        methods.add_method("fields", |lua, this, ()| {
            let names: Vec<String> = this.fields.iter().map(|f| f.name.clone()).collect();
            lua.create_sequence_from(names)
        });

        // ToString
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            let fields_str: Vec<String> = this
                .fields
                .iter()
                .map(|f| {
                    if let Some(len) = f.array_len {
                        format!("  {} {:?}[{}] @ {}", f.name, f.ctype, len, f.offset)
                    } else {
                        format!("  {} {:?} @ {}", f.name, f.ctype, f.offset)
                    }
                })
                .collect();
            Ok(format!(
                "StructDefinition(size={}, align={}) {{\n{}\n}}",
                this.size,
                this.alignment,
                fields_str.join("\n")
            ))
        });
    }
}

// ============================================================================
// StructView - Runtime access to struct fields via pointer
// ============================================================================

/// A view into a struct at a memory location
pub struct StructView {
    pub ptr: *mut c_void,
    pub def: StructDefinition,
    pub arena_id: usize,
}

impl StructView {
    /// Create a view from a pointer and definition
    pub fn new(ptr: &RawPointer, def: StructDefinition) -> Self {
        Self {
            ptr: ptr.addr,
            def,
            arena_id: ptr.arena_id,
        }
    }

    /// Read a field by name
    pub fn read_field(&self, lua: &Lua, name: &str) -> LuaResult<LuaValue> {
        let field = self
            .def
            .get_field(name)
            .ok_or_else(|| LuaError::external(format!("Unknown field: {}", name)))?;

        let ptr = unsafe { self.ptr.cast::<u8>().add(field.offset) };
        crate::pointer::read_value_at(lua, ptr, field.ctype)
    }

    /// Write a field by name
    pub fn write_field(&self, lua: &Lua, name: &str, value: LuaValue) -> LuaResult<()> {
        let field = self
            .def
            .get_field(name)
            .ok_or_else(|| LuaError::external(format!("Unknown field: {}", name)))?;

        let ptr = unsafe { self.ptr.cast::<u8>().add(field.offset) };
        crate::pointer::write_value_at(lua, ptr, field.ctype, value)
    }

    /// Get pointer to a field (for arrays or nested structs)
    pub fn field_ptr(&self, name: &str) -> LuaResult<RawPointer> {
        let field = self
            .def
            .get_field(name)
            .ok_or_else(|| LuaError::external(format!("Unknown field: {}", name)))?;

        let ptr = unsafe { self.ptr.cast::<u8>().add(field.offset) };
        Ok(RawPointer::managed(ptr.cast(), self.arena_id, field.size))
    }
}

impl LuaUserData for StructView {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("size", |_, this| Ok(this.def.size));
        fields.add_field_method_get("addr", |_, this| Ok(this.ptr as usize));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Field access via indexing: view.health, view.position
        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: String| {
            // Check for built-in properties first
            match key.as_str() {
                "size" => return Ok(LuaValue::Integer(this.def.size as i64)),
                "addr" => return Ok(LuaValue::Integer(this.ptr as usize as i64)),
                _ => {}
            }
            this.read_field(lua, &key)
        });

        // Field assignment: view.health = 100
        methods.add_meta_method(
            LuaMetaMethod::NewIndex,
            |lua, this, (key, value): (String, LuaValue)| this.write_field(lua, &key, value),
        );

        // Get pointer to a field
        methods.add_method("fieldPtr", |_, this, name: String| this.field_ptr(&name));

        // ToString
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "StructView(0x{:x}, size={})",
                this.ptr as usize, this.def.size
            ))
        });
    }
}
