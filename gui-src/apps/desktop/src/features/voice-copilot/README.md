# Feature: voice-copilot

## Purpose
Domain UI, local state, and helper hooks for voice-copilot.

## Boundaries
- Do not import internals of other features directly.
- Consume domain types from `@pokerescue/contracts`.
- All operational logic must validate against server-side authorities.
