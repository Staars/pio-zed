# pioarduino — Zed Extension

PlatformIO Arduino integration for the [Zed editor](https://zed.dev).

Provides syntax highlighting for `platformio.ini` and configures `clangd` for C/C++ IntelliSense in PlatformIO projects.

## Features

- **`platformio.ini` syntax highlighting** — sections, settings, comments, bracket matching
- **C/C++ IntelliSense** — auto-detects PlatformIO environments via `pio project config` and starts `clangd` with `--compile-commands-dir` pointing to `.pio/build/<env>/compile_commands.json`
- **Bundled tasks** — Build, Upload, Clean, Monitor, and Update compiledb — each with automatic PlatformIO Core installation via uv
- **Run buttons** — click any `[env:...]` line in `platformio.ini` to run a task for that environment
- **PlatformIO auto-detection** — checks `$PATH` and `~/.platformio/penv/bin/pio`

## Requirements

- **Zed** — any recent version
- **clangd** — `brew install llvm` (macOS) or your system package manager. Must be on `$PATH`.
- **Rust** (`rustup` + `wasm32-wasip2` target) — only needed for dev extension compilation; not required if the extension is published

PlatformIO Core is auto-installed on first task run via the bundled uv bootstrap — no manual setup needed.

## Installation (Dev Extension)

```bash
git clone https://github.com/christianbaars/pio-zed.git
cd pio-zed
rustup target add wasm32-wasip2
```

Then in Zed:
1. Open Extensions view (`Cmd+Shift+X`)
2. Click **Install Dev Extension** (top right)
3. Select the `pio-zed` directory

The extension replaces the built-in C/C++ language server — clangd is configured automatically when a `platformio.ini` is detected.

## Usage

1. Open a PlatformIO project folder in Zed
2. Open any C/C++ file — the extension finds `pio` on `$PATH` or in `~/.platformio/penv/bin/pio`
3. clangd starts with `--compile-commands-dir` pointing to the first environment that has a `.pio/build/<env>/compile_commands.json`
4. If no `compile_commands.json` exists, the extension runs `pio pkg install` to fetch dependencies
5. Click the ▶ run button next to an `[env:...]` line and pick a task (Build, Upload, etc.) — PlatformIO Core is auto-installed on first use
6. If you still lack IntelliSense after build, run **PIO: Update compiledb for clangd** from the command palette or gutter

### Built-in Tasks

| Task | Command |
|------|---------|
| PIO: Build | `pio run -e <env>` |
| PIO: Upload | `pio run --target upload -e <env>` |
| PIO: Monitor | `pio pkg install -e <env> && pio device monitor` |
| PIO: Clean | `pio run --target clean -e <env>` |
| PIO: Update compiledb for clangd | `pio run --target compiledb -e <env>` |

Each task auto-installs PlatformIO Core (via uv) if not found. Tasks are accessible from the gutter run buttons or `Cmd+Shift+R`.

### Custom Tasks

If you need project-specific tasks, create `.zed/tasks.json` in your project. The custom variable `$ZED_CUSTOM_env_name` holds the matched env section (e.g., `env:uno`):

```json
{
  "label": "PIO: Custom",
  "command": "ENV_NAME=$ZED_CUSTOM_env_name; pio run -e ${ENV_NAME#env:} --target upload --upload-port /dev/ttyUSB0",
  "tags": ["pio-env"]
}
```

## How It Works

On opening a C/C++ file, `language_server_command` runs:
1. Finds the `pio` binary (cached after first lookup)
2. Runs `pio project config --json-output` to discover environments
3. If no environment has a `compile_commands.json`, runs `pio pkg install`
4. Finds `clangd` on `$PATH`
5. Strips xtensa-specific GCC flags (`-mlongcalls`, etc.) from the first found `compile_commands.json` (these cause clangd `Unknown argument` errors)
6. Launches clangd with `--compile-commands-dir` pointing to that environment's build directory

## Restrictions

- **IntelliSense for `.ino` files** — clangd treats `.ino` files as C++, but may not resolve `<Arduino.h>` if the toolchain compiler is not on `$PATH`. Build at least once to generate `compile_commands.json`.
- **Serial monitor** — use Zed's integrated terminal with `pio device monitor` directly
- **Single IntelliSense environment** — clangd is configured for the first environment with a `compile_commands.json`. Building or uploading other environments via the run button still works fine; only IntelliSense follows one env at a time.

## Project Structure

```
pio-zed/
├── extension.toml            # Extension manifest
├── Cargo.toml                # Rust crate config (compiled to WASM)
├── src/lib.rs                # Extension logic (clangd LSP integration)
├── grammars/
│   └── ini.wasm              # tree-sitter INI grammar
├── languages/
│   └── pioini/
│       ├── config.toml       # Language definition
│       ├── highlights.scm    # Tree-sitter highlight queries
│       ├── brackets.scm      # Bracket matching queries
│       ├── runnables.scm     # Run button detection for [env:...]
│       └── tasks.json        # Bundled task definitions
├── extension.wasm            # Compiled extension binary
└── LICENSE                   # MIT
```

## License

MIT
