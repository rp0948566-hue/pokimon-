use axum::{
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerHealth {
    pub status: &'static str,
    pub server: &'static str,
    pub language: &'static str,
    pub mode: &'static str,
    pub version: &'static str,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IncidentItem {
    pub id: &'static str,
    pub code: &'static str,
    pub title: &'static str,
    pub severity: &'static str,
    pub status: &'static str,
}

const STATIC_CANVAS_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>PokéRescue Sentinel</title>
  <style>
    *, *::before, *::after {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }
    html, body {
      width: 100%;
      height: 100%;
      overflow: hidden;
      font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background-color: #0c0c0e;
      -webkit-font-smoothing: antialiased;
    }
    /* Global Outer-Edge / Inset-Shell Treatment */
    .global-app-shell {
      width: 100vw;
      height: 100vh;
      height: 100dvh;
      padding: 8px; /* 8-10px space on all four sides */
      background-color: #0c0c0e; /* Subtle dark outer background */
      display: flex;
      box-sizing: border-box;
      overflow: hidden;
    }
    .global-app-shell-inner {
      width: 100%;
      height: 100%;
      flex: 1;
      border-radius: 12px; /* 12-14px rounded corners */
      border: 1px solid rgba(255, 255, 255, 0.22); /* Clearly visible 1px neutral border */
      overflow: hidden; /* Content cannot visually escape through corners */
      position: relative;
      display: flex;
      box-sizing: border-box;
    }
    /* Overlay border ensures outer rounded frame remains visible even when sidebar drawer is open */
    .global-app-shell-inner::after {
      content: "";
      position: absolute;
      inset: 0;
      border-radius: 12px;
      border: 1px solid rgba(255, 255, 255, 0.22);
      pointer-events: none;
      z-index: 30;
      box-sizing: border-box;
    }
    /* Page Outer Backdrop */
    .page-backdrop {
      width: 100%;
      height: 100%;
      overflow: hidden;
      background-color: transparent;
      display: flex;
      padding: 0;
      box-sizing: border-box;
    }
    /* Main Outer Frame with Dotted Background — Full Viewport */
    .main-frame {
      width: 100%;
      height: 100%;
      flex: 1;
      border: none;
      border-radius: 0;
      background-color: #D8D8D8;
      background-image: radial-gradient(circle, #9A9A9A 0.85px, transparent 0.85px);
      background-size: 6px 6px;
      position: relative;
      overflow: hidden;
      box-sizing: border-box;
    }
    /* Top-Left Animated Sidebar Toggle (Image 1 Default -> Image 2 Hover Smooth Animation) */
    .top-nav-sidebar-toggle {
      position: absolute;
      top: 14px;
      left: 14px;
      width: 32px;
      height: 32px;
      padding: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      background: rgba(22, 22, 26, 0.85);
      backdrop-filter: blur(12px);
      -webkit-backdrop-filter: blur(12px);
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 8px;
      color: #c0c0c6;
      cursor: pointer;
      z-index: 20;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
      transition: background-color 0.25s ease, border-color 0.25s ease, color 0.25s ease, transform 0.15s ease, box-shadow 0.25s ease;
    }
    .top-nav-sidebar-toggle:hover {
      background: rgba(38, 38, 44, 0.95);
      border-color: rgba(255, 255, 255, 0.25);
      color: #ffffff;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
    }
    .top-nav-sidebar-toggle:active {
      transform: scale(0.95);
    }
    .sidebar-icon {
      display: block;
      overflow: visible;
    }
    .sidebar-icon-box {
      stroke: currentColor;
      stroke-width: 1.6;
    }
    .sidebar-icon-line-full {
      stroke: currentColor;
      stroke-width: 1.6;
      transition: opacity 0.22s cubic-bezier(0.16, 1, 0.3, 1), transform 0.22s cubic-bezier(0.16, 1, 0.3, 1);
    }
    .sidebar-icon-line-short {
      stroke: currentColor;
      stroke-width: 1.6;
      stroke-linecap: round;
      opacity: 0;
      transition: opacity 0.22s cubic-bezier(0.16, 1, 0.3, 1);
    }
    .sidebar-icon-arrow {
      stroke: currentColor;
      stroke-width: 1.6;
      stroke-linecap: round;
      stroke-linejoin: round;
      opacity: 0;
      transform: translateX(-3px);
      transition: opacity 0.24s cubic-bezier(0.16, 1, 0.3, 1), transform 0.24s cubic-bezier(0.16, 1, 0.3, 1);
    }
    .top-nav-sidebar-toggle:hover .sidebar-icon-line-full {
      opacity: 0;
    }
    .top-nav-sidebar-toggle:hover .sidebar-icon-line-short {
      opacity: 1;
    }
    .top-nav-sidebar-toggle:hover .sidebar-icon-arrow {
      opacity: 1;
      transform: translateX(0);
    }
    /* Collapsible Sliding Sidebar Drawer (Matching uploaded reference image) */
    .collapsible-sidebar-drawer {
      position: absolute;
      top: 0;
      left: 0;
      bottom: 0;
      width: clamp(260px, 22vw, 320px);
      height: 100%;
      background: #191a1a;
      background: rgba(25, 26, 26, 0.96);
      backdrop-filter: blur(20px) saturate(180%);
      -webkit-backdrop-filter: blur(20px) saturate(180%);
      border-right: 1px solid rgba(255, 255, 255, 0.1);
      border-top-left-radius: 12px;
      border-bottom-left-radius: 12px;
      border-top-right-radius: 14px;
      border-bottom-right-radius: 14px;
      box-shadow: 8px 0 32px rgba(0, 0, 0, 0.35);
      transform: translateX(-100%);
      transition: transform 0.35s cubic-bezier(0.16, 1, 0.3, 1), box-shadow 0.35s ease;
      z-index: 15;
      overflow: hidden;
      box-sizing: border-box;
    }
    .collapsible-sidebar-drawer.open {
      transform: translateX(0);
    }
    /* Bottom-Center Apple Glass Dock Bar Widget with Full Reflections (Reference media_1793025780926_932e0617.jpg) */
    .apple-glass-dock-widget,
    .top-right-glass-widget {
      position: absolute;
      bottom: 24px;
      left: 50%;
      top: auto;
      right: auto;
      transform: translateX(-50%);
      width: min(540px, calc(100% - 48px));
      border-radius: 26px;
      background: rgba(18, 22, 30, 0.78);
      background-image:
        radial-gradient(ellipse at 15% 15%, rgba(145, 200, 255, 0.22) 0%, transparent 60%),
        radial-gradient(ellipse at 85% 85%, rgba(255, 200, 140, 0.18) 0%, transparent 60%),
        linear-gradient(135deg, rgba(255, 255, 255, 0.18) 0%, rgba(255, 255, 255, 0.05) 35%, rgba(255, 255, 255, 0.0) 65%);
      backdrop-filter: blur(28px) saturate(200%);
      -webkit-backdrop-filter: blur(28px) saturate(200%);
      border: 1px solid rgba(255, 255, 255, 0.15);
      box-shadow:
        -1.5px -1.5px 0 0 rgba(145, 200, 255, 0.8),
        1.5px 1.5px 0 0 rgba(255, 220, 170, 0.8),
        inset 0 1px 1.5px 0 rgba(255, 255, 255, 0.45),
        -8px -8px 24px 0 rgba(100, 180, 255, 0.2),
        8px 8px 30px 0 rgba(255, 190, 120, 0.2),
        0 20px 48px -8px rgba(0, 0, 0, 0.5),
        0 0 24px rgba(255, 255, 255, 0.08);
      z-index: 10;
      box-sizing: border-box;
      overflow: hidden;
      transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1), box-shadow 0.3s ease;
    }
    .apple-glass-dock-widget::before,
    .top-right-glass-widget::before {
      content: "";
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 52%;
      background: linear-gradient(
        180deg,
        rgba(255, 255, 255, 0.26) 0%,
        rgba(255, 255, 255, 0.08) 60%,
        transparent 100%
      );
      border-top-left-radius: inherit;
      border-top-right-radius: inherit;
      pointer-events: none;
      z-index: 1;
    }
    .apple-glass-dock-widget::after,
    .top-right-glass-widget::after {
      content: "";
      position: absolute;
      top: 1px;
      left: 10%;
      right: 10%;
      height: 1px;
      background: linear-gradient(90deg, transparent 0%, rgba(255, 255, 255, 0.9) 50%, transparent 100%);
      pointer-events: none;
      filter: blur(0.4px);
      z-index: 2;
    }
    .apple-glass-dock-widget:hover,
    .top-right-glass-widget:hover {
      transform: translateX(-50%) translateY(-3px);
      box-shadow:
        -1.5px -1.5px 0 0 rgba(145, 200, 255, 0.95),
        1.5px 1.5px 0 0 rgba(255, 220, 170, 0.95),
        inset 0 1.5px 2px 0 rgba(255, 255, 255, 0.6),
        -10px -10px 32px 0 rgba(100, 180, 255, 0.28),
        10px 10px 36px 0 rgba(255, 190, 120, 0.28),
        0 26px 56px -6px rgba(0, 0, 0, 0.55),
        0 0 32px rgba(255, 255, 255, 0.14);
    }
    .glass-widget-grid {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 12px;
      padding: 12px 14px;
      box-sizing: border-box;
      position: relative;
      z-index: 3;
    }
    .glass-tile {
      aspect-ratio: 16 / 10;
      min-height: 68px;
      border-radius: 16px;
      background: rgba(255, 255, 255, 0.05);
      background-image: linear-gradient(135deg, rgba(255, 255, 255, 0.14) 0%, rgba(255, 255, 255, 0.02) 100%);
      border: 1px solid rgba(255, 255, 255, 0.14);
      box-shadow: inset 0 1px 1.5px rgba(255, 255, 255, 0.25), 0 4px 12px rgba(0, 0, 0, 0.22);
      cursor: pointer;
      box-sizing: border-box;
      position: relative;
      overflow: hidden;
      transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1), background 0.25s ease, border-color 0.25s ease, box-shadow 0.25s ease;
    }
    .glass-tile::before {
      content: "";
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 50%;
      background: linear-gradient(180deg, rgba(255, 255, 255, 0.24) 0%, rgba(255, 255, 255, 0.02) 100%);
      border-top-left-radius: inherit;
      border-top-right-radius: inherit;
      pointer-events: none;
    }
    .glass-tile-tl {
      box-shadow: inset 1px 1px 0 0 rgba(145, 200, 255, 0.55), inset 0 1px 1.5px rgba(255, 255, 255, 0.25), 0 4px 12px rgba(0, 0, 0, 0.22);
    }
    .glass-tile-br {
      box-shadow: inset -1px -1px 0 0 rgba(255, 220, 170, 0.55), inset 0 1px 1.5px rgba(255, 255, 255, 0.25), 0 4px 12px rgba(0, 0, 0, 0.22);
    }
    .glass-tile:hover {
      transform: translateY(-4px);
      background: rgba(255, 255, 255, 0.09);
      background-image: linear-gradient(135deg, rgba(255, 255, 255, 0.22) 0%, rgba(255, 255, 255, 0.05) 100%);
      border-color: rgba(255, 255, 255, 0.3);
      box-shadow: inset 0 1.5px 2px rgba(255, 255, 255, 0.45), 0 10px 24px rgba(0, 0, 0, 0.35);
    }
    .glass-tile:active {
      transform: translateY(-1px) scale(0.98);
    }
  </style>
