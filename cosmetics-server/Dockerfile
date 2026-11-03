FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock catalog.json ./
COPY src src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /data cosmetics && mkdir -p /data && chown cosmetics /data
COPY --from=build /src/target/release/cairn-cosmetics-server /usr/local/bin/cairn-cosmetics-server
USER cosmetics
ENV COSMETICS_BIND=0.0.0.0:8787 COSMETICS_DATA_DIR=/data
VOLUME /data
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=3s CMD curl -fsS http://127.0.0.1:8787/healthz || exit 1
ENTRYPOINT ["cairn-cosmetics-server"]
CMD ["serve"]
