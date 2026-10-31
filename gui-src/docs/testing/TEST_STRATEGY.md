# Test Strategy — PokéRescue Sentinel

## Testing Pyramid
1. **Contract Tests (`packages/contracts`)**: Validate TypeScript type export integrity and JSON schema serialization.
2. **Unit Tests (`apps/desktop/tests/unit`)**: Validate pure utility functions, time formatters, coordinate calculators, and status badges.
3. **Component Tests (`apps/desktop/tests/component`)**: Test isolated UI components using Vitest and React Testing Library.
4. **End-to-End Tests (`apps/desktop/tests/e2e`)**: High-level workflow smoke tests via Playwright (future phase).

## Continuous Validation Standard
- Every build must pass `pnpm typecheck` without warnings or implicit `any` types.
- Scaffold checks must verify configuration integrity before desktop builds.
