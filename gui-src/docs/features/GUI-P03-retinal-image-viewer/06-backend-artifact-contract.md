# 06 — Backend Artifact Contract

## Typed Rust Data Contracts for Freebuff Integration

This contract defines the frontend-facing structures that Freebuff's backend inference pipeline will populate.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactLoadStatus {
    NoImageSelected,
    ImageLoading,
    ImageReady,
    AnalysisPending,
    ArtifactLoading,
    ArtifactReady,
    ArtifactUnavailable,
    ArtifactInvalid,
    AnalysisFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LesionCategory {
    Microaneurysms,
    Hemorrhages,
    HardExudates,
    CottonWoolSpots,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LesionCategoryToggles {
    pub microaneurysms: bool,
    pub hemorrhages: bool,
    pub hard_exudates: bool,
    pub cotton_wool_spots: bool,
}

#[derive(Debug, Clone)]
pub struct LesionMaskArtifact {
    pub id: String,
    pub examination_id: String,
    pub model_id: String,
    pub model_version: String,
    pub dimensions: [usize; 2],
    pub categories: Vec<LesionCategory>,
    pub opacity: f32,
    pub toggles: LesionCategoryToggles,
    pub status: ArtifactLoadStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VesselMaskArtifact {
    pub id: String,
    pub examination_id: String,
    pub model_id: String,
    pub model_version: String,
    pub dimensions: [usize; 2],
    pub opacity: f32,
    pub mask_only: bool,
    pub status: ArtifactLoadStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExplainabilityArtifact {
    pub id: String,
    pub examination_id: String,
    pub method: String, // e.g. "HierarchiRetina Grad-CAM"
    pub model_id: String,
    pub model_version: String,
    pub dimensions: [usize; 2],
    pub opacity: f32,
    pub status: ArtifactLoadStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RetinalExaminationView {
    pub exam_id: String,
    pub patient_pseudo_id: String,
    pub laterality: EyeLaterality,
    pub image_path: Option<std::path::PathBuf>,
    pub dimensions: Option<[usize; 2]>,
    pub sync_mode: SyncMode,
    pub expanded_panel: Option<ViewPanelId>,
    pub panel_states: [ViewerPanelState; 4],
    pub lesion_artifact: LesionMaskArtifact,
    pub vessel_artifact: VesselMaskArtifact,
    pub explainability_artifact: ExplainabilityArtifact,
}
```

## Truthful Fallback Rules
1. If Freebuff backend is disconnected or offline, artifacts default to `ArtifactLoadStatus::ArtifactUnavailable`.
2. The UI renders an informative dark card explaining the pending inference stage (e.g. "Awaiting HSMoE-AUNet Lesion Segmentation").
3. No dummy lesion masks or hallucinated heatmaps are ever generated in the frontend.
