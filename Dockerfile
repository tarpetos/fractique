# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.97.1

FROM rust:${RUST_VERSION}-slim-bookworm AS server
ARG JOBS
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,id=fractique-server-target,target=/src/target \
    cargo build --release --locked -p fractique-server ${JOBS:+--jobs "$JOBS"} \
    && cp target/release/fractique-server /fractique-server

FROM rust:${RUST_VERSION}-slim-bookworm AS web
ARG TRUNK_VERSION=0.22.1
ARG JOBS
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add wasm32-unknown-unknown \
    && curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-$(uname -m)-unknown-linux-musl.tar.gz" \
       | tar -xz -C /usr/local/bin
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,id=fractique-web-target,target=/src/target \
    cd crates/fractique-app \
    && if [ -n "$JOBS" ]; then export CARGO_BUILD_JOBS="$JOBS"; fi \
    && trunk build --release --locked \
    && gzip -9 -k dist/*.wasm dist/*.js \
    && cp -r dist /web

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=server /fractique-server /usr/local/bin/fractique-server
COPY --from=web /web /srv/fractique
ENV FRACTIQUE_BIND=0.0.0.0:8080 \
    FRACTIQUE_WEB=/srv/fractique
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/fractique-server"]
