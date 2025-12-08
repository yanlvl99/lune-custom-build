//! SQL Connection wrapper for SQLite.

use mlua::prelude::*;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;

use crate::statement::SqlStatement;
use crate::value::lua_to_sql;

/// SQLite database connection.
pub struct SqlConnection {
    conn: Arc<Mutex<Connection>>,
    path: String,
}

impl SqlConnection {
    /// Open a database file.
    pub fn open(path: &str) -> LuaResult<Self> {
        let conn = Connection::open(path).into_lua_err()?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: path.to_owned(),
        })
    }

    /// Open an in-memory database.
    pub fn memory() -> LuaResult<Self> {
        let conn = Connection::open_in_memory().into_lua_err()?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: ":memory:".to_owned(),
        })
    }

    /// Execute a query with parameters. Returns rows for SELECT, affected count for others.
    pub fn query(&self, lua: &Lua, sql: &str, params: Vec<LuaValue>) -> LuaResult<LuaValue> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(sql).into_lua_err()?;

        let param_values: Vec<_> = params
            .into_iter()
            .map(|v| lua_to_sql(&v))
            .collect::<LuaResult<_>>()?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = param_values
            .iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();

        // Check if it's a SELECT query
        if sql.trim().to_uppercase().starts_with("SELECT") {
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

    /// Execute multiple statements (for schema creation).
    pub fn exec(&self, sql: &str) -> LuaResult<()> {
        let conn = self.conn.lock();
        conn.execute_batch(sql).into_lua_err()
    }

    /// Prepare a statement for repeated execution.
    pub fn prepare(&self, sql: &str) -> LuaResult<SqlStatement> {
        SqlStatement::new(Arc::clone(&self.conn), sql.to_owned())
    }
}

impl Clone for SqlConnection {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            path: self.path.clone(),
        }
    }
}

impl LuaUserData for SqlConnection {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // query(sql: string, params: {any}?) -> {rows} | number
        // CRITICAL: Params are REQUIRED for any user input to prevent SQL injection
        methods.add_method(
            "query",
            |lua, this, (sql, params): (String, Option<LuaTable>)| {
                let params: Vec<LuaValue> = params
                    .map(|t| t.sequence_values().collect::<LuaResult<_>>())
                    .transpose()?
                    .unwrap_or_default();
                this.query(lua, &sql, params)
            },
        );

        // exec(sql: string) -> () - For schema operations only
        methods.add_method("exec", |_, this, sql: String| this.exec(&sql));

        // prepare(sql: string) -> SqlStatement
        methods.add_method("prepare", |_, this, sql: String| this.prepare(&sql));

        // close() - Connection is closed on drop
        methods.add_method("close", |_, _, ()| Ok(()));
    }
}
