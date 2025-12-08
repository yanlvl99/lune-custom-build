-- Comprehensive FFI test using Windows system DLLs
local ffi = require("@lune/ffi")

print("=== FFI Comprehensive Test ===")
print("")

-- Test with MSVCRT (Microsoft Visual C Runtime)
print("Testing msvcrt.dll (C Runtime)...")
local msvcrt = ffi.open("msvcrt.dll")

-- abs(int) -> int
local absResult = msvcrt:call("abs", "i32", { "i32" }, -42)
print("  abs(-42) =", absResult)

-- sqrt(double) -> double
local sqrtResult = msvcrt:call("sqrt", "f64", { "f64" }, 144.0)
print("  sqrt(144) =", sqrtResult)

-- pow(double, double) -> double
local powResult = msvcrt:call("pow", "f64", { "f64", "f64" }, 2.0, 10.0)
print("  pow(2, 10) =", powResult)

-- strlen(char*) -> size_t
local buf = ffi.buffer(64)
buf:writeString(0, "Hello World!")
local lenResult = msvcrt:call("strlen", "u64", { "pointer" }, buf.ptr)
print("  strlen('Hello World!') =", lenResult)

-- atoi(char*) -> int
buf:zero()
buf:writeString(0, "12345")
local atoiResult = msvcrt:call("atoi", "i32", { "pointer" }, buf.ptr)
print("  atoi('12345') =", atoiResult)

-- cos(double) -> double
local cosResult = msvcrt:call("cos", "f64", { "f64" }, 0.0)
print("  cos(0) =", cosResult)

-- sin(double) -> double
local sinResult = msvcrt:call("sin", "f64", { "f64" }, 0.0)
print("  sin(0) =", sinResult)

msvcrt:close()
print("")

-- Test with Kernel32
print("Testing kernel32.dll...")
local kernel32 = ffi.open("kernel32.dll")

-- GetCurrentProcessId() -> DWORD
local pid = kernel32:call("GetCurrentProcessId", "u32", {})
print("  GetCurrentProcessId() =", pid)

-- GetCurrentThreadId() -> DWORD
local tid = kernel32:call("GetCurrentThreadId", "u32", {})
print("  GetCurrentThreadId() =", tid)

-- GetTickCount() -> DWORD (milliseconds since system boot)
local ticks = kernel32:call("GetTickCount", "u32", {})
print("  GetTickCount() =", ticks, "ms")

-- GetLastError() -> DWORD
local lastErr = kernel32:call("GetLastError", "u32", {})
print("  GetLastError() =", lastErr)

-- SetLastError(DWORD)
kernel32:call("SetLastError", "void", { "u32" }, 999)
local newErr = kernel32:call("GetLastError", "u32", {})
print("  SetLastError(999), GetLastError() =", newErr)

-- Beep(dwFreq, dwDuration) - uncomment to hear a beep!
-- kernel32:call("Beep", "i32", {"u32", "u32"}, 750, 200)
-- print("  Beep(750, 200) - should make a sound!")

kernel32:close()
print("")

-- Test with User32
print("Testing user32.dll...")
local user32 = ffi.open("user32.dll")

-- GetSystemMetrics(SM_CXSCREEN = 0, SM_CYSCREEN = 1)
local screenW = user32:call("GetSystemMetrics", "i32", { "i32" }, 0)
local screenH = user32:call("GetSystemMetrics", "i32", { "i32" }, 1)
print("  Screen resolution:", screenW, "x", screenH)

-- GetDoubleClickTime() -> UINT
local dblClickTime = user32:call("GetDoubleClickTime", "u32", {})
print("  GetDoubleClickTime() =", dblClickTime, "ms")

user32:close()
print("")

print("=== All FFI Tests Passed! ===")
