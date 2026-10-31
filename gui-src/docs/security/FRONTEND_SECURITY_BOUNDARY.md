# Frontend Security Boundary

## Threat Model and Boundaries
1. **Zero Secret Storage in Client**: API keys, signing certificates, and database master credentials must never be bundled into the frontend or Tauri desktop package.
2. **Safe IPC Boundaries**: Communication between Webview and Tauri Rust host must use explicit parameter validation. No arbitrary string evaluation or direct shell commands are permitted.
3. **Content Security Policy (CSP)**:
   - Restrict script execution to local application assets.
   - Forbid inline scripts and `eval()`.
4. **Data Sanitization**: All incoming incident descriptions, field reports, and filenames must be sanitized against cross-site scripting (XSS) before rendering in DOM.
