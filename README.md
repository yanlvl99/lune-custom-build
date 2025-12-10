<div align="center">
  <img src="assets/logo/tilt-grid.png" alt="Lune Custom Build" width="200" />
  <h1>Lune Custom Build</h1>
  <p><strong>A standalone Luau runtime for backend and game-server development</strong></p>
  
  [![Release](https://img.shields.io/github/v/release/yanlvl99/lune-custom-build?style=flat-square&color=C848E9)](https://github.com/yanlvl99/lune-custom-build/releases)
  [![License](https://img.shields.io/github/license/yanlvl99/lune-custom-build?style=flat-square&color=E168FF)](LICENSE.txt)
  [![Downloads](https://img.shields.io/github/downloads/yanlvl99/lune-custom-build/total?style=flat-square&color=9D4EDD)](https://github.com/yanlvl99/lune-custom-build/releases)
  
  [📦 Download](https://github.com/yanlvl99/lune-custom-build/releases) · 
  [📚 Documentation](https://yanlvl99.github.io/lune-custom-build-doc/) ·
  [🚀 Getting Started](https://yanlvl99.github.io/lune-custom-build-doc/getting-started/1-installation/)
</div>

---

## ✨ Features

| Feature                 | Description                                     |
| ----------------------- | ----------------------------------------------- |
| 📦 **Package Manager**  | Built-in `--init` and `--install` commands      |
| 🗄️ **SQL Database**     | SQLite support via `@lune/sql`                  |
| 🔌 **UDP/TCP Sockets**  | Low-level networking for game servers           |
| 🔧 **Extended Globals** | `math.clamp`, `math.lerp`, `uuid.v4`, `uuid.v7` |
| 🔗 **FFI**              | Call native C libraries directly                |
| 🏗️ **Build to EXE**     | Compile scripts to standalone executables       |

## 🚀 Quick Start

### Installation

**Windows (PowerShell):**

```powershell
irm https://yanlvl99.github.io/lune-custom-build-doc/install.ps1 | iex
```

**Manual Download:**
Download from [Releases](https://github.com/yanlvl99/lune-custom-build/releases) and add to PATH.

**From Source:**

```bash
cargo build --release
```

### Usage

```bash
lune script.luau           # Run a script
lune --init                # Initialize project with LSP support
lune --install colors      # Install package from registry
lune --build script.luau   # Build standalone executable
lune --repl                # Interactive REPL
```

## 📝 Examples

### Hello World

```lua
print("Hello from Lune!")
```

### HTTP Server

```lua
local net = require("@lune/net")

net.serve(8080, function(request)
    return {
        status = 200,
        body = "Hello, World!"
    }
end)
```

### SQL Database

```lua
local sql = require("@lune/sql")
local db = sql.open("app.db")

db:execute("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)")
db:execute("INSERT INTO users (name) VALUES (?)", "John")

local users = db:query("SELECT * FROM users")
for _, user in users do
    print(user.name)
end
```

### UDP Game Server

```lua
local net = require("@lune/net")
local socket = net.udp.bind("0.0.0.0:27015")

while true do
    local data, addr = socket:recv()
    print("Received from", addr, ":", data)
    socket:send(addr, "ACK")
end
```

### FFI (Native Libraries)

```lua
local ffi = require("@lune/ffi")
local lib = ffi.open("user32")

local MessageBoxA = lib:fn("MessageBoxA", ffi.types.i32, {
    ffi.types.ptr, ffi.types.ptr, ffi.types.ptr, ffi.types.u32
})

MessageBoxA(nil, "Hello FFI!", "Lune", 0)
```

## 📦 Package Manager

Initialize a new project:

```bash
lune --init
```

Install packages:

```bash
lune --install colors networking discord
```

Update packages:

```bash
lune --updpkg
```

## 📚 Documentation

Full documentation available at: **[yanlvl99.github.io/lune-custom-build-doc](https://yanlvl99.github.io/lune-custom-build-doc/)**

- [Getting Started](https://yanlvl99.github.io/lune-custom-build-doc/getting-started/1-installation/)
- [API Reference](https://yanlvl99.github.io/lune-custom-build-doc/api-reference/fs/)
- [The Lune Book](https://yanlvl99.github.io/lune-custom-build-doc/the-book/1-hello-lune/)

## 🤝 Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## 📄 License

Licensed under the [Mozilla Public License 2.0](LICENSE.txt).

---

<div align="center">
  <sub>Built with ❤️ for the Luau community</sub>
</div>
