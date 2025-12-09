--[=[
	@class Ffi

	FFI (Foreign Function Interface) module for Lune.

	Provides the ability to load native libraries (.dll, .so, .dylib)
	and call functions from them with arbitrary signatures.
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
	| "f32"
	| "f64"
	| "pointer"
	| "string"

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

export type ExportInfo = {
	name: string,
	ordinal: number?,
}

export type NativeLibrary = {
	path: string,
	getSymbol: (self: NativeLibrary, name: string) -> any,
	hasSymbol: (self: NativeLibrary, name: string) -> boolean,
	listExports: (self: NativeLibrary) -> { ExportInfo },
	call: (
		self: NativeLibrary,
		name: string,
		returnType: CType,
		argTypes: { CType },
		...any
	) -> any,
	callPtr: (self: NativeLibrary, ptr: any, returnType: CType, argTypes: { CType }, ...any) -> any,
	callInt: (self: NativeLibrary, name: string) -> number,
	callDouble: (self: NativeLibrary, name: string) -> number,
	callVoid: (self: NativeLibrary, name: string) -> (),
	callString: (self: NativeLibrary, name: string) -> string?,
	close: (self: NativeLibrary) -> (),
}

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
	f32: CType,
	f64: CType,
	pointer: CType,
	string: CType,
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

export type FfiCallback = {
	ptr: any,
	retType: CType,
	argCount: number,
	isValid: boolean,
	getPtr: (self: FfiCallback) -> any,
}

return {
	open = (nil :: any) :: (path: string) -> NativeLibrary,
	buffer = (nil :: any) :: (size: number) -> Buffer,
	cast = (nil :: any) :: (ptr: any, ctype: CType) -> any,
	string = (nil :: any) :: (ptr: any, len: number?) -> string?,
	null = (nil :: any) :: any,
	isNull = (nil :: any) :: (ptr: any) -> boolean,
	sizeof = (nil :: any) :: (ctype: CType) -> number,
	alignof = (nil :: any) :: (ctype: CType) -> number,
	types = (nil :: any) :: Types,
	cdef = (nil :: any) :: (def: string) -> (),
	callback = (
			nil :: any
		) :: (fn: (...any) -> any, retType: CType, argTypes: { CType }) -> FfiCallback,
}
