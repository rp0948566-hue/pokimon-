# Open Technical Decisions

The following architectural choices are explicitly marked as **OPEN** pending user input and empirical testing:

| Decision ID | Topic | Current Candidate | Status | Reason / Unresolved Factor |
| :--- | :--- | :--- | :--- | :--- |
| **OTD-001** | Visual Styling Engine | CSS Modules vs Tailwind vs Vanilla CSS | OPEN | Awaiting user's visual design system. |
| **OTD-002** | Global State Management | Zustand vs React Context | OPEN | Kept feature-local until complex state is proven. |
| **OTD-003** | Map Rendering Engine | MapLibre GL vs Leaflet | OPEN | Dependent on offline raster/vector tile formats. |
| **OTD-004** | Runtime Validation | Zod vs native TypeScript type guards | OPEN | To be evaluated based on contract volume. |
| **OTD-005** | Offline Database | IndexedDB vs SQLite via Tauri plugin | OPEN | Awaiting local storage volume and sync protocol. |
