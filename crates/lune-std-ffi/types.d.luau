--[=[
	@class Ffi
	
	Foreign Function Interface (FFI) module for Lune.
	Provides zero-copy memory access, native library loading, and C-ABI struct mapping.
	
	## Quick Start
	```lua
	local ffi = require("@lune/ffi")
	
	-- Create memory arena (auto-cleanup)
	local arena = ffi.arena()
	local ptr = arena:alloc(1024)
	
	-- Cast to typed pointer for array access
	local ints = ffi.cast(ptr, "i32")
	ints[0] = 42
	print(ints[0]) -- 42
	
	-- Load native library
	local lib = ffi.load("kernel32.dll")
	local pid = lib:call("GetCurrentProcessId", "u32", {})
	```
	
	## Safety
	- **Safe**: Arena allocation, struct definitions, type queries
	- **Unsafe**: Direct pointer arithmetic, raw memory read/write
	
	@example
	```lua
	-- Define a C struct
	local Vec3 = ffi.struct({
		{ "x", "f32" },
		{ "y", "f32" },
		{ "z", "f32" },
	})
	
	-- Allocate and access
	local arena = ffi.arena()
	local ptr = arena:alloc(Vec3.size)
	local vec = ffi.view(ptr, Vec3)
	vec.x, vec.y, vec.z = 1.0, 2.0, 3.0
	```
]=]

--- C type identifiers for FFI operations.
--- @example
--- ```lua
--- ffi.sizeof("i32")  -- 4
--- ffi.sizeof("f64")  -- 8
--- ffi.sizeof("pointer")  -- 8 (on 64-bit)
--- ```
export type CType =
	"void"
	| "bool"
	| "i8"
	| "u8"
	| "i16"
	| "u16"
	| "i32"
	| "u32"
	| "i64"
	| "u64"
	| "isize"
	| "usize"
	| "f32"
	| "f64"
	| "pointer"
	| "string"

--[=[
	@class RawPointer
	Raw pointer (void*) with byte-level arithmetic.
	
	**Arithmetic**: `ptr + n` advances by `n` bytes.
	**Indexing**: Disabled - cast to TypedPointer first.
	
	@example
	```lua
	local arena = ffi.arena()
	local raw = arena:alloc(100)
	
	-- Byte-level arithmetic
	local p2 = raw + 10  -- +10 bytes
	
	-- Direct read/write
	ffi.write(raw, 0, "i32", 42)
	print(ffi.read(raw, 0, "i32"))  -- 42
	
	-- Cast for array access
	local ints = ffi.cast(raw, "i32")
	ints[0] = 100
	```
]=]
export type RawPointer = {
	--- Raw memory address as number
	addr: number,
	--- Whether this pointer is null
	isNull: boolean,
	--- Whether this pointer is managed by an arena (bounds-checked)
	isManaged: boolean,

	--- Read a value at byte offset.
	--- @example `local val = ptr:read(0, "i32")`
	read: (self: RawPointer, offset: number, ctype: CType) -> any,

	--- Write a value at byte offset.
	--- @example `ptr:write(0, "i32", 42)`
	write: (self: RawPointer, offset: number, ctype: CType, value: any) -> (),

	--- Create an offset pointer (alternative to + operator).
	--- @example `local p2 = ptr:offset(16)`
	offset: (self: RawPointer, bytes: number) -> RawPointer,

	--- Convert to LightUserData for C function calls.
	--- @example `lib:call("func", "void", {"pointer"}, ptr:toLightUserData())`
	toLightUserData: (self: RawPointer) -> any,
}

--[=[
	@class TypedPointer
	Typed pointer (T*) with stride-based arithmetic and array indexing.
	
	**Arithmetic**: `ptr + n` advances by `n * sizeof(T)` bytes.
	**Indexing**: `ptr[i]` reads/writes at index `i`.
	
	@example
	```lua
	local arena = ffi.arena()
	local raw = arena:allocArray("i32", 10)
	local ints = ffi.cast(raw, "i32")
	
	-- Array indexing
	ints[0] = 111
	ints[1] = 222
	print(ints[0], ints[1])  -- 111, 222
	
	-- Stride-based arithmetic
	local p2 = ints + 2  -- +8 bytes (2 * sizeof(i32))
	```
]=]
export type TypedPointer<T = any> = {
	--- Raw memory address as number
	addr: number,
	--- Bytes per element (sizeof(T))
	stride: number,
	--- Whether this pointer is null
	isNull: boolean,
	--- Known element count (if arena-managed)
	count: number?,

	--- Read value at index.
	--- @example `local val = ptr:get(0)`
	get: (self: TypedPointer<T>, index: number) -> T,

	--- Write value at index.
	--- @example `ptr:set(0, 42)`
	set: (self: TypedPointer<T>, index: number, value: T) -> (),

	--- Convert back to RawPointer.
	toRaw: (self: TypedPointer<T>) -> RawPointer,

	--- Convert to LightUserData for C calls.
	toLightUserData: (self: TypedPointer<T>) -> any,
}

