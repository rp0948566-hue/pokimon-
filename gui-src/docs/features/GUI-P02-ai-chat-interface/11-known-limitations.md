# Known Limitations — GUI-P02 AI Chat Interface

## 1. Backend Constraints
- Real medical AI inference and clinical LLM processing depend on Codex/Freebuff's backend.
- When backend is absent, `UnavailableChatService` returns a truthful failure event (`AI service is not connected.`).
- No mock clinical data or fake diagnosis is generated in production code paths.

## 2. Scope Boundaries (Task 02 vs Later Tasks)
- Retinal Image Viewer quad-box deep exploration belongs to Task 03.
- Advanced SQLCipher database persistence belongs to Freebuff's backend.
- Full session archival and folder system overhaul belongs to Task 07.
