-- Test FFI module
local ffi = require("@lune/ffi")

print("=== FFI Module Test ===")
print("")

-- Test basic properties
print("ffi.null:", ffi.null)
print("ffi.isNull(ffi.null):", ffi.isNull(ffi.null))

-- Test sizeof
print("")
print("Type sizes:")
print("  sizeof('i8'):", ffi.sizeof("i8"))
print("  sizeof('i16'):", ffi.sizeof("i16"))
print("  sizeof('i32'):", ffi.sizeof("i32"))
print("  sizeof('i64'):", ffi.sizeof("i64"))
print("  sizeof('f32'):", ffi.sizeof("f32"))
print("  sizeof('f64'):", ffi.sizeof("f64"))
print("  sizeof('pointer'):", ffi.sizeof("pointer"))

-- Test buffer
print("")
print("Buffer test:")
local buf = ffi.buffer(64)
print("  Created buffer of size:", buf.size)
print("  Buffer ptr:", buf.ptr)

-- Write and read
buf:write(0, "i32", 12345)
local val = buf:read(0, "i32")
print("  Wrote i32 12345, read back:", val)

buf:write(8, "f64", 3.14159)
local fval = buf:read(8, "f64")
print("  Wrote f64 3.14159, read back:", fval)

-- Write string
buf:writeString(16, "Hello FFI!")
local str = buf:readString(16)
print("  Wrote string 'Hello FFI!', read back:", str)

-- Test types table
print("")
print("Types table:")
print("  ffi.types.int:", ffi.types.int)
print("  ffi.types.float:", ffi.types.float)
print("  ffi.types.pointer:", ffi.types.pointer)

-- Test loading kernel32.dll (standard Windows DLL)
print("")
print("Loading kernel32.dll...")
local ok, lib = pcall(ffi.open, "kernel32.dll")
if ok then
	print("  Loaded successfully!")
	print("  Path:", lib.path)

	-- Check if symbol exists
	print("  hasSymbol('GetLastError'):", lib:hasSymbol("GetLastError"))
	print("  hasSymbol('NonExistent'):", lib:hasSymbol("NonExistent"))

	-- Call GetLastError (returns DWORD/u32, no args)
	local lastError = lib:call("GetLastError", "u32", {})
	print("  GetLastError():", lastError)

	-- Call GetCurrentProcessId
	local pid = lib:call("GetCurrentProcessId", "u32", {})
	print("  GetCurrentProcessId():", pid)

	lib:close()
else
	print("  Failed to load:", lib)
end

print("")
print("=== FFI Test Complete ===")
