# Security Policy

## Current Scaffold State
PokéRescue Sentinel is currently in a scaffold-only phase. No live authentication, GPS tracking, secret storage, or emergency dispatch services are implemented.

## Reporting Vulnerabilities
If you discover a security vulnerability within this repository, please report it privately to the maintainers rather than opening a public issue.

## Security Principles
- Never commit secrets, credentials, or private keys to the repository.
- Keep Tauri permissions strictly minimal following `docs/security/TAURI_CAPABILITY_POLICY.md`.
- All untrusted network inputs must be validated at runtime boundaries before processing.
