//! Prepared statement wrapper.

use mlua::prelude::*;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;

use crate::value::lua_to_sql;

/// Prepared SQL statement for repeated execution.
pub struct SqlStatement {
    conn: Arc<Mutex<Connection>>,
    sql: String,
}

impl SqlStatement {
    pub fn new(conn: Arc<Mutex<Connection>>, sql: String) -> LuaResult<Self> {
        // Validate SQL by preparing it
        {
            let c = conn.lock();
            c.prepare(&sql).into_lua_err()?;
        }
        Ok(Self { conn, sql })
    }

    pub fn execute(&self, lua: &Lua, params: Vec<LuaValue>) -> LuaResult<LuaValue> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&self.sql).into_lua_err()?;

        let param_values: Vec<_> = params
            .into_iter()
            .map(|v| lua_to_sql(&v))
            .collect::<LuaResult<_>>()?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = param_values
            .iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();

        if self.sql.trim().to_uppercase().starts_with("SELECT") {
            let column_names: Vec<String> = stmt
                .column_names()
                .iter()
                .map(|s| (*s).to_owned())
                .collect();

            let mut rows = stmt.query(param_refs.as_slice()).into_lua_err()?;
            let result = lua.create_table()?;
            let mut idx = 1;

            while let Some(row) = rows.next().into_lua_err()? {
                let row_table = lua.create_table()?;
                for (i, name) in column_names.iter().enumerate() {
                    let value = crate::value::sql_to_lua(lua, row, i)?;
                    row_table.set(name.as_str(), value)?;
                }
                result.set(idx, row_table)?;
                idx += 1;
            }

            Ok(LuaValue::Table(result))
        } else {
            let affected = stmt.execute(param_refs.as_slice()).into_lua_err()?;
            Ok(LuaValue::Integer(affected as i64))
        }
    }
}

impl LuaUserData for SqlStatement {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // execute(params: {any}?) -> {rows} | number
        methods.add_method("execute", |lua, this, params: Option<LuaTable>| {
            let params: Vec<LuaValue> = params
                .map(|t| t.sequence_values().collect::<LuaResult<_>>())
                .transpose()?
                .unwrap_or_default();
            this.execute(lua, params)
        });
    }
}
