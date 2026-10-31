# ADR-0003: User Ownership of Visual Design

## Status
Accepted

## Context
Visual appearance, color schemes, brand identity, and layout ergonomics are critical to user workflows in crisis coordination.

## Decision
The user retains absolute ownership over the visual design.
- Autonomous coding agents must not invent color palettes, component visual themes, or typography rules.
- Scaffolding provides unstyled structural layouts with clear handoff slots.
- All visual styling is defined through `docs/ui/USER_DESIGN_HANDOFF.md` and `apps/desktop/src/styles/design-tokens.css`.

## Consequences
- Prevents wasted effort on speculative designs that the user would need to rewrite.
- Clear boundary between software architecture and visual design.
