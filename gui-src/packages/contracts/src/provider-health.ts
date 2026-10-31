/**
 * Provider & Integration Health Contracts
 */

export type ServiceHealthStatus = 'HEALTHY' | 'DEGRADED' | 'DOWN' | 'UNKNOWN';

export interface ExternalProviderHealth {
  provider_id: string;
  name: string;
  status: ServiceHealthStatus;
  latency_ms?: number;
  last_sync_at: string; // ISO-8601 UTC
  error_message?: string;
}

export interface OfflineSyncStatus {
  is_online: boolean;
  pending_mutation_count: number;
  last_successful_sync_at?: string; // ISO-8601 UTC
  cached_incident_count: number;
}
