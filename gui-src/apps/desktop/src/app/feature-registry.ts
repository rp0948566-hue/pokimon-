/**
 * Feature Registry
 * Documents active and deferred feature boundaries.
 */

export interface FeatureFlag {
  id: string;
  name: string;
  isEnabled: boolean;
  status: 'SCAFFOLDED' | 'IN_DEVELOPMENT' | 'PRODUCTION';
}

export const FEATURE_REGISTRY: Record<string, FeatureFlag> = {
  incidentManagement: { id: 'incidentManagement', name: 'Incident Management', isEnabled: true, status: 'SCAFFOLDED' },
  verifiedRouting: { id: 'verifiedRouting', name: 'Verified Routing', isEnabled: true, status: 'SCAFFOLDED' },
  liveMap: { id: 'liveMap', name: 'Interactive Map', isEnabled: true, status: 'SCAFFOLDED' },
  responderTracking: { id: 'responderTracking', name: 'Responder Telemetry', isEnabled: true, status: 'SCAFFOLDED' },
  droneRover: { id: 'droneRover', name: 'Drone/Rover Observation', isEnabled: true, status: 'SCAFFOLDED' },
  experimentalSensing: { id: 'experimentalSensing', name: 'Isolated Sensing Lab', isEnabled: false, status: 'SCAFFOLDED' },
  voiceCopilot: { id: 'voiceCopilot', name: 'Voice Guidance', isEnabled: false, status: 'SCAFFOLDED' },
};
