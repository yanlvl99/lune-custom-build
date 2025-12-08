<table align="center" width="100%">
  <tr>
    <td align="center" width="40%">
      <img src="assets/logo/tilt-grid.png" alt="Lune Custom Build" width="300" />
    </td>
    <td align="center" width="60%">
      <h1>Lune Custom Build</h1>
      <p>A standalone Luau runtime for <b>backend</b> and <b>game-server</b> development</p>
      <p>
        <a href="https://github.com/yanlvl99/lune-custom-build/releases">Download</a> ·
        <a href="https://yanlvl99.github.io/lune-custom-build-doc/">Documentation</a>
      </p>
    </td>
  </tr>
</table>

---

## What is Lune Custom Build?

Lune Custom Build is a fork of [Lune](https://github.com/lune-org/lune) focused on:

- **Backend Development** - Build web servers, APIs, and microservices
- **Game Server Development** - Create dedicated game servers with UDP/TCP support
- **Extended APIs** - Additional utilities like SQL, extended math, UUIDs
- **Package Manager** - Built-in package management with `--init` and `--install`

## Features

### Package Manager

```bash
lune --init              # Initialize project
lune --install colors    # Install packages from registry
```

### SQL Database

```lua
local sql = require("@lune/sql")
local db = sql.open("data.db")
db:query("SELECT * FROM users WHERE id = ?", { userId })
```

### Extended Globals

```lua
math.clamp(5, 0, 10)     -- 5
math.lerp(0, 100, 0.5)   -- 50
uuid.v4()                -- Random UUID
uuid.v7()                -- Time-ordered UUID
```

### UDP/TCP Sockets

```lua
local net = require("@lune/net")
local socket = net.udp.bind("0.0.0.0:27015")
local server = net.tcp.listen("0.0.0.0:3000")
```

## Installation

Download the latest release from [Releases](https://github.com/yanlvl99/lune-custom-build/releases).

Or build from source:

```bash
cargo build --release
```

## Usage

```bash
lune script.luau         # Run a script
lune --init              # Initialize project
lune --install pkg       # Install package
lune --build script.luau # Build standalone exe
lune --repl              # Interactive REPL
```

## Documentation

Full documentation at: [yanlvl99.github.io/lune-custom-build-doc](https://yanlvl99.github.io/lune-custom-build-doc/)

## Package Registry

Packages are registered in the `/manifest` directory. Each package has a JSON manifest:

```json
{
  "name": "my-package",
  "description": "Package description",
  "repository": "https://github.com/owner/repo.git"
}
```

Version is determined by git tags (semver format).

## License

Licensed under the Mozilla Public License 2.0 - see [LICENSE.txt](LICENSE.txt).

---

Built with ❤️ for the Luau community.
