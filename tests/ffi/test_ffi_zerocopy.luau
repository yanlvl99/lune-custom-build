--!nocheck
-- Test FFI Zero-Copy features: Arena, Pointers, Structs
local ffi = require("@lune/ffi")

print("=== FFI Zero-Copy Test ===")
print("")

-- Test Arena allocation
print("1. Arena Allocation:")
local arena = ffi.arena()
print("  Created arena, id:", arena.id)

local raw = arena:alloc(256)
print("  Allocated 256 bytes, addr:", raw.addr)
print("  isManaged:", raw.isManaged)
print("  isNull:", raw.isNull)

-- Test allocArray
local arrayPtr = arena:allocArray("i32", 10)
print("  Allocated i32[10], addr:", arrayPtr.addr)
print("  Arena total allocated:", arena.totalAllocated)
print("  Arena allocation count:", arena.allocationCount)
print("")

-- Test Raw Pointer arithmetic
print("2. Raw Pointer Arithmetic:")
local p1 = raw
local p2 = raw + 10 -- +10 bytes
local p3 = raw + 20 -- +20 bytes
print("  raw addr:", p1.addr)
print("  raw + 10:", p2.addr, "(diff:", p2.addr - p1.addr, "bytes)")
print("  raw + 20:", p3.addr, "(diff:", p3.addr - p1.addr, "bytes)")
print("")

-- Test direct read/write
print("3. Direct Read/Write:")
ffi.write(raw, 0, "i32", 42)
ffi.write(raw, 4, "i32", 100)
ffi.write(raw, 8, "f64", 3.14159)
print("  Wrote i32=42 at offset 0")
print("  Wrote i32=100 at offset 4")
print("  Wrote f64=3.14159 at offset 8")

local v1 = ffi.read(raw, 0, "i32")
local v2 = ffi.read(raw, 4, "i32")
local v3 = ffi.read(raw, 8, "f64")
print("  Read at 0:", v1)
print("  Read at 4:", v2)
print("  Read at 8:", v3)
print("")

-- Test Typed Pointer
print("4. Typed Pointer (cast and indexing):")
local ints = ffi.cast(raw, "i32")
print("  Cast to TypedPointer<i32>, stride:", ints.stride)

-- Write via indexing
ints[0] = 111
ints[1] = 222
ints[2] = 333
print("  Wrote ints[0]=111, ints[1]=222, ints[2]=333")

-- Read via indexing
print("  ints[0]:", ints[0])
print("  ints[1]:", ints[1])
print("  ints[2]:", ints[2])

-- Typed arithmetic
local ints2 = ints + 2 -- +2 elements = +8 bytes
print("  ints + 2: addr diff =", ints2.addr - ints.addr, "bytes (2 * 4)")
print("")

-- Test Struct Definition
print("5. Struct Definition:")
local PlayerDef = ffi.struct({
	{ "x", "f32" },
	{ "y", "f32" },
	{ "z", "f32" },
	{ "health", "i32" },
	{ "mana", "i32" },
})
print("  Created PlayerDef")
print("  size:", PlayerDef.size)
print("  alignment:", PlayerDef.alignment)
print("  fieldCount:", PlayerDef.fieldCount)
print("  offsetOf('x'):", PlayerDef:offsetOf("x"))
print("  offsetOf('y'):", PlayerDef:offsetOf("y"))
print("  offsetOf('health'):", PlayerDef:offsetOf("health"))
print("")

-- Test Struct View
print("6. Struct View:")
local playerPtr = arena:alloc(PlayerDef.size)
local player = ffi.view(playerPtr, PlayerDef)

player.x = 10.5
player.y = 20.3
player.z = -5.0
player.health = 100
player.mana = 50

print("  Set player.x=10.5, y=20.3, z=-5.0, health=100, mana=50")
print("  player.x:", player.x)
print("  player.y:", player.y)
print("  player.z:", player.z)
print("  player.health:", player.health)
print("  player.mana:", player.mana)
print("")

-- Test copy/fill
print("7. Bulk Operations (copy/fill):")
local src = arena:alloc(64)
local dst = arena:alloc(64)

ffi.fill(src, 64, 0xAB)
print("  Filled src with 0xAB")

ffi.copy(dst, src, 64)
print("  Copied 64 bytes from src to dst")

local srcByte = ffi.read(src, 0, "u8")
local dstByte = ffi.read(dst, 0, "u8")
print("  src[0]:", srcByte, string.format("(0x%X)", srcByte))
print("  dst[0]:", dstByte, string.format("(0x%X)", dstByte))
print("")

-- Test arena reset
print("8. Arena Reset:")
print("  Before reset - totalAllocated:", arena.totalAllocated)
arena:reset()
print("  After reset - totalAllocated:", arena.totalAllocated)
print("  allocationCount:", arena.allocationCount)
print("")

print("=== FFI Zero-Copy Test Complete ===")
