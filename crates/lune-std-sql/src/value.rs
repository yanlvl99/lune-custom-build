//! Value conversion between Lua and SQL types.

use mlua::prelude::*;
use rusqlite::{Row, types::Value as SqlValue};

/// Convert Lua value to SQL value.
pub fn lua_to_sql(value: &LuaValue) -> LuaResult<SqlValue> {
    match value {
        LuaValue::Nil => Ok(SqlValue::Null),
        LuaValue::Boolean(b) => Ok(SqlValue::Integer(i64::from(*b))),
        LuaValue::Integer(i) => Ok(SqlValue::Integer(*i)),
        LuaValue::Number(n) => Ok(SqlValue::Real(*n)),
        LuaValue::String(s) => Ok(SqlValue::Text(s.to_str()?.to_owned())),
        _ => Err(LuaError::external(format!(
            "Cannot convert {:?} to SQL value",
            value.type_name()
        ))),
    }
}

/// Convert SQL value from row to Lua value.
pub fn sql_to_lua(lua: &Lua, row: &Row, idx: usize) -> LuaResult<LuaValue> {
    use rusqlite::types::ValueRef;

    let value_ref = row.get_ref(idx).into_lua_err()?;

    match value_ref {
        ValueRef::Null => Ok(LuaValue::Nil),
        ValueRef::Integer(i) => Ok(LuaValue::Integer(i)),
        ValueRef::Real(r) => Ok(LuaValue::Number(r)),
        ValueRef::Text(t) => {
            let s = std::str::from_utf8(t).into_lua_err()?;
            Ok(LuaValue::String(lua.create_string(s)?))
        }
        ValueRef::Blob(b) => Ok(LuaValue::String(lua.create_string(b)?)),
    }
}
