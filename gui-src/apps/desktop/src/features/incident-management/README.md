# Feature: incident-management

## Purpose
Domain UI, local state, and helper hooks for incident-management.

## Boundaries
- Do not import internals of other features directly.
- Consume domain types from `@pokerescue/contracts`.
- All operational logic must validate against server-side authorities.
