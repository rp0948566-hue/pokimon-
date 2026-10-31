# API Contract Placeholder

Candidate REST endpoint groupings for future Rust backend services:

## 1. Incidents API (`/api/v1/incidents`)
- `GET /api/v1/incidents`: List active incidents.
- `GET /api/v1/incidents/:id`: Incident details and timeline.
- `POST /api/v1/incidents`: Register a new incident.

## 2. Routing & Passability API (`/api/v1/routes`)
- `POST /api/v1/routes/calculate`: Calculate verified routes between coordinates.
- `GET /api/v1/roads/:segment_id/evidence`: Fetch verification evidence for a road segment.

## 3. Responders API (`/api/v1/responders`)
- `GET /api/v1/responders`: List authorized responder units.
- `POST /api/v1/responders/:id/location`: Receive authenticated location telemetry.

## 4. Facilities API (`/api/v1/facilities`)
- `GET /api/v1/facilities`: List medical, shelter, and supply facilities.