</head>
<body>
  <div class="global-app-shell">
    <div class="global-app-shell-inner">
      <div class="page-backdrop">
        <div class="main-frame">
          <button type="button" class="top-nav-sidebar-toggle" aria-label="Toggle Sidebar" title="Toggle Sidebar">
            <svg class="sidebar-icon" viewBox="0 0 20 20" width="18" height="18" fill="none">
              <rect class="sidebar-icon-box" x="2.5" y="2.5" width="15" height="15" rx="3.5" />
              <line class="sidebar-icon-line-full" x1="7" y1="2.5" x2="7" y2="17.5" />
              <line class="sidebar-icon-line-short" x1="7" y1="5.5" x2="7" y2="14.5" />
              <path class="sidebar-icon-arrow" d="M10.5 7 L13.5 10 L10.5 13" />
            </svg>
          </button>
          <aside id="sidebar-drawer" class="collapsible-sidebar-drawer"></aside>
          <aside class="apple-glass-dock-widget top-right-glass-widget" aria-label="Apple Glass Dock Widget">
            <div class="glass-widget-grid">
              <div class="glass-tile glass-tile-tl"></div>
              <div class="glass-tile glass-tile-2"></div>
              <div class="glass-tile glass-tile-3"></div>
              <div class="glass-tile glass-tile-br"></div>
            </div>
          </aside>
        </div>
      </div>
    </div>
  </div>
  <script>
    const btn = document.querySelector('.top-nav-sidebar-toggle');
    const drawer = document.getElementById('sidebar-drawer');
    if (btn && drawer) {
      btn.addEventListener('click', () => drawer.classList.toggle('open'));
    }
  </script>
