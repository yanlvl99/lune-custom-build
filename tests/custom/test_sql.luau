-- Test SQL module
local sql = require("@lune/sql")

print("=== SQL Module Test ===")
print("")

-- Create an in-memory database
local db = sql.memory()
print("Database opened:", db)
print("Path:", db.path)

-- Create a table using exec (for schema)
db:exec([[
    CREATE TABLE users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        age INTEGER
    )
]])
print("Table 'users' created")

-- Insert data using query
db:query("INSERT INTO users (name, age) VALUES (?, ?)", { "Alice", 30 })
db:query("INSERT INTO users (name, age) VALUES (?, ?)", { "Bob", 25 })
db:query("INSERT INTO users (name, age) VALUES (?, ?)", { "Charlie", 35 })
print("Inserted 3 users")

-- Query all data
local rows = db:query("SELECT * FROM users ORDER BY age")
print("")
print("Query results:")
for _, row in ipairs(rows) do
	print(string.format("  ID: %d, Name: %s, Age: %d", row.id, row.name, row.age))
end

-- Query with parameters
local result = db:query("SELECT * FROM users WHERE age > ?", { 28 })
print("")
print("Users older than 28:")
for _, row in ipairs(result) do
	print("  " .. row.name)
end

-- Close database
db:close()
print("")
print("=== SQL Test Complete ===")
