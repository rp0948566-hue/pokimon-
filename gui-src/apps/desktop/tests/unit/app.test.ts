import { describe, it, expect } from 'vitest';
import { ROUTE_REGISTRY } from '../../src/app/route-registry';
import { appConfig } from '../../src/app/app-config';

describe('App Scaffold Baseline', () => {
  it('registers all planned routes', () => {
    expect(ROUTE_REGISTRY.length).toBeGreaterThanOrEqual(13);
    const overviewRoute = ROUTE_REGISTRY.find((r) => r.id === 'overview');
    expect(overviewRoute).toBeDefined();
    expect(overviewRoute?.path).toBe('/');
  });

  it('provides baseline development configuration', () => {
    expect(appConfig).toBeDefined();
    expect(appConfig.mode).toBe('DEVELOPMENT');
  });
});