</body>
</html>"#;

async fn render_blank_canvas() -> impl IntoResponse {
    let dist_index = "apps/desktop/dist/index.html";
    if let Ok(content) = std::fs::read_to_string(dist_index) {
        Html(content).into_response()
    } else {
        Html(STATIC_CANVAS_HTML).into_response()
    }
}

async fn serve_asset(axum::extract::Path(file): axum::extract::Path<String>) -> impl IntoResponse {
    let path = format!("apps/desktop/dist/assets/{}", file);
    if let Ok(bytes) = std::fs::read(&path) {
        let mime = if file.ends_with(".js") {
            "application/javascript"
        } else if file.ends_with(".css") {
            "text/css"
        } else if file.ends_with(".png") {
            "image/png"
        } else {
            "application/octet-stream"
        };
        ([(axum::http::header::CONTENT_TYPE, mime)], bytes).into_response()
    } else {
        axum::http::StatusCode::NOT_FOUND.into_response()
    }
}

async fn health_check() -> impl IntoResponse {
    Json(ServerHealth {
        status: "ok",
        server: "retinacare-sentinel",
        language: "rust",
        mode: "standalone-executable",
        version: "0.2.0",
    })
}

async fn list_incidents() -> impl IntoResponse {
    let incidents = vec![IncidentItem {
        id: "inc-001",
        code: "ALPHA-1",
        title: "RetinaCare Clinical Telemetry Signal",
        severity: "LOW",
        status: "REPORTED",
    }];
    Json(incidents)
}

