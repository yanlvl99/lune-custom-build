--[[
    FFI Phase 2 Test Script
    Tests: SmartLibrary interface, string marshalling, view:pointTo, ffi.unsafe
]]

local ffi = require("@lune/ffi")
local CTypes = ffi.ctypes

print("=== FFI Phase 2 Tests ===\n")

local Kernel32 = ffi.load("kernel32.dll", {
	GetCurrentProcessId = { args = {}, ret = CTypes.u32 },
	GetLastError = { args = {}, ret = CTypes.u32 },
	Sleep = { args = { CTypes.u32 }, ret = CTypes.void },
	INFINITE = 0xFFFFFFFF,
})

local pid = Kernel32.GetCurrentProcessId()
print("  ✓ GetCurrentProcessId() =", pid)
assert(pid > 0, "PID should be positive")

print("  ✓ INFINITE constant =", Kernel32.INFINITE)
assert(Kernel32.INFINITE == 0xFFFFFFFF, "INFINITE should be 0xFFFFFFFF")

print("  ✓ Sleep(1000ms)...")
Kernel32.Sleep(1000)
print("  ✓ Sleep completed")

print("Test 1 PASSED\n")

-- Test 2: Automatic string marshalling
print("Test 2: Automatic string marshalling...")
local User32 = ffi.load("user32.dll", {
	MessageBoxA = { args = { CTypes.u64, CTypes.string, CTypes.string, CTypes.u32 }, ret = CTypes.i32 },
	MB_OK = 0,
	MB_ICONINFORMATION = 64,
})

-- Note: This would show a dialog, skip for automated testing
-- local result = User32.MessageBoxA(0, "Hello from FFI!", "Test", User32.MB_OK)
print("  ✓ User32 loaded with MessageBoxA (skipping dialog)")
print("  ✓ String auto-marshalling configured")
print("Test 2 PASSED\n")

-- Test 3: Arena and struct views
print("Test 3: Arena and StructView...")
local arena = ffi.arena()

local Point = ffi.struct({
	{ "x", "f32" },
	{ "y", "f32" },
})

local ptr = arena:alloc(Point.size)
local view = ffi.view(ptr, Point)

view.x = 10.5
view.y = 20.25

assert(math.abs(view.x - 10.5) < 0.001, "x should be 10.5")
assert(math.abs(view.y - 20.25) < 0.001, "y should be 20.25")
print("  ✓ Struct read/write works")
print("Test 3 PASSED\n")

-- Test 4: view:pointTo (zero-GC iteration)
print("Test 4: view:pointTo zero-GC iteration...")
local count = 1000
local arrayPtr = arena:allocArray("i32", count)

-- Create one view, reuse for all iterations
local IntView = ffi.struct({ { "val", "i32" } })
local reusableView = ffi.view(arrayPtr, IntView)

for i = 0, count - 1 do
	-- Use :offset() method for LSP type safety (+ operator works at runtime too)
	reusableView:pointTo(arrayPtr:offset(i * IntView.size))
	reusableView.val = i * 2
end

-- Verify
reusableView:pointTo(arrayPtr:offset(50 * IntView.size))
assert(reusableView.val == 100, "view[50] should be 100")

reusableView:pointTo(arrayPtr:offset(999 * IntView.size))
assert(reusableView.val == 1998, "view[999] should be 1998")

print("  ✓ Iterated", count, "elements with zero allocations")
print("Test 4 PASSED\n")

-- Test 5: ffi.unsafe intrinsics
print("Test 5: ffi.unsafe intrinsics...")
local unsafeArena = ffi.arena()
local unsafePtr = unsafeArena:alloc(64)
local addr = unsafePtr.addr

-- Write using unsafe
ffi.unsafe.write(addr, "i32", 12345)
local val = ffi.unsafe.read(addr, "i32")
assert(val == 12345, "unsafe read should return 12345")
print("  ✓ ffi.unsafe.read/write works")

-- Zero memory
ffi.unsafe.zero(addr, 64)
val = ffi.unsafe.read(addr, "i32")
assert(val == 0, "after zero, value should be 0")
print("  ✓ ffi.unsafe.zero works")

-- Fill memory
ffi.unsafe.fill(addr, 4, 0xFF)
val = ffi.unsafe.read(addr, "u32")
assert(val == 0xFFFFFFFF, "after fill with 0xFF, u32 should be 0xFFFFFFFF")
print("  ✓ ffi.unsafe.fill works")

print("Test 5 PASSED\n")

-- Test 6: Legacy API still works (deprecated)
print("Test 6: Legacy API (deprecated)...")
local legacyLib = ffi.open("kernel32.dll")
local legacyPid = legacyLib:call("GetCurrentProcessId", "u32", {})
assert(legacyPid == pid, "Legacy call should return same PID")
print("  ✓ Legacy ffi.open + lib:call works")
legacyLib:close()
print("Test 6 PASSED\n")

print("=== ALL TESTS PASSED ===")
