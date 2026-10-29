FROM rust:1.93-slim-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --locked --release -p okuldayim-api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 okuldayim \
    && useradd --system --uid 10001 --gid okuldayim --no-create-home okuldayim
COPY --from=builder /build/target/release/okuldayim-api /usr/local/bin/okuldayim-api
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/okuldayim-api"]
