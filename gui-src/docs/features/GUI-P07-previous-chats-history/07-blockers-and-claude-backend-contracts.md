# Previous Chats & History Management — Blockers & Claude Backend Contracts (07)

## Status Verdict
- **GUI Workspace & Architecture**: **COMPLETE** (100% native Rust egui/eframe, responsive desktop/narrow layouts, stable sorting, search, timeline ruler, event lanes, thumbnail rail, inspector).
- **Durable History & Event Persistence**: **PARTIAL / BLOCKED** (Codex backend crates `retinacare-persistence` and `retinacare-app` do not yet expose durable SQLCipher tables for conversation messages or audit timelines).

---

## Required Backend Contracts (Handoff to Claude Backend)

### 1. Durable Chat Session & Message Persistence
To upgrade from ephemeral in-memory sessions to persistent clinical audit archives, the backend must expose:
```rust
pub trait ConversationPersistenceService: Send + Sync {
    /// Return health and encryption unlock state
    fn readiness(&self) -> PersistenceReadiness;

    /// List archived consultations with metadata
    fn list_sessions(&self, query: &SessionQuery) -> Result<Vec<ChatSessionSummary>, PersistenceError>;

    /// Fetch complete message transcript for a session
    fn get_session_transcript(&self, session_id: &str) -> Result<Vec<ChatMessageRecord>, PersistenceError>;

    /// Append new message or upsert session
    fn save_session(&self, session: &ChatSessionRecord) -> Result<(), PersistenceError>;
}
```

### 2. Clinical Examination Event Ledger
To support long-term retrospective tracking on the Clinical Timeline across visits:
```rust
pub trait ClinicalTimelineService: Send + Sync {
    /// List timeline events for a validated patient ID
    fn list_patient_events(&self, patient_id: i64) -> Result<Vec<ClinicalEventRecord>, PersistenceError>;

    /// Record a clinical milestone (capture, screening gate, mask generation, report export)
    fn record_event(&self, event: &NewClinicalEvent) -> Result<String, PersistenceError>;
}
```

---

## Anti-Fabrication Guarantee
The Antigravity GUI layer guarantees:
1. **Zero Fake Database Writes**: The UI does not silently create local plaintext JSON files pretending to be encrypted databases.
2. **Zero Fabricated Diagnostic Grades**: Incomplete stages (Stage II lesion masks, Stage III DR grading) remain explicitly marked as unavailable.
3. **Truthful UI Banners**: When persistence is offline, prominent notices inform the operator that sessions are ephemeral.
