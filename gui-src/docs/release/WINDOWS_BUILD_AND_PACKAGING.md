# Windows Build and Packaging Guide

## Prerequisites for Windows Packaging
To produce a native Windows executable (`.exe`) or Windows Installer (`.msi`):

1. **Operating System**: Windows 10/11 (or cross-compilation pipeline with MSVC toolchain).
2. **Build Tools**:
   - Visual Studio 2022 C++ Build Tools (including Windows 10/11 SDK).
   - Node.js >= 20 and pnpm.
   - Rust toolchain (`rustup target add x86_64-pc-windows-msvc`).
   - WebView2 Runtime (pre-installed on Windows 10/11).
   - WiX Toolset v3 (required by Tauri v1/v2 for generating `.msi` installers).

## Packaging Commands
```powershell
# 1. Install dependencies
pnpm install

# 2. Build contracts and frontend
pnpm build

# 3. Run Tauri desktop bundler
pnpm --filter @pokerescue/desktop tauri build
```

## Output Locations
- Standard MSI: `apps/desktop/src-tauri/target/release/bundle/msi/`
- Portable EXE: `apps/desktop/src-tauri/target/release/bundle/nsis/` or raw release executable.

> [!NOTE]
> In Linux/development environments without the MSVC toolchain, running `tauri build` for Windows targets is blocked until built on a Windows runner or container.
