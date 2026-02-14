# Build Scripts Reference

This document describes every PowerShell (`.ps1`) and batch (`.bat`) script in
the project root. These scripts were created during initial development on a
Windows machine where the Rust toolchain required manual environment setup.
They are **not required for CI/CD** — the GitHub Actions workflows handle
environment setup automatically on each platform.

---

## Table of Contents

1. [Overview](#overview)
2. [Environment Architecture](#environment-architecture)
3. [Script Reference](#script-reference)
   - [cargo_build_full.ps1](#cargo_build_fullps1)
   - [build_with_sdk.ps1](#build_with_sdkps1)
   - [run_cargo_test.ps1](#run_cargo_testps1)
   - [build_test.bat](#build_testbat)
   - [check_sdk.ps1](#check_sdkps1)
   - [get_xwin.ps1](#get_xwinps1)
   - [get_xwin2.ps1](#get_xwin2ps1)
   - [run_xwin.ps1](#run_xwinps1)
   - [check_xwin_sdk.ps1](#check_xwin_sdkps1)
   - [get_winsdk.ps1](#get_windsdkps1)
   - [install_sdk.ps1](#install_sdkps1)
   - [install_mingw.ps1](#install_mingwps1)
   - [setup_mingw.ps1](#setup_mingwps1)
   - [install_xwin.ps1](#install_xwinps1)
   - [test_lld.ps1](#test_lldps1)
4. [CI/CD vs Local Development](#cicd-vs-local-development)
5. [Recommended Local Workflow](#recommended-local-workflow)

---

## Overview

The KVM-Over-IP Rust workspace (`src/`) contains crates that depend on
Windows-specific APIs (the `windows` crate for `WH_MOUSE_LL`/`WH_KEYBOARD_LL`
hooks in `kvm-master`, `SendInput` in `kvm-client`). On a developer machine
that does **not** have Visual Studio's C++ workload fully installed, the MSVC
linker cannot find the Windows SDK import libraries (`.lib` files such as
`kernel32.lib`, `user32.lib`, etc.).

The scripts in this directory were written to work around this problem by:

1. Downloading the Windows SDK libraries through `xwin` (a cross-platform
   Windows SDK downloader).
2. Manually setting the `LIB` and `INCLUDE` environment variables to point
   Cargo/MSVC at those libraries before invoking `cargo`.
3. Alternatively, initialising Visual Studio's developer environment by
   sourcing `VsDevCmd.bat` before invoking `cargo`.

---

## Environment Architecture

```
Developer machine (Windows)
│
├── cargo (Rust compiler + linker driver)
│       │
│       └── links against → Windows SDK .lib files
│                              │
│                   One of three sources:
│                   A) Visual Studio C++ workload (preferred)
│                      %LIB% set by vcvars64.bat / VsDevCmd.bat
│                   B) xwin-downloaded SDK stubs
│                      %TEMP%\xwin_sdk\sdk\lib\um\x86_64\*.Lib
│                   C) MinGW-w64 (alternative, not fully supported)
│
└── src\  (Cargo workspace — Cargo.toml lives here, NOT in project root)
```

---

## Script Reference

---

### cargo_build_full.ps1

**Purpose:** Invoke `cargo` with the xwin SDK libraries already on `%LIB%`.
This is the primary day-to-day build script for developers who do not have a
complete Visual Studio installation but **do** have the xwin SDK already
downloaded (see `get_xwin2.ps1`).

**Parameters:**

| Parameter    | Type   | Default              | Description                          |
|--------------|--------|----------------------|--------------------------------------|
| `$CargoArgs` | String | `build --workspace`  | Arguments forwarded verbatim to cargo |

**Usage examples:**

```powershell
# Default: build the whole workspace in debug mode
.\cargo_build_full.ps1

# Build in release mode
.\cargo_build_full.ps1 -CargoArgs "build --workspace --release"

# Run tests
.\cargo_build_full.ps1 -CargoArgs "test --workspace"
```

**What it does internally:**

1. Hardcodes the paths to the xwin SDK libraries and the MSVC compiler
   binaries (`Hostx64\x64`).
2. Constructs a semicolon-separated `LIB` string pointing at:
   - `%TEMP%\xwin_sdk\sdk\lib\um\x86_64`  — Windows user-mode import libs
   - `%TEMP%\xwin_sdk\sdk\lib\ucrt\x86_64` — Universal CRT libs
   - `%TEMP%\xwin_sdk\crt\lib\x86_64`     — xwin CRT libs
   - MSVC's own `lib\x64` directory
3. Prepends the MSVC `bin\Hostx64\x64` directory to `PATH` so `link.exe` is
   found.
4. Changes to `src\` and runs `cargo` with the supplied arguments.

**Pre-requisites:**

- Visual Studio 2022 Community (or Build Tools) installed at the default path.
- xwin SDK downloaded by `get_xwin2.ps1` (output at `%TEMP%\xwin_sdk`).

**Limitations:**

- Hardcodes the MSVC toolchain version `14.44.35207`. If Visual Studio is
  updated this path will change and the script will stop working.
- Paths are absolute and specific to the developer's machine
  (`C:\Users\jerry\...`). Other developers must update these paths.

---

### build_with_sdk.ps1

**Purpose:** Identical in structure to `cargo_build_full.ps1` but also sets
the `INCLUDE` environment variable so that C header files can be located.
Some Rust crates use `bindgen` or include C headers at build time; without
`INCLUDE` the build may fail with "cannot open include file" errors.

**Parameters:**

| Parameter    | Type   | Default             | Description                          |
|--------------|--------|---------------------|--------------------------------------|
| `$CargoArgs` | String | `build --workspace` | Arguments forwarded verbatim to cargo |

**Usage:**

```powershell
.\build_with_sdk.ps1
.\build_with_sdk.ps1 -CargoArgs "build --workspace --release"
```

**Additional environment variables set (beyond cargo_build_full.ps1):**

```
INCLUDE = <MSVC_include>;<xwin_sdk_include>\ucrt;<xwin_sdk_include>\shared;
          <xwin_sdk_include>\um;<xwin_sdk_include>\winrt
```

**When to use this instead of cargo_build_full.ps1:**
Use `build_with_sdk.ps1` if you encounter errors such as:
- `cannot open include file: 'windows.h'`
- `error[E0425]: cannot find function ... in ...`
  caused by a `build.rs` script that invokes a C compiler.

---

### run_cargo_test.ps1

**Purpose:** Runs `cargo` inside a properly initialised Visual Studio developer
shell by sourcing `VsDevCmd.bat`. This is the most reliable approach for
developers who **do** have a full Visual Studio installation because it sets
up every environment variable (LIB, INCLUDE, PATH, WindowsSdkDir, etc.)
exactly as Visual Studio itself would.

**Parameters:**

| Parameter    | Type   | Default             | Description                          |
|--------------|--------|---------------------|--------------------------------------|
| `$CargoArgs` | String | `build --workspace` | Arguments forwarded verbatim to cargo |

**Usage:**

```powershell
# Run the full test suite
.\run_cargo_test.ps1 -CargoArgs "test --workspace"

# Build in release mode
.\run_cargo_test.ps1 -CargoArgs "build --workspace --release"
```

**How it works:**

1. Searches common Visual Studio installation paths for `VsDevCmd.bat`.
2. Creates a temporary `.bat` file that:
   a. Calls `VsDevCmd.bat -arch=x64 -no_logo` to initialise the developer
      environment.
   b. Changes to the `src\` workspace directory.
   c. Runs `cargo` with the supplied arguments.
3. Launches `cmd.exe` to execute the temporary `.bat` file.
4. Cleans up the temporary file and exits with `cargo`'s exit code.

**Pre-requisites:**

- Visual Studio 2022 Community or Build Tools installed with the
  "Desktop development with C++" workload.

---

### build_test.bat

**Purpose:** A minimal batch file used to verify that the Visual Studio
developer environment is configured correctly. It initialises `vcvars64.bat`
and then prints key environment variables (`LIB`, `VCINSTALLDIR`,
`WindowsSdkDir`, `WindowsSdkVersion`) so you can confirm the Windows SDK is
visible to the linker.

**Usage:**

```batch
build_test.bat
```

**Expected output (example):**

```
LIB=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\...\lib\x64;...
WindowsSdkDir=C:\Program Files (x86)\Windows Kits\10\
WindowsSdkVersion=10.0.22621.0\
```

If `LIB` is empty or `WindowsSdkDir` is not set, the Windows SDK component is
missing from your Visual Studio installation. Run `install_sdk.ps1` or
re-run the VS installer and add the "Windows 11 SDK" component.

---

### check_sdk.ps1

**Purpose:** Diagnoses the Visual Studio and Windows SDK installation by:

1. Running `vswhere.exe` to list all installed Visual Studio products and
   their components in JSON format.
2. Reading the Windows Registry key
   `HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots` to find the
   installed Windows SDK versions and their root directories.

**Usage:**

```powershell
.\check_sdk.ps1
```

**When to use:** Run this first if `cargo build` fails with linker errors
such as `cannot find -lkernel32` or `error LNK1104: cannot open file
'kernel32.lib'`. The output tells you which Visual Studio workloads and
SDK versions are installed so you can identify what is missing.

---

### get_xwin.ps1

**Purpose:** First-generation script that downloads and runs
[xwin](https://github.com/Jake-Shadle/xwin) to obtain the Windows SDK library
stubs. `xwin` is a cross-platform tool that downloads the Windows SDK from
Microsoft's servers and reorganises it into a cross-compilation-friendly
layout.

**Note:** This script has been superseded by `get_xwin2.ps1`, which uses an
absolute path to `C:\Windows\System32\tar.exe` for more reliable extraction
on Windows. Prefer `get_xwin2.ps1`.

**Output directory:** `%TEMP%\xwin_sdk`

---

### get_xwin2.ps1

**Purpose:** Improved version of `get_xwin.ps1`. Downloads xwin 0.8.0 from
GitHub, extracts it using `C:\Windows\System32\tar.exe`, and runs
`xwin splat` to download and lay out the Windows SDK libraries.

**Usage:**

```powershell
# Run once to download the SDK. Subsequent runs are skipped if already done.
.\get_xwin2.ps1
```

**What it produces in `%TEMP%\xwin_sdk`:**

```
xwin_sdk\
  sdk\
    lib\
      um\x86_64\     <- Windows user-mode import libs (kernel32.Lib, user32.Lib, etc.)
      ucrt\x86_64\   <- Universal CRT import libs
    include\
      10.0.26100\
        um\          <- Win32 API headers
        shared\      <- Shared headers
        ucrt\        <- C runtime headers
  crt\
    lib\x86_64\      <- MSVC CRT static libs
```

**Idempotent:** If `xwin_sdk\sdk\lib\um\x86_64\kernel32.lib` already exists,
the script exits immediately without re-downloading anything.

**Pre-requisites:**

- Internet access to `github.com` and Microsoft's SDK distribution servers.
- Windows 10 version 1803 or later (for the built-in `tar.exe`).

---

### run_xwin.ps1

**Purpose:** Runs the already-downloaded `xwin.exe` to execute the `splat`
subcommand, which downloads the actual SDK content and organises it under
`%TEMP%\xwin_sdk`. This is a helper called internally after `get_xwin2.ps1`
extracts `xwin.exe` but before the SDK download completes.

**Usage:**

```powershell
# Only needed if get_xwin2.ps1 extracted xwin but did not complete the splat step
.\run_xwin.ps1
```

---

### check_xwin_sdk.ps1

**Purpose:** Verifies that the xwin SDK download was successful by checking
for the presence of several key `.Lib` files. Prints a `True/False` status
for each file.

**Files checked:**

| File                                              | Purpose               |
|---------------------------------------------------|-----------------------|
| `xwin_sdk\sdk\lib\um\x86_64\kernel32.Lib`        | Core Windows API      |
| `xwin_sdk\sdk\lib\um\x86_64\ntdll.Lib`           | Native API            |
| `xwin_sdk\sdk\lib\um\x86_64\ws2_32.Lib`          | Winsock (networking)  |
| `xwin_sdk\sdk\lib\um\x86_64\userenv.Lib`         | User environment API  |
| `xwin_sdk\sdk\lib\um\x86_64\dbghelp.Lib`         | Debug helper API      |
| `xwin_sdk\crt\lib\x86_64\msvcrt.Lib`             | MSVC runtime          |
| `xwin_sdk\crt\lib\x86_64\libcmt.Lib`             | Static C runtime      |

**Usage:**

```powershell
.\check_xwin_sdk.ps1
```

Run this after `get_xwin2.ps1` to confirm the download succeeded before
attempting a build.

---

### get_winsdk.ps1

**Purpose:** An exploratory script that investigated two alternative
approaches for obtaining the Windows SDK:

1. Downloading it as NuGet packages from `globalcdn.nuget.org`.
2. Using the GitHub API to find and download the latest `xwin` release.

**Status:** This script was written during investigation of SDK acquisition
methods and is not part of the standard build workflow. It is retained for
reference only. Use `get_xwin2.ps1` instead.

---

### install_sdk.ps1

**Purpose:** Attempts to install the Windows 11 SDK component into an existing
Visual Studio 2022 Community installation using the Visual Studio installer
command-line interface.

**Usage:**

```powershell
# Run with administrator rights for best results
.\install_sdk.ps1
```

**What it does:**

1. Locates `vs_installer.exe` at its default installation path.
2. Runs a quiet, non-interactive installer command to add the component
   `Microsoft.VisualStudio.Component.Windows11SDK.22621`.
3. After the installer finishes, reads the Windows Registry to confirm the
   SDK is now registered.

**Pre-requisites:**

- Visual Studio 2022 Community (or Professional/Enterprise) already installed.
- Administrator rights may be required for the VS installer to succeed.

**Alternative:** If you do not have Visual Studio installed, use
`get_xwin2.ps1` instead to obtain SDK library stubs without a full VS
installation.

---

### install_mingw.ps1

**Purpose:** Downloads and installs a portable MinGW-w64 (GCC for Windows)
toolchain from [winlibs.com](https://winlibs.com). This was investigated as
an alternative to the MSVC linker for building the Rust workspace on Windows
without a Visual Studio installation.

**Status:** MinGW-w64 is not the recommended linker for this project because
the `windows` crate (used by `kvm-master` and `kvm-client`) is designed to
link against MSVC-compatible import libraries. Using MinGW/GCC as the linker
requires additional configuration and may cause ABI compatibility issues.
This script is retained for reference.

**Output directory:** `%USERPROFILE%\mingw64`

---

### setup_mingw.ps1

**Purpose:** Companion to `install_mingw.ps1`. This script downloads the
MinGW-w64 `.7z` archive (the download portion; extraction was handled
separately). Superseded by `install_mingw.ps1`.

**Status:** Retained for reference only. Not used in the standard workflow.

---

### install_xwin.ps1

**Purpose:** Downloads a pre-built `cargo-xwin` binary from the
`rust-cross/cargo-xwin` GitHub repository. `cargo-xwin` is a Cargo wrapper
that uses `xwin` internally to cross-compile Rust code targeting Windows from
Linux or macOS — essentially the CI-native approach for cross-compilation.

**Status:** This approach was investigated but not adopted for local
development (on Windows, native compilation is simpler). The script was
retained for reference. In CI, the GitHub Actions runners have the full MSVC
toolchain pre-installed, so neither `cargo-xwin` nor `xwin` is needed in the
CI workflows.

---

### test_lld.ps1

**Purpose:** A diagnostic script that tests whether the LLVM `clang-cl`
compiler and `lld-link` linker are correctly installed and functional by
compiling and linking a trivial C program.

**Usage:**

```powershell
.\test_lld.ps1
```

**What it tests:**

1. Compiles a one-line `#include <windows.h>; int main() { return 0; }` C
   source file using `clang-cl.exe`.
2. Links the resulting `.obj` file using `lld-link.exe` to produce a `.exe`.
3. Reports success or failure at each step.

**Pre-requisites:**

- LLVM installed at `C:\Program Files\LLVM\` (available via
  `winget install LLVM.LLVM`).
- Windows SDK headers accessible (e.g., via `%INCLUDE%`).

**When to use:** Use this script to confirm that the LLVM/Clang toolchain is
working before attempting to configure Rust to use `lld-link` as its linker.

---

## CI/CD vs Local Development

| Concern                    | CI (GitHub Actions)                                    | Local (Windows developer)                     |
|----------------------------|--------------------------------------------------------|-----------------------------------------------|
| Rust toolchain             | Installed by `dtolnay/rust-toolchain@stable`           | Pre-installed via `rustup`                    |
| Windows SDK                | Pre-installed on `windows-latest` runner image         | Manual via VS installer or `get_xwin2.ps1`    |
| Linux system libs (X11)    | Installed via `apt-get` step in workflow               | Not applicable (build on Linux VM if needed)  |
| macOS system libs          | Included in macOS SDK on runner image                  | Included in Xcode Command Line Tools          |
| Cargo workspace location   | `src/` — passed via `--manifest-path src/Cargo.toml`  | `cd src && cargo build` or use `.ps1` scripts |
| npm dependencies           | Installed by `npm ci` step                             | `npm install` or `npm ci`                     |

---

## Recommended Local Workflow

For a Windows developer with Visual Studio 2022 Community + C++ workload:

```powershell
# 1. Verify the SDK is visible
.\build_test.bat

# 2. Build the workspace (uses VsDevCmd.bat for the most reliable environment)
.\run_cargo_test.ps1 -CargoArgs "build --workspace"

# 3. Run all tests
.\run_cargo_test.ps1 -CargoArgs "test --workspace"
```

For a Windows developer **without** Visual Studio (using xwin SDK stubs):

```powershell
# 1. Download the Windows SDK stubs (one-time setup, ~500 MB download)
.\get_xwin2.ps1

# 2. Verify the SDK downloaded correctly
.\check_xwin_sdk.ps1

# 3. Build the workspace
.\cargo_build_full.ps1

# 4. Run all tests
.\cargo_build_full.ps1 -CargoArgs "test --workspace"
```

For UI development (any platform with Node.js 20+):

```bash
# ui-master
cd src/packages/ui-master
npm ci
npm run lint
npm run test:coverage
npx tsc --noEmit

# ui-client
cd src/packages/ui-client
npm ci
npm run lint
npm run test:coverage
npx tsc --noEmit
```
