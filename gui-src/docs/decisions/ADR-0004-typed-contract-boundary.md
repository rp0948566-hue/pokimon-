# ADR-0004: Strongly Typed Contract Boundary

## Status
Accepted

## Context
Future backend services will be written in Rust or separate backend architectures. Frontend and backend must communicate through stable, versioned schemas without coupling UI components to backend implementations.

## Decision
Create an isolated package `packages/contracts` containing pure TypeScript type definitions and validation schemas.
- Both desktop client and future mock/real services depend on this package.
- No backend code or frontend component code is permitted in `packages/contracts`.
- All timestamps must be ISO-8601 UTC strings.

## Consequences
- Guaranteed compile-time type safety across service integration boundaries.
- Independent versioning of domain models.
