# ADR-0001: Adoption of React and TypeScript for Frontend

## Status
Accepted

## Context
PokéRescue Sentinel requires an interactive, accessible, and maintainable user interface for disaster-response command centers.

## Decision
Adopt React 19 with TypeScript and TSX, built with Vite.
- React provides component composability and rapid iteration.
- TypeScript strict mode eliminates runtime type errors across complex incident data structures.
- Domain logic is reserved for backend services, keeping the UI strictly focused on presentation and interaction.

## Consequences
- Requires strict typing across all data models.
- Fast build and reload cycles via Vite.
