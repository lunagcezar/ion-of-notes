# syntax=docker/dockerfile:1

# Server-only image: no eframe/egui/wgpu, no window, no X server.
FROM rust:1.99-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY locales ./locales
COPY tests ./tests

RUN cargo build --release --locked --no-default-features --bin ion-of-notes-server

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc AS runtime

# rusqlite is bundled, so SQLite needs no runtime package. curl is only for
# the healthcheck; ca-certificates is for outgoing TLS webhooks.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl passwd \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --uid 10001 ion

COPY --from=builder /src/target/release/ion-of-notes-server /usr/local/bin/ion-of-notes-server

RUN mkdir -p /data && chown ion:ion /data
USER ion

EXPOSE 8787
VOLUME ["/data"]

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8787/health || exit 1

ENTRYPOINT ["ion-of-notes-server"]
CMD ["--serve", "0.0.0.0:8787", "--database", "/data/ion-of-notes.sqlite3"]
