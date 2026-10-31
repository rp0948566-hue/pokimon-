# Ollama streaming implementation

Inspect existing Rust Ollama client. Use configured endpoint and model. For local Ollama, `POST /api/chat` with `stream: true` returns newline-delimited JSON. Incrementally buffer raw network bytes, decode complete lines and append actual `message.content` deltas to the existing assistant draft. TCP boundaries do not equal JSON records or UTF-8 character boundaries.

Use background worker + bounded event channel to egui. Keep the UI thread responsive. Track request ID and state: connecting, loading, generating, completed, interrupted, cancelled, failed. Redraw on genuine deltas, avoid accidental full-paragraph buffering and duplication. Honor `done` and capture usage metrics when present. Cancellation must stop the stream; retries create new generation IDs. On failure keep partial content labeled interrupted and show actionable error.

A model cold start may be slow; show an honest loading state rather than fake words. Measure time-to-first-token, model load duration, tokens/sec and full latency. Use Ollama `keep_alive` only after measuring RAM/VRAM and concurrent CUDA inference pressure. A ready sidebar GPU badge is not proof that Ollama used GPU.

Sources: https://docs.ollama.com/api/chat and https://docs.ollama.com/api/streaming
