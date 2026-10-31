# Repository Tree — PokéRescue Sentinel Workspace

```
PokéRescue Sentinel Workspace
├── .editorconfig
├── .env.example
├── .gitattributes
├── .github
│   ├── ISSUE_TEMPLATE
│   │   ├── bug_report.md
│   │   └── feature_request.md
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── README.md
│   └── workflows
│       └── frontend-checks.yml
├── .gitignore
├── .npmrc
├── AGENTS.md
├── CHANGELOG.md
├── CLAUDE.md
├── CODE_OF_CONDUCT.md
├── CONTRIBUTING.md
├── GEMINI.md
├── Ghost.md
├── LICENSE
├── README.md
├── SECURITY.md
├── package.json
├── pnpm-workspace.yaml
├── apps
│   └── desktop
│       ├── .prettierrc.json
│       ├── README.md
│       ├── eslint.config.js
│       ├── index.html
│       ├── package.json
│       ├── tsconfig.json
│       ├── tsconfig.node.json
│       ├── vite.config.ts
│       ├── src
│       │   ├── main.tsx
│       │   ├── vite-env.d.ts
│       │   ├── app
│       │   │   ├── App.tsx
│       │   │   ├── app-config.ts
│       │   │   ├── app-providers.tsx
│       │   │   ├── app-router.tsx
│       │   │   ├── feature-registry.ts
│       │   │   ├── route-registry.ts
│       │   │   └── error-boundary/README.md
│       │   ├── layouts
│       │   │   ├── CommandCenterLayout.tsx
│       │   │   ├── ResponderLayout.tsx
│       │   │   └── README.md
│       │   ├── pages/ (13 route placeholder directories)
│       │   ├── features/ (12 domain feature directories)
│       │   ├── components/ (7 UI and layout primitive groups)
│       │   ├── contracts/ (Local client adapter re-exports)
│       │   ├── lib/ (formatting, validation, platform)
│       │   ├── hooks/
│       │   ├── state/
│       │   ├── services/ (api, realtime, desktop)
│       │   ├── styles/ (index.css, design-tokens.css, README.md)
│       │   ├── assets/ (icons, images, illustrations)
│       │   └── test/ (setup.ts, README.md)
│       ├── tests
│       │   ├── unit/app.test.ts
│       │   ├── component/README.md
│       │   └── e2e/README.md
│       └── src-tauri
│           ├── Cargo.toml
│           ├── build.rs
│           ├── tauri.conf.json
│           ├── README.md
│           ├── capabilities
│           │   └── default.json
│           ├── icons/README.md
│           └── src
│               ├── lib.rs
│               └── main.rs
├── packages
│   └── contracts
│       ├── README.md
│       ├── package.json
│       ├── tsconfig.json
│       └── src
│           ├── evidence.ts
│           ├── incident.ts
│           ├── index.test.ts
│           ├── index.ts
│           ├── provider-health.ts
│           ├── responder.ts
│           └── route.ts
├── docs
│   ├── README.md
│   ├── product/ (Overview, Goals, Personas, Scope, Glossary)
│   ├── architecture/ (Scaffold Plan, Frontend Arch, Ownership, Boundary, Tauri, Dependency Policy, Open Decisions)
│   ├── decisions/ (ADR-0001 through ADR-0004)
│   ├── ui/ (Design Handoff, Inventory, Tokens, Components, Routes)
│   ├── api/ (REST, Realtime, Errors)
│   ├── security/ (Security Boundary, Capability Policy)
│   ├── privacy/ (Location and Sensor Policy)
│   ├── testing/ (Strategy, Accessibility, Performance Budgets)
│   ├── release/ (Windows Packaging, Release Checklist)
│   └── roadmap/ (Phases, Integration Register)
└── scripts
    ├── README.md
    └── check-scaffold.sh
```
