//! Model inventory, manifest, and checkpoint compatibility verification for HierarchiRetina v1.0.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    PyTorchStateDict, // .pth / .pt (Requires ONNX export for Rust runtime)
    Onnx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointStatus {
    VerifiedActive,
    BlockedNeedsOnnxExport,
    MissingFromDisk,
}

impl CheckpointStatus {
    pub fn badge_text(&self) -> &'static str {
        match self {
            CheckpointStatus::VerifiedActive => "ACTIVE (ONNX, CPU verified; CUDA unverified)",
            CheckpointStatus::BlockedNeedsOnnxExport => "BLOCKED (NEEDS ONNX EXPORT)",
            CheckpointStatus::MissingFromDisk => "UNAVAILABLE (WEIGHTS MISSING)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelStageInfo {
    pub stage_name: &'static str,
    pub architecture: &'static str,
    pub target_task: &'static str,
    pub weights_file: &'static str,
    pub format: ModelFormat,
    pub status: CheckpointStatus,
    pub input_shape: &'static str,
    pub notes: &'static str,
}

pub struct HierarchiRetinaManifest {
    pub version: &'static str,
    pub repository: &'static str,
    pub weight_source: &'static str,
    pub license: &'static str,
    pub stages: Vec<ModelStageInfo>,
}

impl HierarchiRetinaManifest {
    pub fn load_audit() -> Self {
        Self {
            version: "v1.0",
            repository: "https://github.com/asifuddin01/HierarchiRetina",
            weight_source: "https://github.com/asifuddin01/HierarchiRetina/releases/tag/v1.0",
            license: "Creative Commons Attribution-NonCommercial 4.0 (CC BY-NC 4.0)",
            stages: vec![
                ModelStageInfo {
                    stage_name: "Stage I Screening Gate",
                    architecture: "ConvNeXt V2-Large",
                    target_task: "Gradable vs Ungradable Quality Filter",
                    weights_file: "models/stage1_convnextv2l_768/stage1.onnx",
                    format: ModelFormat::Onnx,
                    status: CheckpointStatus::VerifiedActive,
                    input_shape: "[1, 3, 768, 768], Normalization: ImageNet Mean/Std",
                    notes: "Runs through backend ModelService (Rust ONNX, CPU parity on demo image). Hash and threshold come from the backend manifest at runtime; CUDA and licensed-image acceptance pending.",
                },
                ModelStageInfo {
                    stage_name: "Stage II Lesion Segmentation",
                    architecture: "HSMoE-AUNet",
                    target_task: "Pixel-wise Microaneurysms (MA), Hemorrhages (HE), Exudates (EX), Cotton-Wool (CWS)",
                    weights_file: "models/stage2_hsmoe_aunet/{ma,he,ex,cws}/stage2_*.onnx",
                    format: ModelFormat::Onnx,
                    status: CheckpointStatus::VerifiedActive,
                    input_shape: "[1, 3, 1024, 1024] per model (MA, HE, EX, CWS), original-frame masks",
                    notes: "Four separate ONNX models run one at a time by the backend cascade; parity checked on the demo image only. Licensed-image acceptance pending.",
                },
                ModelStageInfo {
                    stage_name: "Stage II Vessel Segmentation",
                    architecture: "SwinHRUNet++",
                    target_task: "Retinal Vascular Caliber & Vessel Tree Segmentation",
                    weights_file: "models/stage2_swinhrunetpp/stage2_vessel.onnx",
                    format: ModelFormat::Onnx,
                    status: CheckpointStatus::VerifiedActive,
                    input_shape: "[1, 3, 1024, 1024], vessel mask restored to the original frame",
                    notes: "Runs in the backend cascade; parity checked on the demo image only.",
                },
                ModelStageInfo {
                    stage_name: "Stage III DR Grading",
                    architecture: "Lesion-Aware LG-DRG",
                    target_task: "Five-Grade Diabetic Retinopathy Classification (Grade 0-4)",
                    weights_file: "models/stage3_lgdrg/stage3_lgdrg_fold0.onnx",
                    format: ModelFormat::Onnx,
                    status: CheckpointStatus::VerifiedActive,
                    input_shape: "[1, 8, 512, 512]: RGB + five Stage II masks",
                    notes: "FOLD 0 ONLY unless all five fold manifests are installed (then the five-fold ensemble). The result states which one ran.",
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_stages() {
        let manifest = HierarchiRetinaManifest::load_audit();
        assert_eq!(manifest.stages.len(), 4);
        assert_eq!(manifest.version, "v1.0");
    }
}
