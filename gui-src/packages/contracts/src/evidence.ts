/**
 * Evidence & Sensing Contracts
 * Multi-source observations validating road or incident states.
 */

import type { GeoCoordinate } from './incident.js';

export type EvidenceSourceType = 'DRONE' | 'ROVER' | 'SATELLITE' | 'FIELD_RESPONDER' | 'CITIZEN_REPORT';

export interface EvidenceRecord {
  id: string;
  source_type: EvidenceSourceType;
  observed_at: string; // ISO-8601 UTC
  received_at: string; // ISO-8601 UTC
  location: GeoCoordinate;
  description: string;
  media_url?: string;
  confidence_score: number; // 0.0 - 1.0
  is_authoritative: boolean;
}

export interface DroneObservation {
  observation_id: string;
  drone_callsign: string;
  timestamp: string; // ISO-8601 UTC
  coordinates: GeoCoordinate;
  telemetry: {
    battery_percentage: number;
    altitude_agl_meters: number;
  };
  detected_hazard?: string;
}

/**
 * Isolated Experimental Sensing Observation
 * Note: Decoupled from rescue routing contracts.
 */
export interface ExperimentalSensingObservation {
  frame_id: string;
  timestamp: string; // ISO-8601 UTC
  mode: 'SIMULATED' | 'EXPERIMENTAL';
  channel_metric: number;
  anomaly_detected: boolean;
  notes: string;
}
