# Windows Development

Build and run Rivet natively on Windows, without WSL. Run all commands in PowerShell from the repository root.

## Prerequisites

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install Rustlang.Rustup
winget install LLVM.LLVM
winget install Kitware.CMake
winget install NASM.NASM
winget install OpenJS.NodeJS.LTS
```

NASM does not add itself to `PATH`, and `bindgen` needs to find `libclang`:

```powershell
[Environment]::SetEnvironmentVariable("Path", [Environment]::GetEnvironmentVariable("Path", "User") + ";$env:LOCALAPPDATA\bin\NASM", "User")
[Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "$env:ProgramFiles\LLVM\bin", "User")
```

Open a new terminal so the changes apply.

## Build

```powershell
npm install --global pnpm
pnpm install
pnpm -F @rivetkit/engine-frontend build:engine
cargo build -p rivet-engine
$env:SKIP_WASM_BUILD = "1"
pnpm build -F rivetkit -F @rivetkit/react
```

`build:engine` builds the dashboard that the engine embeds, so build it before the engine. `SKIP_WASM_BUILD` skips the wasm runtime, which this guide does not cover.

## Run

The published Windows engine package does not run yet, so point RivetKit at the locally built engine:

```powershell
$env:RIVET_ENGINE_BINARY_PATH = "$PWD\target\debug\rivet-engine.exe"
cd examples\hello-world
pnpm dev
```

Open http://localhost:5173 for the example and http://127.0.0.1:6420/ui/ for the dashboard.
