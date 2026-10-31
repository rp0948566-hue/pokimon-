# Error and Status Conventions

## Status Envelopes
All API responses that return operational data conform to standard envelope semantics:

- `SUCCESS`: Valid data returned within freshness threshold.
- `LOADING`: Awaiting asynchronous response.
- `STALE`: Data returned from local cache; live connection unavailable.
- `DEGRADED`: Partial data returned due to provider outage.
- `ERROR`: Request failed; envelope contains error details.

## Error Schema (`ApplicationErrorEnvelope`)
```typescript
interface ApplicationErrorEnvelope {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  timestamp: string; // ISO-8601 UTC
}
```
