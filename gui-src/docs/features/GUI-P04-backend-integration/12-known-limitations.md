# 12. Known Limitations & Truthful AI Scope

## 1. Vision Classifier vs Conversational AI Separation
- **HierarchiRetina Stage I**: A vision screening classifier based on ConvNeXt-V2-Large with attention pooling and an MLP single-logit output.
- **HierarchiRetina is NOT a language model**: It cannot answer free-form clinical questions, explain pathophysiology, or engage in clinical consultation.
- **Scope Enforcement**: The Chat interface remains connected to `ChatService` (with honest unavailable states when no LLM provider is connected) and never fabricates conversational responses from a Stage I logit.

## 2. Unintegrated Stages (Stages II, III, and IV)
- **Stage II Lesion Segmentation**: Not integrated. Codex backend lacks the segmentation model checkpoint and inference decoder. Panel displays honest `ArtifactUnavailable` state.
- **Stage II Vessel Segmentation**: Not integrated. Codex backend lacks the vascular segmentation model. Panel displays honest `ArtifactUnavailable` state.
- **Stage III 5-Class DR Grading**: Not integrated. The Stage I model only produces a binary gate score (`NegativeRoute` vs `ContinueCascade`). DR Grades 0 through 4 are not diagnosed.
- **Attribution / Heatmaps**: No Grad-CAM or integrated gradients attribution maps exist. Decorative simulated heatmaps are strictly forbidden.

## 3. Hardware Execution
- In the current environment, ONNX Runtime runs on the CPU execution provider.
- Although the host has an NVIDIA RTX 5070 GPU, CUDA acceleration requires `ort/cuda` features and matching TensorRT / cuDNN libraries compiled into the backend binary. The GUI explicitly displays `CPU Execution Provider` without false claims of GPU acceleration.

## 4. Regulatory & Clinical Notice
- RetinaCare is a research and clinical hackathon prototype.
- Stage I screening gate results do NOT constitute a certified medical diagnosis. Definite clinical diagnosis requires examination by an accredited ophthalmologist.
