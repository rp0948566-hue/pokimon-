# Dependency Policy

## Policy Standards
1. **Zero Over-Engineering**: Follow the Ponytail ladder reflex: Standard library/native web platform -> existing local helper -> single dependency -> minimum code.
2. **Strict License Audit**: Only permissive licenses (MIT, Apache 2.0, BSD-2/3, ISC) are permitted in production builds. GPL/AGPL packages are prohibited in client bundles.
3. **No Wildcard Versions**: All dependencies must be pinned or caret-bounded to stable semver versions.
4. **Single Lockfile**: The repository strictly standardizes on `pnpm-lock.yaml`. Generating `package-lock.json` or `yarn.lock` is prohibited.
5. **Evaluation Criteria**: Before adding any third-party dependency, verify:
   - Bundle size impact.
   - Tree-shaking support (ESM).
   - Maintenance status and zero open critical vulnerabilities.
