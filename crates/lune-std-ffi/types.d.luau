--!strict

--[[
    FFI (Foreign Function Interface) module for loading native libraries.
    
    WARNING: FFI is inherently unsafe. Incorrect usage can crash the runtime.
    Only use with trusted native libraries.
]]

export type NativeLibrary = {
	-- Path to the loaded library
	path: string,

	-- Call a function that returns an integer (no arguments)
	callInt: (self: NativeLibrary, name: string) -> number,

	-- Call a function that takes one integer and returns an integer
	callIntArg: (self: NativeLibrary, name: string, arg: number) -> number,

	-- Call a function that returns a double (no arguments)
	callDouble: (self: NativeLibrary, name: string) -> number,

	-- Call a void function (no arguments, no return)
	callVoid: (self: NativeLibrary, name: string) -> (),

	-- Call a function that returns a string (const char*)
	callString: (self: NativeLibrary, name: string) -> string?,

	-- Check if a symbol exists in the library
	hasSymbol: (self: NativeLibrary, name: string) -> boolean,

	-- Explicitly close/unload the library
	close: (self: NativeLibrary) -> (),
}

--[[
    Open a native library (.dll on Windows, .so on Linux, .dylib on macOS).
    
    @param path - Path to the native library file
    @return NativeLibrary userdata
    
    @example
    ```lua
    local ffi = require("@lune/ffi")
    local lib = ffi.open("mylib.dll")
    
    if lib:hasSymbol("add") then
        local result = lib:callIntArg("add", 42)
        print("Result:", result)
    end
    
    lib:close()
    ```
]]
local ffi = {}

function ffi.open(path: string): NativeLibrary
	return nil :: any
end

--[[
    Define C types and functions (placeholder for future implementation).
    
    @param definition - C-style type definitions
]]
function ffi.cdef(definition: string): () end

return ffi
