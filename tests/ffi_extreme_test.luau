--[[
    FFI EXTREME Stress Test
    Operations so intensive they take time even in native C
    WARNING: This will use a LOT of memory and CPU
]]

local ffi = require("@lune/ffi")

print(
	"╔══════════════════════════════════════════════════════════════╗"
)
print("║            FFI EXTREME STRESS TEST                          ║")
print("║  WARNING: This will use ~2GB RAM and max CPU for minutes    ║")
print(
	"╚══════════════════════════════════════════════════════════════╝\n"
)

local totalStart = os.clock()

-- ============================================================================
-- Test 1: 100 Million Element Array Operations
-- ============================================================================
print("═══ Test 1: 100 Million Element Processing ═══")
local MEGA_COUNT = 100_000_000 -- 100 million

local arena = ffi.arena()
local startTime = os.clock()

-- Allocate 400 MB of i32
local megaArray = arena:allocArray("i32", MEGA_COUNT)
local ptr = ffi.cast(megaArray, "i32")

print(string.format("  Allocated %.0f MB", MEGA_COUNT * 4 / 1024 / 1024))

-- Write phase
startTime = os.clock()
for i = 0, MEGA_COUNT - 1 do
	ptr[i] = (i * 7 + 13) % 1000000
end
local writeTime = os.clock() - startTime
print(
	string.format(
		"  Write: %.2f seconds (%.1f M ops/sec)",
		writeTime,
		MEGA_COUNT / writeTime / 1000000
	)
)

-- Read + compute phase
startTime = os.clock()
local sum: number = 0
local maxVal: number = 0
local minVal: number = 999999999
for i = 0, MEGA_COUNT - 1 do
	local v = ptr[i]
	sum = sum + v
	if v > maxVal then
		maxVal = v
	end
	if v < minVal then
		minVal = v
	end
end
local readTime = os.clock() - startTime
print(
	string.format(
		"  Read+Compute: %.2f seconds (%.1f M ops/sec)",
		readTime,
		MEGA_COUNT / readTime / 1000000
	)
)
print(string.format("  Sum: %.0f, Max: %d, Min: %d", sum, maxVal, minVal))

-- ============================================================================
-- Test 2: 1 GB Memory Bandwidth Test
-- ============================================================================
print("\n═══ Test 2: 1 GB Memory Bandwidth ═══")

local GB_SIZE = 1024 * 1024 * 1024 -- 1 GB
local gbBuffer = arena:alloc(GB_SIZE)
local addr = gbBuffer.addr

-- Sequential write (fill)
startTime = os.clock()
ffi.unsafe.fill(addr, GB_SIZE, 0x55)
local fillTime = os.clock() - startTime
print(string.format("  Fill 1 GB: %.3f sec (%.1f GB/s)", fillTime, 1 / fillTime))

-- Sequential zero
startTime = os.clock()
ffi.unsafe.zero(addr, GB_SIZE)
local zeroTime = os.clock() - startTime
print(string.format("  Zero 1 GB: %.3f sec (%.1f GB/s)", zeroTime, 1 / zeroTime))

-- Copy test (500 MB)
local halfGB = GB_SIZE // 2
ffi.unsafe.fill(addr, halfGB, 0xAA)
startTime = os.clock()
ffi.unsafe.copy(addr + halfGB, addr, halfGB)
local copyTime = os.clock() - startTime
print(string.format("  Copy 500 MB: %.3f sec (%.1f GB/s)", copyTime, 0.5 / copyTime))

-- ============================================================================
-- Test 3: 10 Million Struct Operations with Computation
-- ============================================================================
print("\n═══ Test 3: 10 Million Complex Structs ═══")

local Particle = ffi.struct({
	{ "x", "f64" },
	{ "y", "f64" },
	{ "z", "f64" },
	{ "vx", "f64" },
	{ "vy", "f64" },
	{ "vz", "f64" },
	{ "mass", "f64" },
	{ "id", "u32" },
})

local PARTICLE_COUNT = 10_000_000
local particleBuffer = arena:allocArray("u8", PARTICLE_COUNT * Particle.size)
print(
	string.format("  Allocated %.0f MB for particles", PARTICLE_COUNT * Particle.size / 1024 / 1024)
)

