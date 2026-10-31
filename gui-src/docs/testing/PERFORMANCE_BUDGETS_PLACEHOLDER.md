# Performance Budgets Placeholder

Performance thresholds to be verified during integration phases:

| Metric | Budget Target | Measurement Method |
| :--- | :--- | :--- |
| **Initial Desktop Window Render** | < 800 ms | Tauri webview DOMContentLoaded |
| **Route Transition Time** | < 50 ms | React Profiler |
| **Production JavaScript Bundle** | < 350 KB (gzipped) | Vite build output report |
| **Frame Rate During Map Pan** | 60 fps steady | Chrome DevTools Performance panel |
| **Memory Consumption (Idle)** | < 120 MB | Windows Task Manager / Activity Monitor |
