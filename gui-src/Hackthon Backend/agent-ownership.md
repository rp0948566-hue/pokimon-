---
title: Repository and agent ownership
type: ownership
domain: project
status: VERIFIED
verified_on: 2026-10-28
source_refs:
  - "../../Dird/AGENTS.md"
  - "../../Dird/native/Cargo.toml"
  - "../rust/Cargo.toml"
owner: Codex backend; Antigravity GUI
tags: [ownership, repositories]
---
# Ownership and repositories

| Workstream | Repository / branch | Owner | Guardrail |
|---|---|---|---|
| Backend, domain, persistence, reports, AI bridge, Python worker | /home/claude/Desktop/Dird, main | Codex/Claude | Work only in task-scoped backend files. |
| Desktop GUI | /home/claude/Desktop/Hackthon dezine, dezine | Antigravity | rust/src/ is read-only unless user reassigns ownership. |
| Legacy DIRD+ reference | Dird src/, src-tauri/ | Existing project | Preserve until migration is explicitly verified. |
| Backend Obsidian vault | Hackthon dezine/Hackthon Backend (canonical, owner decision) | Claude backend | Keep separate from GUI vault memory/. |

Verified paths: GUI rust/Cargo.toml uses path dependencies
../../Dird/native/crates/retinacare-ai-bridge and retinacare-app. They resolve
only with the present sibling layout. Both repositories have existing
uncommitted work. See [[source-map]], [[architecture]], [[risks-and-blockers]].
