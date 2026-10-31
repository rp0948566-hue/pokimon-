# Frontend Architecture — PokéRescue Sentinel

## Technology Stack
- **Framework**: React 19 (TSX)
- **Language**: TypeScript (Strict Mode enabled, no implicit `any`)
- **Build Tool**: Vite 6
- **Desktop Runtime**: Tauri 2 (Rust shell)
- **Monorepo Manager**: pnpm workspace

## Architectural Principles
1. **Feature-Driven Structure**: Features are self-contained modules located in `apps/desktop/src/features/`. Cross-feature coupling is prohibited.
2. **Contract-Driven Communication**: Frontend components communicate with services using strictly typed definitions defined in `packages/contracts`.
3. **Neutral Styling Strategy**: Layouts and pages are established without hardcoded visual designs. All aesthetic decisions await user-provided design tokens.
4. **Resilient Error Boundaries**: Top-level and route-level error boundaries isolate crashes and prevent complete application disruption.
