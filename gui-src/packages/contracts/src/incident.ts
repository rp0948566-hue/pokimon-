/**
 * Incident Contracts
 * Data structures representing emergency events in the command theater.
 */

export type IncidentSeverity = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
export type IncidentStatus = 'REPORTED' | 'TRIAGED' | 'DISPATCHED' | 'CONTAINED' | 'RESOLVED';

export interface GeoCoordinate {
  latitude: number;
  longitude: number;
  altitude_meters?: number;
}

export interface IncidentSummary {
  id: string;
  code: string;
  title: string;
  description: string;
  severity: IncidentSeverity;
  status: IncidentStatus;
  location: GeoCoordinate;
  created_at: string; // ISO-8601 UTC
  updated_at: string; // ISO-8601 UTC
  assigned_responder_ids: string[];
}

export interface IncidentTimelineEntry {
  id: string;
  incident_id: string;
  timestamp: string; // ISO-8601 UTC
  summary: string;
  author: string;
  severity_change?: IncidentSeverity;
}
