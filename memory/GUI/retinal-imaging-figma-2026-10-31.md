# Retinal Imaging Figma page + session state (2026-10-31)

## Figma (design only, no code changed)
- File: https://www.figma.com/design/zV5EBNbVGXyo8kRJTN9F3d (page "Retinal Imaging"); main frame node 68-228.
- Frames: 1600x900 (main), 1448x1086, 1280x820. Clones of "Retinal Review - Light" shell; sidebar gets exactly one new item "Retinal Imaging" after "Retinal Review".
- Tokens: only existing variables (collections Color Light mode, Space, Radius). Fonts only Geist Regular/Medium/SemiBold + Instrument Serif (page title). Audit: 690 nodes, 0 unbound colours. Mirror: `GUI/Hackthon dezine/design/figma-tokens.json`.
- Brief hexes (#F5F0EA, #A95320, mint +) are NOT app tokens; not used. Selected states use `accent/primary`.
- Components reused: Toggle, Button (Secondary). Nothing new created.
- Preview strip relabelled to real saved outputs: Original Fundus, Vessel Mask, Lesion Overlay, Explainability. FA / ICG / OCT / Red-Free radios = Unavailable.
- Prototype: clicking a preview card opens that image large + centered with a Back button (only in opened state); Back returns to main. 4 opened frames "Open: <name>" (nodes 72:23, 72:263, 72:503, 72:743), 0.3 s dissolve. Flow only at 1600x900.
- Limits: image slots are dashed "no image loaded" outlines (Figma tool cannot embed images). No hover/focus/loading/error variants. Right control column scrolls (969 px content).

## Repo state
- Backend `fork` (rp0948566-hue/Dird): test-gpu = main = final = e746b7c; backup-2026-10-31 = eecbc3f.
- GUI `origin` (rp0948566-hue/AI-Manthan-2.0): test = final = 010d906; main = f350ad4; backup-2026-10-31 = 010d906.
- `RetinaCare-Training-Lab/` never committed.
- Commit e746b7c added: docs/gui-manual.md + PDF, regenerated technical report PDF, scripts/audit.sh, scripts/generate-gui-manual.py, CLAUDE.md audit lines.

## Open items
- Windows .exe: mingw missing (needs `sudo apt install gcc-mingw-w64-x86-64` or run GitHub Actions workflow); Mac build needs macOS runner. Not built.
- `scripts/audit.sh` not run yet; docs/audit-latest.md absent.
- Not done: doctor/RBAC role-play, screenshot GUI sweep, corrupt-image fuzz, full A-Z manual, panic!/println! triage, security.rs:95 unsafe review, GPU use confirmation via retinacare-launch + nvidia-smi.
- Implement the Figma design in Rust only on explicit request.
- claude-mem outage: credentials refused ("Your organization has disabled Claude subscription access for Claude Code"); this vault is the durable memory.

## 2026-11-01 update
- Installed binary = latest release (SHA-256 `93f64917691578ec109e0c8b9004b6b9f41bdb38f496d469e3c0a3cc55ccb9c7`, matched). Code and tests: [[Handoffs/changelog-and-handoffs]] 2026-11-01.
- Project report: `src/ui2/bilateral_assessment/FINAL_REPORT.md`.
- Open: see [[Handoffs/next-actions]] item 0c.
