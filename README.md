# svg2png-api

High-performance SVG to PNG conversion API built with Rust + [resvg](https://github.com/linebender/resvg). Full CJK (Chinese/Japanese/Korean) font support via Noto Sans SC.

## Features

- Fast SVG → PNG rendering powered by resvg (Rust)
- Chinese / CJK text rendering with bundled Noto Sans SC font
- Scale factor support (e.g. 2x for retina)
- Optional background color fill
- Fetch SVG from URL or POST raw SVG body

## API

### `POST /convert`

**Input method 1** — POST raw SVG:

```bash
curl -X POST https://your-service.onrender.com/convert \
  --data-binary @input.svg \
  -o output.png
```

**Input method 2** — SVG from URL:

```bash
curl -X POST "https://your-service.onrender.com/convert?url=https://example.com/icon.svg" \
  -o output.png
```

**Query parameters:**

| Param | Type   | Default | Description                            |
| ----- | ------ | ------- | -------------------------------------- |
| `url` | string | —       | Fetch SVG from this URL                |
| `scale` | float | `1.0`  | Output scale (0.1–10, e.g. 2 for retina) |
| `bg`  | string | —       | Background color hex, e.g. `ffffff`    |

### `GET /health`

Returns `ok` — used by Render health checks.

## Deploy to Render

### Option A: Blueprint (recommended)

1. Push this repo to GitHub
2. Go to [Render Dashboard](https://dashboard.render.com)
3. Click **New → Blueprint** and connect your repo
4. Render will use `render.yaml` to create the service automatically

### Option B: Manual

1. Go to Render Dashboard → **New → Web Service**
2. Connect your GitHub repo
3. Settings:
   - **Runtime**: Rust
   - **Build Command**: `bash build.sh`
   - **Start Command**: `./target/release/svg2png-api`
   - **Plan**: Free (or Starter for always-on)
4. Click **Deploy**

### Option C: Docker

1. Build: `docker build -t svg2png-api .`
2. Run: `docker run -p 3000:3000 svg2png-api`
3. Or push to Docker Hub and deploy on Render as a Docker service

## Local Development

```bash
# Download CJK font (one-time)
mkdir -p fonts
curl -L -o fonts/NotoSansSC-Regular.otf \
  "https://github.com/notofonts/noto-cjk/raw/main/Sans/OTF/SimplifiedChinese/NotoSansSC-Regular.otf"

# Run dev server
cargo run

# Test with a Chinese SVG
curl -X POST http://localhost:3000/convert \
  --data-binary '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="60"><text x="10" y="40" font-size="32">你好世界</text></svg>' \
  -o chinese.png
```

## Project Structure

```
svg2png-api/
├── Cargo.toml       # Rust dependencies
├── src/
│   └── main.rs      # Axum server + resvg rendering
├── fonts/           # CJK fonts (downloaded at build time)
├── build.sh         # Render build script
├── render.yaml      # Render Blueprint config
├── Dockerfile       # Docker alternative
└── .gitignore
```

## License

MIT
