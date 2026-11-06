FROM node:22-trixie-slim AS frontend-build
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/index.html frontend/tsconfig.json frontend/vite.config.ts ./
COPY frontend/src ./src
RUN npm run build

FROM rust:1.99-slim-trixie AS backend-build
WORKDIR /src/backend
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/backend/target \
    cargo build --release --locked && \
    install -D target/release/feeder /out/feeder

# Optional Linux-only test target; the runtime build does not carry test tools.
FROM backend-build AS backend-test
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/backend/target \
    cargo test --locked

FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates curl libgcc-s1 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 feeder \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin feeder \
    && install -d -o 10001 -g 10001 /data /app/frontend/dist
WORKDIR /app
COPY --from=backend-build --chown=10001:10001 /out/feeder /usr/local/bin/feeder
COPY --from=frontend-build --chown=10001:10001 /src/frontend/dist /app/frontend/dist
ENV FEEDER_BIND=0.0.0.0:8080 \
    FEEDER_DB=/data/feeder.sqlite3 \
    FEEDER_DEMO=0
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/feeder"]
CMD []