pub fn create_router() -> Router {
    Router::new()
        .route("/", get(render_blank_canvas))
        .route("/assets/{file}", get(serve_asset))
        .route("/api/health", get(health_check))
        .route("/api/v1/incidents", get(list_incidents))
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let app = create_router();

    println!("==================================================");
    println!(" PokéRescue Sentinel &mdash; Rust Standalone Server");
    println!(" Running on http://{}", addr);
    println!(" Canvas: Static Dot-Grid (#D8D8D8 / #9A9A9A / 6px)");
    println!("==================================================");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_html_contains_dot_grid_spec() {
        assert!(STATIC_CANVAS_HTML.contains("global-app-shell"));
        assert!(STATIC_CANVAS_HTML.contains("global-app-shell-inner"));
        assert!(STATIC_CANVAS_HTML.contains("rgba(255, 255, 255, 0.22)"));
        assert!(STATIC_CANVAS_HTML.contains("#D8D8D8"));
        assert!(STATIC_CANVAS_HTML.contains("#9A9A9A"));
        assert!(STATIC_CANVAS_HTML.contains("6px 6px"));
        assert!(STATIC_CANVAS_HTML.contains("main-frame"));
        assert!(STATIC_CANVAS_HTML.contains("border: none"));
        assert!(STATIC_CANVAS_HTML.contains("top-nav-sidebar-toggle"));
        assert!(STATIC_CANVAS_HTML.contains("sidebar-icon"));
        assert!(STATIC_CANVAS_HTML.contains("collapsible-sidebar-drawer"));
        assert!(STATIC_CANVAS_HTML.contains("top-right-glass-widget"));
        assert!(STATIC_CANVAS_HTML.contains("apple-glass-dock-widget"));
        assert!(STATIC_CANVAS_HTML.contains("glass-widget-grid"));
        assert!(STATIC_CANVAS_HTML.contains("glass-tile"));
    }

    #[test]
    fn test_router_creation() {
        let _router = create_router();
    }
}
