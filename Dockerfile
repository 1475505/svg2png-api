# ---------- build stage ----------
FROM rust:1.82-slim AS builder

RUN apt-get update && apt-get install -y curl pkg-config && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Download CJK font
RUN mkdir -p fonts && \
    curl -fsSL -o fonts/NotoSansSC-Regular.otf \
    "https://github.com/notofonts/noto-cjk/raw/main/Sans/OTF/SimplifiedChinese/NotoSansSC-Regular.otf" \
    || echo "WARNING: CJK font download failed"

# Build release binary
RUN cargo build --release

# ---------- runtime stage ----------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary and fonts
COPY --from=builder /app/target/release/svg2png-api /app/svg2png-api
COPY --from=builder /app/fonts /app/fonts

ENV PORT=3000
EXPOSE 3000

CMD ["/app/svg2png-api"]
