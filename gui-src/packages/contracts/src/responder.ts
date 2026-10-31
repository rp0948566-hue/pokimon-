/**
 * Responder Telemetry Contracts
 * Models representing authorized response teams and telemetry.
 */

import type { GeoCoordinate } from './incident.js';

export type ResponderRole = 'COMMANDER' | 'SEARCH_RESCUE' | 'MEDICAL' | 'LOGISTICS' | 'DRONE_OPERATOR';
export type ResponderOperationalStatus = 'IDLE' | 'EN_ROUTE' | 'ON_SCENE' | 'RETURNING' | 'OFFLINE';

export interface ResponderUnitSummary {
  id: string;
  callsign: string;
  role: ResponderRole;
  status: ResponderOperationalStatus;
  current_location?: GeoCoordinate;
  last_checkin_at: string; // ISO-8601 UTC
  current_incident_id?: string;
}

export interface ResponderLocationUpdate {
  responder_id: string;
  timestamp: string; // ISO-8601 UTC
  location: GeoCoordinate;
  heading_degrees?: number;
  speed_kmh?: number;
}
