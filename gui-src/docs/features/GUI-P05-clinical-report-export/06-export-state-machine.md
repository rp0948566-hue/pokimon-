# 06 - Export State Machine & Nonblocking Lifecycle

```
       [User clicks Export PDF / Session]
                       │
                       ▼
       ┌───────────────────────────────┐
       │             Idle              │
       └───────────────┬───────────────┘
                       │ [Export Dialog Confirmed]
                       ▼
       ┌───────────────────────────────┐
       │           Preparing           │
       └───────────────┬───────────────┘
                       │
                       ▼
       ┌───────────────────────────────┐
       │          Generating           │◄─── (Worker thread executes I/O)
       └───────┬───────────────┬───────┘
               │               │
  [Write OK]   │               │ [Disk/Parse Error or Unavailable]
               ▼               ▼
┌────────────────────────┐  ┌────────────────────────┐
│  Saved (Path, Bytes)   │  │   Failed / Unavailable  │
└────────────────────────┘  └────────────────────────┘
```

## State Definitions
- `Idle`: No export operation active.
- `Preparing`: Packing view model parameters and launching save dialog.
- `Generating`: Background worker executing export task.
- `Saved { path: PathBuf, bytes: usize }`: File successfully written and verified.
- `Failed { error: String }`: Export encountered I/O or rendering error.
- `Cancelled`: Clinician cancelled file dialog or generation.
- `BackendUnavailable { reason: String }`: Truthful notice that backend export service is not present.