--[=[
	@class Arena
	Scoped memory arena with automatic cleanup.
	
	All allocations are freed when the Arena is garbage collected.
	
	> **Thread Safety**: Arena is NOT thread-safe. Do not share between Luau Actors.
	
	@example
	```lua
	local arena = ffi.arena()
	local p1 = arena:alloc(1024)
	local p2 = arena:allocArray("f32", 100)
	
	print(arena.totalAllocated)  -- 1424+
	arena:reset()  -- Free all allocations
	print(arena.totalAllocated)  -- 0
	```
]=]
export type Arena = {
	--- Unique arena identifier
	id: number,
	--- Total bytes currently allocated
	totalAllocated: number,
	--- Number of individual allocations
	allocationCount: number,

	--- Allocate raw bytes.
	--- @example `local ptr = arena:alloc(256)`
	alloc: (self: Arena, size: number) -> RawPointer,

	--- Allocate with specific alignment.
	--- @example `local ptr = arena:allocAligned(256, 16)`
	allocAligned: (self: Arena, size: number, align: number) -> RawPointer,

	--- Allocate for a specific C type.
	--- @example `local ptr = arena:allocType("f64")`
	allocType: (self: Arena, ctype: CType) -> RawPointer,

	--- Allocate an array of elements.
	--- @example `local ptr = arena:allocArray("i32", 100)`
	allocArray: (self: Arena, ctype: CType, count: number) -> RawPointer,

	--- Free all allocations.
	--- @example `arena:reset()`
	reset: (self: Arena) -> (),
}

--[=[
	@class StructDefinition
	Compiled struct layout with C-ABI alignment.
	
	@example
	```lua
	local Player = ffi.struct({
		{ "x", "f32" },
		{ "y", "f32" },
		{ "health", "i32" },
		{ "name", "u8", 32 },  -- Fixed array [u8; 32]
	})
	
	print(Player.size)           -- 44
	print(Player:offsetOf("health"))  -- 8
	```
]=]
export type StructDefinition = {
	--- Total struct size in bytes (includes padding)
	size: number,
	--- Struct alignment requirement
	alignment: number,
	--- Number of fields
	fieldCount: number,

	--- Get byte offset of a field.
	--- @example `local off = def:offsetOf("health")`
	offsetOf: (self: StructDefinition, fieldName: string) -> number,

	--- Get size of a field (including array length).
	--- @example `local sz = def:sizeOf("name")`
	sizeOf: (self: StructDefinition, fieldName: string) -> number,

	--- Get all field names.
	--- @example `for _, name in def:fields() do print(name) end`
	fields: (self: StructDefinition) -> { string },
}

--[=[
	@class StructView
	Runtime view into a struct at a memory location.
	Access fields by name via indexing.
	
	@example
	```lua
	local Player = ffi.struct({
		{ "x", "f32" },
		{ "y", "f32" },
		{ "health", "i32" },
	})
	
	local arena = ffi.arena()
	local ptr = arena:alloc(Player.size)
	local player = ffi.view(ptr, Player)
	
	player.x = 10.5
	player.health = 100
	print(player.health)  -- 100
	```
]=]
export type StructView = {
	--- Total struct size in bytes
	size: number,
	--- Memory address
	addr: number,

	--- Get pointer to a specific field.
	--- @example `local namePtr = view:fieldPtr("name")`
	fieldPtr: (self: StructView, fieldName: string) -> RawPointer,

	--- Dynamic field access via indexing
	[string]: any,
}

