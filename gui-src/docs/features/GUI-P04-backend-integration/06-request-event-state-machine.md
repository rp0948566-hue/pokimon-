# 06. Request and Event State Machine

## Lifecycle Diagram

```mermaid
stateDiagram-v2
    [*] --> Idle: App Startup
    Idle --> ModelMissing: Manifest or Checkpoint Not Found
    Idle --> Initializing: Manifest Found, Loading Session
    Initializing --> Ready: Session Verified & Loaded
    Initializing --> Failed: Load Error / Incompatible Graph

    Ready --> Processing: User submits "Run Stage I Screening"
    Processing --> Processing: Progress Event (Preprocessing / Inference)
    Processing --> Stage1Completed: Genuine DecodedStage1Gate Received
    Processing --> Cancelling: User clicks Cancel
    Processing --> InferenceFailed: Runtime / Session Error

    Cancelling --> Ready: Cancel Confirmed / Reaped
    Stage1Completed --> Ready: User resets or selects new image
    InferenceFailed --> Ready: User acknowledges error or retries
```

## Examination Isolation Rules
1. Every outgoing request records `examination_id` and `image_hash`.
2. When the GUI changes active examination, the active `pending_request_id` is updated.
3. If an incoming `InferenceEvent::Stage1Completed` matches an old or superseded `request_id` or `examination_id`, it is dropped with an informational log: `Stale event rejected`.
4. Result state is tied to `(examination_id, image_path)`. Replacing the image resets the analysis state to Idle.
