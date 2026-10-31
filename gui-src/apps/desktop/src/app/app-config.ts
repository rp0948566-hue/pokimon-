/**
 * Application Configuration
 */

export interface AppConfig {
  apiBaseUrl: string;
  realtimeWsUrl: string;
  mode: 'DEVELOPMENT' | 'SIMULATION' | 'PRODUCTION';
  enableExperimentalSensing: boolean;
  enableVoiceGuidance: boolean;
}

export const appConfig: AppConfig = {
  apiBaseUrl: import.meta.env.VITE_API_BASE_URL || 'http://127.0.0.1:8080/api/v1',
  realtimeWsUrl: import.meta.env.VITE_REALTIME_WS_URL || 'ws://127.0.0.1:8080/events',
  mode: (import.meta.env.VITE_APP_MODE as AppConfig['mode']) || 'DEVELOPMENT',
  enableExperimentalSensing: import.meta.env.VITE_ENABLE_EXPERIMENTAL_SENSING === 'true',
  enableVoiceGuidance: import.meta.env.VITE_ENABLE_VOICE_GUIDANCE === 'true',
};
