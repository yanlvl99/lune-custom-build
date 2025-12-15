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

## What is Lune?

Lune is a lightweight, high-performance runtime for **Luau** (the language used by Roblox). It allows you to write scripts, servers, and tools using the language you already love, but running standalone on your machine or server—independent of Roblox.

## ✨ Features

Lune comes batteries-included with everything you need for robust development:

### 🛠️ Runtime & Tooling

- **Package Manager**: Built-in `lune --install` and `lune --init`.
- **Standalone Binaries**: Compile your scripts into `.exe` or binary files with `lune --build`.
- **Interactive REPL**: Test code quickly with `lune --repl`.
- **Task Scheduler**: Async/await support with `task.spawn`, `task.delay`, `task.wait`.

### 📚 Standard Library

- **Networking**: Full TCP/UDP sockets and HTTP/WebSocket clients/servers (`@lune/net`).
- **File System**: Read/write files and directories asynchronously (`@lune/fs`).
- **Process Control**: Spawn and control child processes (`@lune/process`).
- **Database**: Built-in SQLite support (`@lune/sql`).
- **Foreign Function Interface (FFI)**: Zero-copy access to native C libraries (`@lune/ffi`).

### 📦 Data & Utilities

- **Serialization**: JSON, YAML, and TOML parsing/encoding (`@lune/serde`).
- **Roblox Types**: Native support for CFrame, Vector3, Color3, etc. (`@lune/roblox`).
- **Cryptography**: Hashing, encoding, and UUID generation.
- **Console**: Rich terminal output with colors and formatting (`@lune/stdio`).

---

## � Quick Start

### Installation (Windows)

```powershell
irm https://raw.githubusercontent.com/yanlvl99/lune-custom-build/main/installer/install.ps1 | iex
```

### Other Platforms

Download the latest binary for Linux or macOS from the **[Releases Page](https://github.com/yanlvl99/lune-custom-build/releases)**.

---

## 📚 Documentation

Detailed guides, API references, and tutorials are available at:

### [👉 yanlvl99.github.io/lune-custom-build-doc](https://yanlvl99.github.io/lune-custom-build-doc/)

---

<div align="center">
  <sub>Built with ❤️ for the Luau community</sub>
  <br>
  <sub>Licensed under MPL 2.0</sub>
</div>
