/**
 * Route & Passability Contracts
 * Strongly-typed definitions for verified passage through disaster zones.
 */

import type { GeoCoordinate } from './incident.js';

export type RoadVerificationState = 'VERIFIED_PASSABLE' | 'BLOCKED' | 'UNVERIFIED' | 'STALE';

export interface RoadSegmentStatus {
  segment_id: string;
  name: string;
  state: RoadVerificationState;
  start_coord: GeoCoordinate;
  end_coord: GeoCoordinate;
  last_verified_at?: string; // ISO-8601 UTC
  verified_by?: string;
  hazard_notes?: string;
}

export interface RouteRecommendation {
  route_id: string;
  origin: GeoCoordinate;
  destination: GeoCoordinate;
  total_distance_km: number;
  estimated_duration_minutes: number;
  is_fully_verified: boolean;
  segments: RoadSegmentStatus[];
  calculated_at: string; // ISO-8601 UTC
}

export interface RouteApproval {
  approval_id: string;
  route_id: string;
  commander_id: string;
  approved_at: string; // ISO-8601 UTC
  notes?: string;
}
