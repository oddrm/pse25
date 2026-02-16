FROM docker.io/rust:latest AS build

## co package name: customize here or provide via --build-arg
ARG pkg=backend

WORKDIR /build

# 1. Install build-time dependencies
RUN apt-get update && apt-get install -y \
    libpq-dev \
    gcc \
    python3 \
    libpython3-dev \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

# 2. Copy dependency manifests first (better caching)
COPY Cargo.toml Cargo.lock ./

# 3. Fetch dependencies to populate the cargo cache without compiling the crate
# This avoids creating a dummy src file and prevents cargo from trying to
# compile a missing library target during the dependency-only step.
RUN cargo fetch

# 4. Now copy actual source code
COPY . .

# 5. Touch main.rs to ensure it rebuilds, then build for real
RUN --mount=type=cache,target=/build/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    set -eux; \
    touch src/main.rs; \
    # Build all binaries and download mcap CLI from GitHub releases
    cargo build --release --bins || cargo build --release; \
        mkdir -p /usr/local/cargo/bin; \
        curl -fsSL -o /usr/local/cargo/bin/mcap "https://github.com/foxglove/mcap/releases/download/releases/mcap-cli/v0.0.61/mcap-linux-amd64"; \
        chmod +x /usr/local/cargo/bin/mcap; \
    objcopy --compress-debug-sections target/release/$pkg ./main

################################################################################

FROM docker.io/debian:bookworm-slim

# 2. Install runtime dependencies (libpq5 is required to run the binary)
RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    python3 \
    libpython3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

## copy the main binary
COPY --from=build /build/main ./

## copy the mcap CLI downloaded in the builder into the runtime image
COPY --from=build /usr/local/cargo/bin/mcap /usr/local/bin/mcap

## copy runtime assets which may or may not exist
COPY --from=build /build/Rocket.tom[l] ./static
COPY --from=build /build/stati[c] ./static
COPY --from=build /build/template[s] ./templates

# Install curl so the container healthcheck (curl) works during runtime
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

RUN mkdir /data

## runtime checks removed

## ensure the container listens globally on port 8080
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8080
ENV DATABASE_URL=postgresql://postgres:postgres@localhost:5432/postgres

CMD ./main
