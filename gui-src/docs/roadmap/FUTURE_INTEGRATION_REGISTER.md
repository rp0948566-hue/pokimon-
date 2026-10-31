# Future Integration Register

Catalogue of future subsystems and their architectural integration status:

| Subsystem | Planned Provider / Technology | Current Status | Integration Boundary |
| :--- | :--- | :--- | :--- |
| **Interactive Map** | MapLibre GL / Vector Tiles | Scaffolded Folder | `apps/desktop/src/features/live-map/` |
| **Voice Copilot** | Local Whisper / Web Audio | Scaffolded Folder | `apps/desktop/src/features/voice-copilot/` |
| **Drone Telemetry** | RTSP / WebRTC stream | Scaffolded Folder | `apps/desktop/src/features/drone-rover/` |
| **Wi-Fi CSI Lab** | Isolated Native Bridge | Scaffolded Folder | `apps/desktop/src/features/experimental-sensing/` |
| **Offline Cache** | SQLite via Tauri Plugin | Scaffolded Folder | `apps/desktop/src/features/offline-sync/` |
| **Incident API** | Rust backend service | Typed Contracts | `packages/contracts/src/incident.ts` |
| **Verified Routing** | Rust routing service | Typed Contracts | `packages/contracts/src/route.ts` |
