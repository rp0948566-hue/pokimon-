# Real-Time Event Contracts (WebSocket / SSE)

Candidate event names and message payloads for real-time dispatch streams:

| Event Name | Direction | Payload Type | Description |
| :--- | :--- | :--- | :--- |
| `incident.updated` | Server -> Client | `IncidentSummary` | Emitted when incident status or severity changes. |
| `responder.telemetry`| Server -> Client | `ResponderLocationUpdate` | Streaming GPS coordinates and velocity. |
| `road.status_changed`| Server -> Client | `RoadSegmentStatus` | Road confirmed passable, blocked, or stale. |
| `evidence.received` | Server -> Client | `EvidenceRecord` | Drone imagery or field report ingested. |
| `sensing.frame` | Server -> Client | `ExperimentalSensingObservation` | Isolated experimental sensing frame. |
