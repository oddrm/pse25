FROM docker.io/rust:1-slim-bookworm AS build

## co package name: customize here or provide via --build-arg
ARG pkg=backend

WORKDIR /build

# 1. Install build-time dependencies
RUN apt-get update && apt-get install -y \
    libpq-dev \
    gcc \
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
    # Build only the requested binary to prevent cargo from attempting to
    # build a missing library target.
    cargo build --release --bin ${pkg} || cargo build --release; \
    objcopy --compress-debug-sections target/release/$pkg ./main

################################################################################

FROM docker.io/debian:bookworm-slim

# 2. Install runtime dependencies (libpq5 is required to run the binary)
RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

## copy the main binary
COPY --from=build /build/main ./

## copy runtime assets which may or may not exist
COPY --from=build /build/Rocket.tom[l] ./static
COPY --from=build /build/stati[c] ./static
COPY --from=build /build/template[s] ./templates

# Install curl so the container healthcheck (curl) works during runtime
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

RUN mkdir /data

## ensure the container listens globally on port 8080
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8080
ENV DATABASE_URL=postgresql://postgres:postgres@localhost:5432/postgres

CMD ./main
