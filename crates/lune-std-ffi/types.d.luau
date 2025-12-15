--[=[
	@within Ffi
	@interface CType

	C type identifiers for FFI operations.

	**Integer types:**
	- `i8` (1 byte) - Signed 8-bit (-128 to 127)
	- `u8` (1 byte) - Unsigned 8-bit (0 to 255)
	- `i16` (2 bytes) - Signed 16-bit
	- `u16` (2 bytes) - Unsigned 16-bit
	- `i32` (4 bytes) - Signed 32-bit (standard int)
	- `u32` (4 bytes) - Unsigned 32-bit
	- `i64` (8 bytes) - Signed 64-bit
	- `u64` (8 bytes) - Unsigned 64-bit
	- `isize` (8 bytes*) - Signed pointer-sized
	- `usize` (8 bytes*) - Unsigned pointer-sized (size_t)

	**Floating point:**
	- `f32` (4 bytes) - Single precision float
	- `f64` (8 bytes) - Double precision float

	**Other:**
	- `void` - Empty type for function returns
	- `bool` (1 byte) - Boolean
	- `pointer` (8 bytes*) - Raw pointer (void*)
	- `string` (8 bytes*) - C string (const char*)

	*Size on 64-bit systems
]=]
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

--- Value types that can be read/written via FFI
export type FfiValue = number | boolean | string | RawPointer

--- Pointer-like types accepted by FFI functions
export type PointerLike = RawPointer | TypedPointer<FfiValue> | Buffer | LightUserData

--- LightUserData type (opaque pointer)
export type LightUserData = typeof(newproxy())

--[=[
	@within Ffi
	@interface RawPointer

	Raw pointer (void*) with byte-level arithmetic.

	**Properties:**
	- `addr` - Memory address as number
	- `isNull` - True if null pointer
	- `isManaged` - True if arena-managed
]=]
export type RawPointer = {
	addr: number,
	isNull: boolean,
	isManaged: boolean,
	read: (self: RawPointer, offset: number, ctype: CType) -> FfiValue,
	write: (self: RawPointer, offset: number, ctype: CType, value: FfiValue) -> (),
	offset: (self: RawPointer, bytes: number) -> RawPointer,
	toLightUserData: (self: RawPointer) -> LightUserData,
}

--[=[
	@within Ffi
	@interface TypedPointer

	Typed pointer (T*) with stride-based arithmetic and array indexing.

	**Properties:**
	- `addr` - Memory address
	- `stride` - Size of each element in bytes
	- `isNull` - True if null pointer
	- `count` - Element count (if known)
]=]
export type TypedPointer<T> = {
	addr: number,
	stride: number,
	isNull: boolean,
	count: number?,
	get: (self: TypedPointer<T>, index: number) -> T,
	set: (self: TypedPointer<T>, index: number, value: T) -> (),
	toRaw: (self: TypedPointer<T>) -> RawPointer,
	toLightUserData: (self: TypedPointer<T>) -> LightUserData,
	[number]: T,
}

--[=[
	@within Ffi
	@interface Buffer

	Legacy buffer type.
]=]
export type Buffer = {
	ptr: RawPointer,
	size: number,
	as_ptr: (self: Buffer) -> RawPointer,
}

--[=[
	@within Ffi
	@interface Arena

	Scoped memory allocator with automatic cleanup.
]=]
export type Arena = {
	alloc: (self: Arena, size: number) -> RawPointer,
	allocAligned: (self: Arena, size: number, align: number) -> RawPointer,
	allocType: (self: Arena, ctype: CType) -> RawPointer,
	allocArray: (self: Arena, ctype: CType, count: number) -> RawPointer,
	reset: (self: Arena) -> (),
	free: (self: Arena) -> (),
}

--[=[
	@within Ffi
	@interface StructDefinition

	C struct layout definition created via `ffi.struct()`.
]=]
export type StructDefinition = {
	size: number,
	alignment: number,
	fieldCount: number,
	createView: (self: StructDefinition, ptr: RawPointer) -> StructView,
}

--[=[
	@within Ffi
	@interface StructView

	Runtime view into a struct at a memory location.
	Field access uses `[string]` indexing.
]=]
export type StructView = {
	size: number,
	addr: number,
	fieldPtr: (self: StructView, fieldName: string) -> RawPointer,
	pointTo: (self: StructView, ptr: RawPointer | number) -> (),
	[string]: any, -- Field values (numbers, booleans, etc.)
}

--[=[
	@within Ffi
	@interface FunctionSignature

	Signature for binding a C function in SmartLibrary.

	- `args` - Array of argument type names (optional)
	- `ret` - Return type name (optional)
]=]
export type FunctionSignature = {
	args: { CType }?,
	ret: CType?,
}

