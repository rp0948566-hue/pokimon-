# Release Checklist

Verify each step before publishing an official PokéRescue Sentinel desktop release:

- [ ] All workspace tests pass (`pnpm test`).
- [ ] TypeScript type checks cleanly across packages (`pnpm typecheck`).
- [ ] No placeholder or dummy secrets in `.env` or configurations.
- [ ] Semantic version incremented in `package.json`, `apps/desktop/package.json`, and `tauri.conf.json`.
- [ ] `CHANGELOG.md` updated with released features and bug fixes.
- [ ] Windows installer binary signed with authorized Authenticode certificate.
- [ ] Installer runs cleanly on a pristine Windows 11 machine without developer tools.
