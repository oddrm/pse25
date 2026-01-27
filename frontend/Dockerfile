# Stage 1: Build
FROM node:20-slim AS builder

WORKDIR /src

# Copy yarn files first for better layer caching
COPY package.json yarn.lock ./
RUN yarn install --frozen-lockfile

# Copy the rest of the source code
COPY . .

RUN yarn nuxi prepare

# Build the Nuxt application
RUN yarn build

# Stage 2: Runner
FROM node:20-slim AS runner

WORKDIR /app

# Install curl so the container healthcheck (curl) works during runtime
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

# Nitro output includes everything needed to run the server
COPY --from=builder /src/.output ./.output

# Environment setup
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000

EXPOSE 3000

# Nuxt 4/Nitro entry point
CMD ["node", ".output/server/index.mjs"]