--- Interface definition for SmartLibrary
export type LibraryInterface = { [string]: FunctionSignature | number | string | boolean }

--[=[
	@within Ffi
	@interface Library

	Handle to a loaded native library (.dll, .so, .dylib).
	Used with legacy `lib:call()` API.
]=]
export type Library = {
	path: string,
	hasSymbol: (self: Library, name: string) -> boolean,
	getSymbol: (self: Library, name: string) -> RawPointer?,
	call: (
		self: Library,
		name: string,
		retType: CType,
		argTypes: { CType },
		...FfiValue
	) -> FfiValue,
	callInt: (self: Library, name: string) -> number,
	callVoid: (self: Library, name: string) -> (),
	close: (self: Library) -> (),
}

--[=[
	@within Ffi
	@interface SmartLibrary

	Library with pre-bound interface for direct function access.
	Created via `ffi.load(path, interface)`.

	Functions and constants are accessed directly as fields.
]=]
export type SmartLibrary = {
	path: string,
	close: (self: SmartLibrary) -> (),
	[string]: any, -- Functions, constants, etc.
}

--[=[
	@within Ffi
	@interface UnsafeIntrinsics

	Direct memory access without safety checks.

	**WARNING**: No null checks, no bounds checks!
]=]
export type UnsafeIntrinsics = {
	read: (addr: number, ctype: CType) -> FfiValue,
	write: (addr: number, ctype: CType, value: FfiValue) -> (),
	copy: (dst: number, src: number, len: number) -> (),
	fill: (addr: number, len: number, byte: number) -> (),
	zero: (addr: number, len: number) -> (),
}

--[=[
	@within Ffi
	@interface FfiCallback

	Callback wrapper for passing Lua functions to C code.
]=]
export type FfiCallback = {
	ptr: LightUserData,
	retType: CType,
	argCount: number,
	isValid: boolean,
	getPtr: (self: FfiCallback) -> LightUserData,
}

--[=[
	@class Ffi

	Foreign Function Interface (FFI) module for Lune.
	Provides zero-copy memory access, native library loading, and C-ABI struct mapping.

	### Quick Start

	```lua
	local ffi = require("@lune/ffi")

	-- Create memory arena (auto-cleanup)
	local arena = ffi.arena()
	local ptr = arena:alloc(1024)

	-- Load native library with interface
	local C = ffi.ctypes
	local Kernel32 = ffi.load("kernel32.dll", {
		GetCurrentProcessId = { ret = C.u32 },
		Sleep = { args = { C.u32 } },
	})
	local pid = Kernel32.GetCurrentProcessId()
	```
]=]
local ffi = {}

--[=[
	@within Ffi
	@tag must_use

	Create a memory arena for scoped allocations.
	Memory is automatically cleaned up when arena is garbage collected.

	@return Arena -- New memory arena
]=]
function ffi.arena(): Arena
	return nil :: any
end

--[=[
	@within Ffi
	@tag must_use

	Load a native library (.dll, .so, .dylib).

	**Without interface**: Use `lib:call()` for function calls.
	**With interface**: Access functions directly as fields.

	@param path -- Path to the library
	@param interface -- Optional interface definition
	@return Library & SmartLibrary -- Intersection allows both patterns
]=]
function ffi.load(path: string, interface: LibraryInterface?): Library & SmartLibrary
	return nil :: any
end

--[=[
	@within Ffi
	@tag must_use
	@deprecated Use ffi.load() instead

	Load a native library (legacy alias).

	@param path -- Path to the library
	@return Library
]=]
function ffi.open(path: string): Library
	return nil :: any
end

--[=[
	@within Ffi
	@tag must_use

	Define a C struct layout from schema.

	@param schema -- Array of field definitions: { {"name", "type"}, ... }
	@return StructDefinition
]=]
function ffi.struct(schema: { { string | number } }): StructDefinition
	return nil :: any
end

--[=[
	@within Ffi
	@tag must_use

	Create a struct view at a pointer location.

	@param ptr -- Pointer to the memory
	@param structDef -- Struct definition
	@return StructView
]=]
function ffi.view(ptr: PointerLike, structDef: StructDefinition): StructView
	return nil :: any
end

--[=[
	@within Ffi
	@tag must_use

	Cast a raw pointer to a typed pointer.

	@param ptr -- Raw pointer to cast
	@param ctype -- Target type
	@return TypedPointer
]=]
function ffi.cast(ptr: PointerLike, ctype: CType): TypedPointer<FfiValue>
	return nil :: any
end

--[=[
	@within Ffi
	@tag must_use

	Create a RawPointer from a numeric address.

	@param addr -- Memory address
	@return RawPointer
]=]
function ffi.ptr(addr: number): RawPointer
	return nil :: any
end

