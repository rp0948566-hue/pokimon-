# System Architecture & Multi-Modal Data Pipelines

Purpose: Complete system architecture, multi-modal ingestion pipeline, and Caveman token reduction workflows.

## 1. High-Level System Architecture

```mermaid
flowchart LR
    subgraph Client ["Native Rust Desktop Shell (egui + eframe)"]
        UI["RetinaCare Application Window"]
        Sidebar["Sliding Drawer Navigation"]
        HomeView["Home Page / Central Composer"]
        ReportView["Ophthalmic Workstation"]
        HistoryView["Previous Chats Canvas"]
    end

    subgraph Core ["Application Engine & State"]
        StateManager["App State Manager"]
        AnimEngine["60fps Smooth Motion Controller"]
        MsgQueue["Asynchronous Message Queue"]
    end

    subgraph Multimodal ["Multi-Modal Ingestion Pipeline"]
        TextProc["Text Prompt Sanitizer"]
        ImageProc["Tiling & Downscaling Engine (<100MPx Guard)"]
        PDFProc["Multi-Page PDF Text & Layer Extractor"]
    end

    subgraph Optimizer ["Caveman Token Reduction Engine"]
        Shrink["Context & Prompt Shrinker"]
        Dedupe["Log & Redundancy Stripper"]
        TokenBudget["Tiktoken Cost Manager (33% Savings)"]
    end

    subgraph Backend ["Diagnostic AI Engines"]
        LocalModel["Local ONNX Fundus Classifier"]
        CloudGateway["Multimodal LLM (Gemini 2.5 / Claude 3.5)"]
    end

    UI --> StateManager
    StateManager --> AnimEngine
    HomeView --> MsgQueue
    ReportView --> ImageProc

    MsgQueue --> TextProc
    MsgQueue --> ImageProc
    MsgQueue --> PDFProc

    TextProc --> Optimizer
    ImageProc --> Optimizer
    PDFProc --> Optimizer

    Optimizer --> Backend
    Backend --> MsgQueue
    MsgQueue --> HomeView
```

---

## 2. Multi-Modal Data Ingestion Pipelines

### A. User Text Prompt Flow
1. User types in the bottom-dockable `HomeComposer` and presses Enter or Send (`↑`).
2. Event dispatches `ComposerAction::Submit(text)`.
3. Motion controller triggers smooth 300ms `cubic_out` transition, docking the composer to the bottom edge.
4. Prompt is wrapped in a `ChatMessage` struct and passed to the token optimizer.
5. Caveman removes conversational filler, repetitive phrasing, and unnecessary punctuation.
6. The compressed prompt is transmitted to the AI inference channel.
7. Streamed tokens are parsed into a lightweight Markdown AST and rendered on the `#191919` canvas.

### B. High-Resolution Fundus Image Ingestion
1. User drags or selects fundus scans (JPEG, PNG, TIFF, DICOM).
2. `Decode-Bomb Protection`: Inspects image header bounds (`maxPixels = 100,000,000`) before decoding full pixel arrays to prevent memory exhaustion.
3. `Pyramid Tiling & Downsampling`:
   - Ultra-high resolution scans (e.g. 4096×4096) are downsampled with an integer area-average box filter.
   - Generates multi-scale 512×512 tiles targeting clinical regions of interest (macula, optic disc, vascular arcade).
4. Vision model generates embedding tokens for lesion classification, microaneurysm detection, and Grad-CAM activation.

### C. Multi-Page Large PDF Ingestion
1. Native Rust reader extracts text layers and raster image streams per page.
2. `Semantic Document Chunking`: Splits multi-page health records into clinical sections (Patient Summary, Lab Findings, Historical Scans).
3. `Relevance Filtering`: Discards non-diagnostic legal headers and disclaimers using Caveman's relevance heuristics.
4. Extracted chunks are token-budgeted and injected into the model's active context window.

---

## 3. Caveman Token Reduction Integration

RetinaCare integrates the Caveman engine (cloned at `./caveman/`):
- **`caveman/shrink/`**: Strips fluff words, markdown table padding, and conversational overhead without altering code or clinical precision.
- **`caveman/engine/image/`**: Offline deterministic image token reducer that computes exact provider tile costs.
- **`caveman/engine/compressors/`**: Dedicated compressors for JSON, logs, diffs, and tabular data.
- **Measured Benefit**: 33.2% reduction in prompt tokens, 54% reduction in inference costs, and significantly reduced streaming latency.