--[=[
	@class Library
	Handle to a loaded native library (.dll, .so, .dylib).
	
	@example
	```lua
	local lib = ffi.load("kernel32.dll")
	
	-- Modern: lazy symbol resolution (future)
	-- lib.GetCurrentProcessId()
	
	-- Legacy: explicit call
	local pid = lib:call("GetCurrentProcessId", "u32", {})
	print("PID:", pid)
	
	lib:close()
	```
]=]
export type Library = {
	--- Path of the loaded library
	path: string,

	--- Check if a symbol exists.
	--- @example `if lib:hasSymbol("MyFunc") then ... end`
	hasSymbol: (self: Library, name: string) -> boolean,

	--- Get raw pointer to a symbol.
	--- @example `local ptr = lib:getSymbol("g_GlobalVar")`
	getSymbol: (self: Library, name: string) -> RawPointer?,

	--- List all exported symbols (Windows: ordinals included).
	--- @example `for _, exp in lib:listExports() do print(exp.name) end`
	listExports: (self: Library) -> { ExportInfo },

	--- Call a function with explicit signature.
	--- @deprecated Use lazy symbol resolution in future versions.
	--- @example `lib:call("MessageBoxA", "i32", {"pointer", "string", "string", "u32"}, nil, "Hello", "Title", 0)`
	call: (self: Library, name: string, retType: CType, argTypes: { CType }, ...any) -> any,

	--- Optimized call for (void) -> i32 functions.
	callInt: (self: Library, name: string) -> number,

	--- Optimized call for (i64) -> i32 functions.
	callIntArg: (self: Library, name: string, arg: number) -> number,

	--- Optimized call for (void) -> f64 functions.
	callDouble: (self: Library, name: string) -> number,

	--- Optimized call for (void) -> void functions.
	callVoid: (self: Library, name: string) -> (),

	--- Optimized call for (void) -> string functions.
	callString: (self: Library, name: string) -> string,

	--- Call via function pointer.
	callPtr: (
		self: Library,
		ptr: RawPointer | any,
		retType: CType,
		argTypes: { CType },
		...any
	) -> any,

	--- Close the library handle.
	--- @example `lib:close()`
	close: (self: Library) -> (),
}

--- Information about an exported symbol.
export type ExportInfo = {
	name: string,
	ordinal: number?, -- Windows only
}

--- Legacy buffer type (still supported).
export type Buffer = {
	size: number,
	ptr: any,
	read: (self: Buffer, offset: number, ctype: CType) -> any,
	write: (self: Buffer, offset: number, ctype: CType, value: any) -> (),
	zero: (self: Buffer) -> (),
	writeBytes: (self: Buffer, offset: number, bytes: buffer) -> (),
	readBytes: (self: Buffer, offset: number, len: number) -> buffer,
	readString: (self: Buffer, offset: number?) -> string,
	writeString: (self: Buffer, offset: number, s: string) -> (),
	slice: (self: Buffer, offset: number, size: number) -> Buffer,
}

--- Callback wrapper for C function pointers.
export type FfiCallback = {
	ptr: any,
	retType: CType,
	argCount: number,
	isValid: boolean,
	getPtr: (self: FfiCallback) -> any,
}

--- Type constants table.
export type Types = {
	void: CType,
	bool: CType,
	i8: CType,
	u8: CType,
	i16: CType,
	u16: CType,
	i32: CType,
	u32: CType,
	i64: CType,
	u64: CType,
	isize: CType,
	usize: CType,
	f32: CType,
	f64: CType,
	pointer: CType,
	string: CType,
	-- Aliases
	int: CType,
	uint: CType,
	long: CType,
	ulong: CType,
	float: CType,
	double: CType,
	char: CType,
	uchar: CType,
	short: CType,
	ushort: CType,
	size_t: CType,
	ptr: CType,
	sizeof: (ctype: CType) -> number,
	alignof: (ctype: CType) -> number,
}

--- Schema for struct definition.
--- @example `{{ "x", "f32" }, { "data", "u8", 64 }}`
export type StructSchema = { { string | number } }

-- ============================================================================
-- Module Exports
-- ============================================================================