--[=[
	@within Ffi
	@tag must_use

	Read a primitive value from memory.

	@param ptr -- Pointer to read from
	@param offset -- Byte offset
	@param ctype -- Type to read
	@return FfiValue
]=]
function ffi.read(ptr: PointerLike, offset: number, ctype: CType): FfiValue
	return nil :: any
end

--[=[
	@within Ffi

	Write a primitive value to memory.

	@param ptr -- Pointer to write to
	@param offset -- Byte offset
	@param ctype -- Type to write
	@param value -- Value to write
]=]
function ffi.write(ptr: PointerLike, offset: number, ctype: CType, value: FfiValue): () end

--[=[
	@within Ffi

	Copy memory (SIMD-optimized memcpy).

	@param dst -- Destination pointer
	@param src -- Source pointer
	@param len -- Number of bytes to copy
]=]
function ffi.copy(dst: PointerLike, src: PointerLike, len: number): () end

--[=[
	@within Ffi

	Fill memory with a byte value (SIMD-optimized memset).

	@param ptr -- Pointer to fill
	@param len -- Number of bytes
	@param byte -- Byte value (0-255)
]=]
function ffi.fill(ptr: PointerLike, len: number, byte: number): () end

--[=[
	@within Ffi
	@tag must_use

	Read a C string from memory.

	@param ptr -- Pointer to string
	@param len -- Optional max length
	@return string?
]=]
function ffi.string(ptr: PointerLike, len: number?): string?
	return nil
end

--[=[
	@within Ffi
	@tag must_use

	Get size of a C type in bytes.

	@param ctype -- Type to query
	@return number
]=]
function ffi.sizeof(ctype: CType): number
	return 0
end

--[=[
	@within Ffi
	@tag must_use

	Get alignment of a C type.

	@param ctype -- Type to query
	@return number
]=]
function ffi.alignof(ctype: CType): number
	return 0
end

--[=[
	@within Ffi
	@tag must_use

	Check if a pointer is null.

	@param ptr -- Pointer to check
	@return boolean
]=]
function ffi.isNull(ptr: PointerLike): boolean
	return false
end

--[=[
	@within Ffi
	@tag must_use

	Create a callback for C code to call into Lua.

	@param fn -- Lua function to wrap
	@param retType -- Return type
	@param argTypes -- Argument types
	@return FfiCallback
]=]
function ffi.callback(
	fn: (...FfiValue) -> FfiValue,
	retType: CType,
	argTypes: { CType }
): FfiCallback
	return nil :: any
end

--[=[
	@within Ffi

	C definition placeholder (not yet implemented).

	@param def -- C header definitions
]=]
function ffi.cdef(def: string): () end

--- Null LightUserData (legacy).
ffi.null = newproxy() :: LightUserData

--- Null RawPointer.
ffi.nullPtr = (nil :: any) :: RawPointer

--- Direct memory access without safety checks.
--- **WARNING**: No null checks, no bounds checks!
ffi.unsafe = (nil :: any) :: UnsafeIntrinsics

--[=[
	@within Ffi
	@interface CTypesTable

	C type identifiers for FFI operations.

	**Integer types:**
	- `i8` (1 byte) - Signed 8-bit (-128 to 127)
	- `u8` (1 byte) - Unsigned 8-bit (0 to 255)
	- `i16` (2 bytes) - Signed 16-bit
	- `u16` (2 bytes) - Unsigned 16-bit
	- `i32` (4 bytes) - Signed 32-bit (standard int)
	- `u32` (4 bytes) - Unsigned 32-bit
	- `i64` (8 bytes) - Signed 64-bit
	- `u64` (8 bytes) - Unsigned 64-bit
	- `isize` (8 bytes*) - Signed pointer-sized
	- `usize` (8 bytes*) - Unsigned pointer-sized (size_t)

	**Floating point:**
	- `f32` (4 bytes) - Single precision float
	- `f64` (8 bytes) - Double precision float

	**Other:**
	- `void` - Empty type for function returns
	- `bool` (1 byte) - Boolean
	- `pointer` (8 bytes*) - Raw pointer (void*)
	- `string` (8 bytes*) - C string (const char*)

	*Size on 64-bit systems

	Type name constants for use in function signatures.

	**Example:**
	```lua
	local C = ffi.ctypes
	local Kernel32 = ffi.load("kernel32.dll", {
		GetCurrentProcessId = { ret = C.u32 },
		Sleep = { args = { C.u32 } },
	})
	```
]=]
export type CTypesTable = {
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
	f32: CType,
	f64: CType,
	isize: CType,
	usize: CType,
	pointer: CType,
	string: CType,
}

--- Type name constants (use instead of string literals).
ffi.ctypes = (nil :: any) :: CTypesTable

return ffi
