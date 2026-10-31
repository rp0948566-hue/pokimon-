# 02. Rust Workspace Compatibility

## Compatibility Assessment Date
2026-10-28

## 1. Workspace Topology
- **GUI Workspace**: `/home/claude/Desktop/Hackthon dezine/rust`
  - Package: `retinacare-desktop`
  - Edition: 2021
  - Framework: `eframe = "0.37.0"`, `egui = "0.37.0"`
  - Image handling: `image = "0.25.10"`
- **Medical Backend Workspace**: `/home/claude/Desktop/Dird/native`
  - Workspace root: `/home/claude/Desktop/Dird/native/Cargo.toml`
  - Edition: 2024
  - Members: `crates/retinacare-domain`, `crates/retinacare-persistence`, `crates/retinacare-reports`, `crates/retinacare-ai-bridge`, `crates/retinacare-app`, `crates/retinacare-ui`
  - Pinned runtime: `ort = "=2.0.0-rc.15"`, `opencv = "=0.101.1"`

## 2. Path Dependency Integration
Cargo supports path dependencies pointing to external workspace member crates:
```toml
[dependencies]
retinacare-ai-bridge = { path = "../../Dird/native/crates/retinacare-ai-bridge" }
retinacare-app = { path = "../../Dird/native/crates/retinacare-app" }
```
When Cargo resolves these path dependencies, it traverses up to `/home/claude/Desktop/Dird/native/Cargo.toml` to satisfy `version.workspace = true` and `dependencies.*.workspace = true`.
- Crate version conflicts: Zero. Both workspaces use `image = 0.25.10`, `eframe = 0.37.0`, `sha2 = 0.10`.
- Native build toolchain: System `clang-18`, `libclang-dev`, `libopencv-dev 4.10.0` are active and fully operational.
- Verification status: `cargo check --manifest-path rust/Cargo.toml` completes cleanly with exit code 0.

## 3. Git Branch Isolation and Safety
- The GUI repository (`Hackthon dezine`) remains strictly on branch `dezine`.
- The backend repository (`Dird`) remains strictly on branch `main`.
- Neither repository modifies the other's Git commits or working tree.
- Zero destructive Git operations performed (no force-checkout, no reset, no cherry-pick).
- Code changes remain 100% confined to `/home/claude/Desktop/Hackthon dezine/rust/`.
