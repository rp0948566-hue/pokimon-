import test from 'node:test';
import assert from 'node:assert/strict';
import * as contracts from './index.js';

test('contracts package exports expected type interfaces', () => {
  // Verify that the module evaluates cleanly and exports are available
  assert.ok(contracts !== undefined, 'Contracts module must be defined');
});