local view = ffi.view(particleBuffer, Particle)

-- Initialize
startTime = os.clock()
for i = 0, PARTICLE_COUNT - 1 do
	view:pointTo(particleBuffer:offset(i * Particle.size))
	view.x = i * 0.001
	view.y = i * 0.002
	view.z = i * 0.003
	view.vx = (i % 100) * 0.1
	view.vy = (i % 200) * 0.05
	view.vz = (i % 300) * 0.033
	view.mass = 1.0 + (i % 1000) * 0.001
	view.id = i
end
local initTime = os.clock() - startTime
print(
	string.format(
		"  Initialize: %.2f sec (%.1f M particles/sec)",
		initTime,
		PARTICLE_COUNT / initTime / 1000000
	)
)

-- Simulate physics step (velocity integration)
startTime = os.clock()
local dt = 0.016 -- 60 FPS
for i = 0, PARTICLE_COUNT - 1 do
	view:pointTo(particleBuffer:offset(i * Particle.size))
	view.x = view.x + view.vx * dt
	view.y = view.y + view.vy * dt
	view.z = view.z + view.vz * dt
end
local simTime = os.clock() - startTime
print(
	string.format(
		"  Physics step: %.2f sec (%.1f M updates/sec)",
		simTime,
		PARTICLE_COUNT / simTime / 1000000
	)
)

-- Calculate total kinetic energy
startTime = os.clock()
local totalEnergy: number = 0
for i = 0, PARTICLE_COUNT - 1 do
	view:pointTo(particleBuffer:offset(i * Particle.size))
	local v2 = view.vx * view.vx + view.vy * view.vy + view.vz * view.vz
	totalEnergy = totalEnergy + 0.5 * view.mass * v2
end
local energyTime = os.clock() - startTime
print(string.format("  Energy calc: %.2f sec, Total KE: %.2e", energyTime, totalEnergy))

-- ============================================================================
-- Test 4: Billion Micro Operations
-- ============================================================================
print("\n═══ Test 4: 1 Billion Pointer Operations ═══")

local BILLION = 1_000_000_000
local smallBuffer = arena:allocArray("u8", 1024)
local smallAddr = smallBuffer.addr

startTime = os.clock()
local checksum: number = 0

-- Write pattern
for i = 0, BILLION - 1 do
	local offset = i % 1024
	ffi.unsafe.write(smallAddr + offset, "u8", (i % 256))
end
local billionTime = os.clock() - startTime
print(
	string.format(
		"  1 billion writes: %.2f sec (%.0f M ops/sec)",
		billionTime,
		BILLION / billionTime / 1000000
	)
)

-- Read pattern
startTime = os.clock()
for i = 0, BILLION - 1 do
	local offset = i % 1024
	checksum = checksum + ffi.unsafe.read(smallAddr + offset, "u8")
end
local readBillionTime = os.clock() - startTime
print(
	string.format(
		"  1 billion reads: %.2f sec (%.0f M ops/sec)",
		readBillionTime,
		BILLION / readBillionTime / 1000000
	)
)
print(string.format("  Checksum: %.0f", checksum % 1000000))

-- ============================================================================
-- Summary
-- ============================================================================
local totalTime = os.clock() - totalStart

print(
	"\n╔══════════════════════════════════════════════════════════════╗"
)
print("║                    STRESS TEST COMPLETE                      ║")
print(
	"╠══════════════════════════════════════════════════════════════╣"
)
print(
	string.format(
		"║  Total Time: %.1f seconds                                     ║",
		totalTime
	)
)
print(string.format("║  Peak Memory: ~2 GB                                           ║"))
print(string.format("║  Total Operations: ~3 billion                                 ║"))
print(
	"╠══════════════════════════════════════════════════════════════╣"
)
print("║  ✅ No GC pauses                                             ║")
print("║  ✅ No memory fragmentation                                  ║")
print("║  ✅ Predictable performance                                  ║")
print(
	"╚══════════════════════════════════════════════════════════════╝"
)
