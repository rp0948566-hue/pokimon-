# Model inventory (read-only, P0)

Backend `test-gpu` dirty=2; GUI `test` dirty=0

| model_id | task | arch | pytorch ckpt | onnx sha256 (manifest) | thresholds |
|---|---|---|---|---|---|
| hierarchiretina-stage1-convnextv2l-768 | dr_screening_gate | convnextv2_large+gem+attn_pool+mlp_head | stage1_convnextv2l_768_best_auc_model.pth | `9ad51e1f428d` | {} |
| hierarchiretina-stage2-cws-hsmoe-aunet | cotton_wool_spot_segmentation | HSMoE-AUNet-CWS | stage2_hsmoe_aunet_CWS_best_model.pth | `6c928a75b288` | {"deployed_probability_threshold": 0.4, "threshold_operator": "greater_than"} |
| hierarchiretina-stage2-ex-hsmoe-aunet | hard_exudate_segmentation | HSMoE-AUNet-EX | stage2_hsmoe_aunet_EX_best_model.pth | `2aa92099f233` | {"deployed_probability_threshold": 0.325, "threshold_operator": "greater_than"} |
| hierarchiretina-stage2-he-hsmoe-aunet | hemorrhage_segmentation | HSMoE-AUNet-HE | stage2_hsmoe_aunet_HE_best_model.pth | `16f767a8deb3` | {"deployed_probability_threshold": 0.2, "threshold_operator": "greater_than"} |
| hierarchiretina-stage2-ma-hsmoe-aunet | microaneurysm_segmentation | HSMoE-AUNet-MA | stage2_hsmoe_aunet_MA_best_model.pth | `ac874bee868b` | {"deployed_probability_threshold": 0.01, "threshold_operator": "greater_than"} |
| hierarchiretina-stage2-vessel-swinhrunetpp | vessel_segmentation | SwinHRUNet++-vessel | stage2_swinhrunetpp_best_dice.pth | `cc7ef0bb2373` | {"deployed_probability_threshold": 0.46, "threshold_operator": "greater_than_or_equal"} |
| hierarchiretina-stage3-lgdrg-fold0 | lg_drg_grading | LG-DRG-ConvNeXtV2-L | stage3_lgdrg_best_fold0.pt | `31f20d62f972` | {"corn_thresholds": [0.38, 0.6, 0.19], "gradability_threshold": 0.1} |
| hierarchiretina-stage3-lgdrg-fold1 | lg_drg_grading | LG-DRG-ConvNeXtV2-L | stage3_lgdrg_best_fold1.pt | `3f0deb84cb04` | {"corn_thresholds": [0.38, 0.6, 0.19], "gradability_threshold": 0.1} |
| hierarchiretina-stage3-lgdrg-fold2 | lg_drg_grading | LG-DRG-ConvNeXtV2-L | stage3_lgdrg_best_fold2.pt | `a55301c7e272` | {"corn_thresholds": [0.38, 0.6, 0.19], "gradability_threshold": 0.1} |
| hierarchiretina-stage3-lgdrg-fold3 | lg_drg_grading | LG-DRG-ConvNeXtV2-L | stage3_lgdrg_best_fold3.pt | `ee7c16614859` | {"corn_thresholds": [0.38, 0.6, 0.19], "gradability_threshold": 0.1} |
| hierarchiretina-stage3-lgdrg-fold4 | lg_drg_grading | LG-DRG-ConvNeXtV2-L | stage3_lgdrg_best_fold4.pt | `659e65fb3363` | {"corn_thresholds": [0.38, 0.6, 0.19], "gradability_threshold": 0.1} |

## Compatibility matrix

| stage | task | artifacts | data / training compatibility |
|---|---|---|---|
| Stage I gate (ConvNeXtV2-L 768) | binary screening (grade>=1) | stage1_convnextv2l_768_best_auc_model.pth + stage1.onnx | baseline: APTOS/IDRiD grade>=1 labels. fine-tune: needs upstream arch (hierarchiretina-src, MIT) + VRAM probe; NOT measured on 8 GB |
| Stage II MA/HE/EX/CWS (HSMoE-AUNet) | lesion segmentation | *_best_model.pth + stage2_*.onnx | baseline/fine-tune: IDRiD pixel masks (MA,HE,EX,SE~CWS). CWS vs SE semantic mapping unverified |
| Stage II vessel (SwinHRUNet++) | vessel segmentation | stage2_swinhrunetpp_best_dice.pth + stage2_vessel.onnx | baseline/fine-tune: DRIVE training references only (test refs withheld) |
| Stage III LG-DRG x5 folds | 5-grade CORN + gradability, input RGB+5 masks | stage3_lgdrg_best_fold{0-4}.pt + .onnx | needs Stage II masks for every image; cascade baseline NOT implemented in lab (PARTIAL) |

Licence of deployed weights: CC-BY-NC-4.0 (non-commercial), code MIT. ONNX files are inference-only; fine-tuning must start from the `.pth/.pt` PyTorch checkpoints with the upstream architecture.
