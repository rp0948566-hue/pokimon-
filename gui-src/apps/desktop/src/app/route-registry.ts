/**
 * Route Registry
 * Enumerates all planned screen routes across the command center.
 */

export interface RouteDefinition {
  id: string;
  path: string;
  title: string;
  description: string;
}

export const ROUTE_REGISTRY: RouteDefinition[] = [
  { id: 'overview', path: '/', title: 'Command Overview', description: 'Real-time situational dashboard' },
  { id: 'incidents', path: '/incidents', title: 'Incidents', description: 'Active triage and timeline' },
  { id: 'map', path: '/map', title: 'Live Map', description: 'Geospatial theatre view' },
  { id: 'routing', path: '/routing', title: 'Verified Routing', description: 'Passable corridor planning' },
  { id: 'teams', path: '/teams', title: 'Teams & Responders', description: 'Authorized unit check-in' },
  { id: 'facilities', path: '/facilities', title: 'Facilities', description: 'Medical and shelter capacities' },
  { id: 'drone-rover', path: '/drone-rover', title: 'Drone & Rover', description: 'Observation feeds' },
  { id: 'sensing-lab', path: '/sensing-lab', title: 'Sensing Lab', description: 'Isolated Wi-Fi CSI experimental lab' },
  { id: 'reports', path: '/reports', title: 'Reports & Evidence', description: 'Multi-source records' },
  { id: 'voice-copilot', path: '/voice-copilot', title: 'Voice Guidance', description: 'Audio copilot settings' },
  { id: 'data-sources', path: '/data-sources', title: 'Data Sources', description: 'Provider health monitoring' },
  { id: 'offline-status', path: '/offline-status', title: 'Offline Status', description: 'Local mesh sync and storage' },
  { id: 'settings', path: '/settings', title: 'Settings', description: 'Desktop preferences and diagnostics' },
];
