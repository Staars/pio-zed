# pioarduino — Zed Extension

PlatformIO Arduino integration for the [Zed editor](https://zed.dev).

Provides syntax highlighting for `platformio.ini` and configures `clangd` for C/C++ IntelliSense in PlatformIO projects.

## Features

- **`platformio.ini` syntax highlighting** — sections, settings, comments, bracket matching
- **C/C++ IntelliSense** — auto-detects PlatformIO environments and starts `clangd` with the correct `--compile-commands-dir` pointing to `.pio/build/<env>/compile_commands.json`
- **PlatformIO auto-detection** — checks both `$PATH` and the default virtual environment location (`~/.platformio/penv/bin/pio`) installed by the official bootstrapper
- **Guided auto-install** — if `pio` is not found, a Zed diagnostic points the user directly to the bundled `PlatformIO: Auto-Install Core` task

## Requirements

- **Zed** — any recent version
- **Rust** — must be installed via [rustup](https://rustup.rs) (required by Zed dev extensions; Homebrew installations won't work)
- **PlatformIO CLI** — either already on `$PATH`, installed in `~/.platformio/penv/` by the official bootstrapper, or installed automatically via the bundled **`PlatformIO: Auto-Install Core`** task (requires Python 3)
- **clangd** — `brew install llvm` (macOS), or your system package manager. Must be on `$PATH`.

## Installation (Dev Extension)

Since this extension is not yet published to the Zed registry, install it as a dev extension:

```bash
git clone https://github.com/christianbaars/pio-zed.git
cd pio-zed
```

Make sure the `wasm32-wasip2` target is installed:

```bash
rustup target add wasm32-wasip2
```

Then in Zed:

1. Open the Extensions view (`Cmd+Shift+X` or `zed: extensions`)
2. Click **Install Dev Extension** (top right)
3. Select the `pio-zed` directory

The extension replaces the built-in C/C++ language server for this project — clangd will be configured automatically when a `platformio.ini` is detected.

## Usage

1. Open a PlatformIO project folder in Zed
2. The extension checks for `pio` in your system `$PATH`, then falls back to `~/.platformio/penv/bin/pio` (the location used by the official bootstrapper)
3. If `pio` is **not found**, a diagnostic notification appears — run `Cmd+Shift+T` and select **`PlatformIO: Auto-Install Core`** to install it automatically
4. Once `pio` is present, the extension configures `clangd` with IntelliSense for your build environment(s)
5. Edit `.ino` or `.cpp` files with full LSP support (completions, go-to-definition, diagnostics)

### PlatformIO Tasks

The extension automatically bundles task definitions for compiling, uploading, and cleaning. It provides **run buttons** in the gutter next to `[env:...]` lines in `platformio.ini`. Clicking one opens a task picker with build/upload/clean commands specifically configured for that environment.

If you wish to override these or define your own custom tasks in your project's `.zed/tasks.json`, you can use the custom variable `$ZED_CUSTOM_env_name` (which holds the matched environment section, e.g., `env:uno`):

```json
[
  {
    "label": "PIO: Build",
    "command": "ENV_NAME=$ZED_CUSTOM_env_name; pio run ${ENV_NAME:+-e} ${ENV_NAME#env:}",
    "tags": ["pio-env"]
  },
  {
    "label": "PIO: Upload",
    "command": "ENV_NAME=$ZED_CUSTOM_env_name; pio run --target upload ${ENV_NAME:+-e} ${ENV_NAME#env:}",
    "tags": ["pio-env"]
  },
  {
    "label": "PIO: Monitor",
    "command": "pio device monitor",
    "tags": ["pio-env"]
  },
  {
    "label": "PIO: Clean",
    "command": "ENV_NAME=$ZED_CUSTOM_env_name; pio run --target clean ${ENV_NAME:+-e} ${ENV_NAME#env:}",
    "tags": ["pio-env"]
  }
]
```

Open tasks with `Cmd+Shift+T` (`task: spawn`) and select the desired command, or click the run button in the gutter.

## How It Works

On opening any C/C++ file, the extension runs `language_server_command`:

1. Checks if `platformio.ini` exists in the project root
2. Parses environment names from `[env:...]` sections
3. For each environment, checks if `.pio/build/<env>/compile_commands.json` exists
4. Launches `clangd` with `--compile-commands-dir` pointing to the first matching build directory

## Restrictions

- **Task templates must be defined manually** in `.zed/tasks.json` (see template above). The extension provides run buttons via `runnables.scm`, but the actual task commands need user configuration until the Zed extension API supports dynamic task generation.
- **PlatformIO CLI must be installed separately** — extensions cannot bundle external tools per Zed policy. Install with `pip install platformio`.
- **clangd must be installed separately** — the extension detects it on `$PATH`.
- **Rust must be installed via rustup**, not Homebrew. This is a Zed requirement for compiling WASM extensions.
- **Serial monitor** — use Zed's integrated terminal with `pio device monitor` directly.

## Project Structure

```
pio-zed/
├── extension.toml            # Extension manifest
├── Cargo.toml                # Rust crate config (compiled to WASM)
├── src/lib.rs                # Extension logic (clangd LSP integration)
├── languages/
│   └── pioini/
│       ├── config.toml       # Language definition
│       ├── highlights.scm    # Tree-sitter highlight queries
│       ├── brackets.scm      # Bracket matching queries
│       └── runnables.scm     # Runnable detection for [env:*] sections
├── tasks.example.json        # Example task templates (copy to .zed/tasks.json)
└── LICENSE                   # MIT
```

## License

MIT
