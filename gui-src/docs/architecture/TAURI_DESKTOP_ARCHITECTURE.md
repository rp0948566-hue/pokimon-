# Tauri Desktop Architecture — PokéRescue Sentinel

## Architectural Overview
Tauri 2 binds the React web frontend (`apps/desktop/dist`) into a native Windows executable or MSI installer.

```
+-----------------------------------------------------------+
| Windows Desktop Application Window                        |
|                                                           |
|  +-----------------------------------------------------+  |
|  | Webview (WebView2 on Windows)                       |  |
|  | React 19 Frontend Shell                             |  |
|  +-----------------------------------------------------+  |
|                            | (IPC Invocation via invoke)  |
|  +-----------------------------------------------------+  |
|  | Native Tauri Host (Rust Shell: src-tauri)           |  |
|  | - Window management & lifecycle                     |  |
|  | - Least-privilege capabilities                      |  |
|  | - Future IPC dispatch to local Rust services         |  |
|  +-----------------------------------------------------+  |
+-----------------------------------------------------------+
```

## Security & Capability Boundaries
- **Permissions**: Defined in `apps/desktop/src-tauri/capabilities/default.json`.
- **Restricted Access**: No unrestricted filesystem, shell execution, or arbitrary network access.
- **CSP**: Strict Content Security Policy configured in `tauri.conf.json`.
