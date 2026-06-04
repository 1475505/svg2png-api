use axum::{
    body::Bytes,
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use once_cell::sync::OnceCell;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::fontdb;
use resvg::usvg::{self, Tree};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

// ---------------------------------------------------------------------------
// Global font database (loaded once at startup)
// ---------------------------------------------------------------------------
static FONT_DB: OnceCell<Arc<fontdb::Database>> = OnceCell::new();

// ---------------------------------------------------------------------------
// Query parameters for POST /convert
// ---------------------------------------------------------------------------
#[derive(Deserialize, Default)]
struct ConvertParams {
    /// Fetch SVG from this URL instead of reading the request body
    url: Option<String>,
    /// Output scale factor (default 1.0, e.g. 2.0 for retina)
    scale: Option<f32>,
    /// Background color in hex without '#' (e.g. "ffffff" for white)
    bg: Option<String>,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "svg2png_api=info,tower_http=info".into()),
        )
        .init();

    // Load fonts once
    let db = load_fonts();
    FONT_DB.set(Arc::new(db)).expect("font db init failed");
    info!("Font database ready");

    let app = Router::new()
        .route("/convert", post(convert))
        .route("/health", get(health));

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse()
        .expect("invalid PORT");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("svg2png-api listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------
async fn health() -> &'static str {
    "ok"
}

async fn convert(
    Query(params): Query<ConvertParams>,
    body: Bytes,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let scale = params.scale.unwrap_or(1.0).clamp(0.1, 10.0);

    // --- Obtain raw SVG bytes ---
    let svg_bytes: Vec<u8> = if let Some(ref url) = params.url {
        reqwest::get(url)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to fetch SVG URL: {e}")))?
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read SVG body: {e}")))?
            .to_vec()
    } else if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Empty body. Send SVG XML or use ?url=".into()));
    } else {
        body.to_vec()
    };

    // --- Build usvg options with loaded fonts ---
    let font_db = FONT_DB.get().expect("font db not initialized").clone();
    let mut opt = usvg::Options::default();
    opt.fontdb = font_db;

    // --- Parse SVG ---
    let tree = Tree::from_data(&svg_bytes, &opt)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("SVG parse error: {e}")))?;

    let svg_w = tree.size().width();
    let svg_h = tree.size().height();
    let px_w = (svg_w * scale) as u32;
    let px_h = (svg_h * scale) as u32;

    // --- Create pixmap ---
    let mut pixmap = Pixmap::new(px_w, px_h)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid output dimensions".into()))?;

    // Fill background if requested
    if let Some(ref hex) = params.bg {
        if let Some(color) = parse_hex_color(hex) {
            pixmap.fill(color);
        }
    }

    // --- Render ---
    let transform = Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // --- Encode PNG ---
    let png_data = pixmap
        .encode_png()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("PNG encode error: {e}")))?;

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("content-type", "image/png".parse().unwrap());
    resp_headers.insert(
        "cache-control",
        "public, max-age=3600".parse().unwrap(),
    );

    Ok((resp_headers, png_data))
}

// ---------------------------------------------------------------------------
// Font loading
// ---------------------------------------------------------------------------
fn load_fonts() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    info!("Loaded {} system fonts", db.len());

    // Try multiple candidate paths for bundled CJK fonts:
    //   1. ./fonts/              (Render CWD = project root)
    //   2. {exe_dir}/fonts/      (binary run directly)
    let exe_dir = app_dir();
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidates = [cwd.join("fonts"), exe_dir.join("fonts")];

    let mut fonts_loaded = false;
    for fonts_dir in &candidates {
        if !fonts_dir.is_dir() {
            continue;
        }
        let mut count = 0u32;
        for entry in std::fs::read_dir(fonts_dir).into_iter().flatten() {
            let path = match entry {
                Ok(e) => e.path(),
                Err(_) => continue,
            };
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext == "ttf" || ext == "otf" {
                match db.load_font_file(&path) {
                    Ok(_) => {
                        info!("Loaded font: {}", path.display());
                        count += 1;
                    }
                    Err(e) => error!("Skip font {}: {e}", path.display()),
                }
            }
        }
        info!(
            "Loaded {count} bundled CJK font(s) from {}",
            fonts_dir.display()
        );
        fonts_loaded = true;
        break;
    }

    if !fonts_loaded {
        error!("No fonts/ directory found. Chinese text may not render correctly.");
    }

    db
}

/// Returns the directory containing the running executable.
fn app_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
fn parse_hex_color(hex: &str) -> Option<resvg::tiny_skia::Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    resvg::tiny_skia::Color::from_rgba8(r, g, b, 255)
}
