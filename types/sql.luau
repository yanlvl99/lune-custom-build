--!strict

export type SqlConnection = {
	path: string,

	--- Execute a SQL query with parameterized values.
	--- For SELECT queries, returns an array of row tables.
	--- For INSERT/UPDATE/DELETE, returns the number of affected rows.
	---
	--- IMPORTANT: Always use parameters for user input to prevent SQL injection!
	--- Example: db:query("SELECT * FROM users WHERE id = ?", {userId})
	query: (self: SqlConnection, sql: string, params: { any }?) -> { [string]: any } | number,

	--- Execute raw SQL for schema operations (CREATE TABLE, etc).
	--- Do NOT use this with user input!
	exec: (self: SqlConnection, sql: string) -> (),

	--- Prepare a statement for repeated execution.
	prepare: (self: SqlConnection, sql: string) -> SqlStatement,

	--- Close the database connection.
	close: (self: SqlConnection) -> (),
}

export type SqlStatement = {
	--- Execute the prepared statement with parameters.
	execute: (self: SqlStatement, params: { any }?) -> { [string]: any } | number,
}

local sql = {}

--- Open a SQLite database file.
--- Creates the file if it doesn't exist.
function sql.open(path: string): SqlConnection
	return nil :: any
end

--- Open an in-memory SQLite database.
--- Data is lost when the connection is closed.
function sql.memory(): SqlConnection
	return nil :: any
end

return sql
