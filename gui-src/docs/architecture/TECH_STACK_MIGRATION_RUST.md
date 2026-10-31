# Technology Stack Change & Migration Notice

## Architectural Pivot Summary
In accordance with the latest user instruction, PokéRescue Sentinel has transitioned from a hybrid React/TypeScript/Tauri monorepo to a **pure Rust application and server architecture**.

---

## Comparison Matrix

| Dimension | Previous Stack | New 100% Rust Stack |
| :--- | :--- | :--- |
| **Primary Language** | TypeScript + Rust | **Rust (100%)** |
| **Runtime Environment** | Node.js v24 + Webview2 | **Native Operating System Binary (.exe)** |
| **Package Management** | pnpm workspace | **Cargo** |
| **Web Server Layer** | Vite development server | **Axum / Tokio Native Async Server** |
| **Static Canvas Delivery** | React TSX components | **Rust Embedded / Zero-Dependency Static Assets** |
| **Memory Footprint** | ~120 MB (Node/Browser) | **< 15 MB (Native Rust Executable)** |
| **Build Artifact** | Multi-file node_modules + bundle | **Single Self-Contained Executable (.exe)** |

---

## Retained Assets & Contracts
1. **Visual Dot-Grid Specifications**: The precise colorimetric metrics (`#D8D8D8` canvas base, `#C4C4C4` micro-dots, 6px pitch) documented in `documents/dotes adn backgroudn colour/` are preserved and directly rendered by the Rust server.
2. **Domain Models**: Data structures (`IncidentSummary`, `RouteRecommendation`, `ResponderUnitSummary`) are converted directly to idiomatic Rust structs with Serde serialization.
3. **Non-Interactive Rule**: The static, magnetic-free requirement remains strictly enforced.