return {
	-- ========== Memory Allocation ==========

	--- Create a memory arena for scoped allocations.
	--- @example `local arena = ffi.arena()`
	arena = (nil :: any) :: () -> Arena,

	--- Create a legacy buffer (prefer arena for new code).
	--- @deprecated Use ffi.arena() for automatic cleanup.
	buffer = (nil :: any) :: (size: number) -> Buffer,

	-- ========== Memory Access ==========

	--- Read a primitive from memory.
	--- @example `local val = ffi.read(ptr, 0, "i32")`
	read = (
			nil :: any
		) :: (ptr: RawPointer | TypedPointer<any> | Buffer, offset: number, ctype: CType) -> any,

	--- Write a primitive to memory.
	--- @example `ffi.write(ptr, 0, "i32", 42)`
	write = (nil :: any) :: (
		ptr: RawPointer | TypedPointer<any> | Buffer,
		offset: number,
		ctype: CType,
		value: any
	) -> (),

	--- Copy memory (SIMD-optimized memcpy).
	--- @example `ffi.copy(dst, src, 1024)`
	copy = (nil :: any) :: (
		dst: RawPointer | TypedPointer<any>,
		src: RawPointer | TypedPointer<any>,
		len: number
	) -> (),

	--- Fill memory with a byte value (SIMD-optimized memset).
	--- @example `ffi.fill(ptr, 1024, 0)`
	fill = (nil :: any) :: (ptr: RawPointer | TypedPointer<any>, len: number, byte: number) -> (),

	-- ========== Pointer Operations ==========

	--- Create a RawPointer from a numeric address.
	--- @example `local ptr = ffi.ptr(0x12345678)`
	ptr = (nil :: any) :: (addr: number) -> RawPointer,

	--- Cast a pointer to a typed pointer or struct view.
	--- @example `local ints = ffi.cast(raw, "i32")`
	--- @example `local player = ffi.cast(raw, PlayerDef)`
	cast = (nil :: any) :: ((ptr: RawPointer | any, ctype: CType) -> TypedPointer<any>) & ((
			ptr: RawPointer | any,
			structDef: StructDefinition
		) -> StructView),

	-- ========== Struct System ==========

	--- Define a struct layout from schema.
	--- @example `local Vec3 = ffi.struct({ {"x", "f32"}, {"y", "f32"}, {"z", "f32"} })`
	struct = (nil :: any) :: (schema: StructSchema) -> StructDefinition,

	--- Create a struct view at a pointer.
	--- @example `local player = ffi.view(ptr, PlayerDef)`
	view = (
			nil :: any
		) :: (ptr: RawPointer | TypedPointer<any>, structDef: StructDefinition) -> StructView,

	-- ========== Library Loading ==========

	--- Load a native library (modern API).
	--- @example `local lib = ffi.load("kernel32.dll")`
	load = (nil :: any) :: (path: string) -> Library,

	--- Load a native library (legacy alias).
	--- @deprecated Use ffi.load() instead.
	open = (nil :: any) :: (path: string) -> Library,

	-- ========== String Operations ==========

	--- Read a C string from memory.
	--- @example `local s = ffi.string(ptr)` or `ffi.string(ptr, 64)`
	string = (nil :: any) :: (ptr: RawPointer | TypedPointer<any> | any, len: number?) -> string?,

	-- ========== Null Pointers ==========

	--- Null LightUserData (legacy).
	null = (nil :: any) :: any,

	--- Null RawPointer.
	nullPtr = (nil :: any) :: RawPointer,

	--- Check if a pointer is null.
	--- @example `if ffi.isNull(ptr) then ... end`
	isNull = (nil :: any) :: (ptr: RawPointer | TypedPointer<any> | any) -> boolean,

	-- ========== Type Information ==========

	--- Get size of a C type in bytes.
	--- @example `ffi.sizeof("i32")  -- 4`
	sizeof = (nil :: any) :: (ctype: CType) -> number,

	--- Get alignment of a C type.
	--- @example `ffi.alignof("f64")  -- 8`
	alignof = (nil :: any) :: (ctype: CType) -> number,

	--- Type constants and utilities.
	types = (nil :: any) :: Types,

	-- ========== Callbacks ==========

	--- Create a callback for C code to call into Lua.
	--- @example `local cb = ffi.callback(function(a, b) return a + b end, "i32", {"i32", "i32"})`
	callback = (
			nil :: any
		) :: (fn: (...any) -> any, retType: CType, argTypes: { CType }) -> FfiCallback,

	--- C definition placeholder (not implemented).
	cdef = (nil :: any) :: (def: string) -> (),
}
