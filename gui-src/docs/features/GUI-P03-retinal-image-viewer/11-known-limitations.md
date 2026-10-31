# 11 — Known Limitations: Quad-Panel Workspace

## 1. Backend AI Inference Engine Dependencies
- **HSMoE-AUNet Lesion Segmentation**: The multi-class lesion segmentation inference engine owned by Freebuff (Codex) is not yet connected to the native GUI. Panels 1, 2, and 3 display truthful "Awaiting Freebuff Inference" cards rather than fake lesion masks.
- **SwinHRUNet++ Vessel Caliber**: Vascular segmentation masks require Freebuff's ONNX runtime. When offline, the raw fundus image remains visible in Panel 2 with an unreviewed status.
- **HierarchiRetina Grad-CAM**: Attribution heatmaps require the clinical model execution pipeline. The frontend provides complete blending, opacity, and legend controls awaiting connection to the real tensor pipeline.

## 2. Rendering & Platform Characteristics
- **Single Process Graphics**: All rendering runs via native OpenGL on X11 / Wayland through `eframe` with zero Chromium / WebView overhead.
- **Large Image Ingestion**: High-resolution fundus images (e.g. 4000x3000) are safely scrubbed of metadata and bounded during texture creation. Bounded tiling will be evaluated in Phase 2 if multi-gigapixel whole-slide scans are introduced.
- **Single Examination Scope**: Current workspace displays one active retinal examination at a time. Multi-patient split-screen comparison is deferred to Phase 2.
