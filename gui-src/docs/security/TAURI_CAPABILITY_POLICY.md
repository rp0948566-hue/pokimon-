# Tauri Capability and Permissions Policy

## Least-Privilege Standard
Tauri 2 applications use capability files under `src-tauri/capabilities/` to control webview permissions.

## Allowed Capabilities (Minimal Scaffold)
- Window operations: Resize, minimize, focus, fullscreen toggle.
- Dialog: System file picker for report export (read/write to selected directory only).
- Store / Local KV: App settings persistence.

## Prohibited Capabilities
- Unrestricted shell command execution (`shell:allow-all`).
- Raw arbitrary network listening (`http:default` without domain whitelist).
- Unrestricted filesystem access (`fs:allow-all`).
