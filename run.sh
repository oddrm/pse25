#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<EOF
Usage: $0 <dev|backend|frontend|e2e|prod> [--] [args...]

Commands:
  dev          Start the full stack in development watch mode
  backend      Run backend tests
  frontend     Run frontend unit & component tests
  e2e          Run end-to-end tests against full stack
  prod         Run the production compose stack

Currently only tested for linux, probably works for mac as well

Examples:
  $0 dev
  $0 backend
  $0 frontend
  $0 e2e
  $0 prod
EOF
}

if [[ ${1-} == "" || ${1-} == "-h" || ${1-} == "--help" ]]; then
  usage
  exit 0
fi

CMD=$1
shift || true

# Use sudo for docker if needed (empty when running as root or sudo not available)
SUDO=""
if [[ $(id -u) -ne 0 ]] && command -v sudo >/dev/null 2>&1; then
  SUDO=sudo
fi


case "$CMD" in
  backend)
    echo "Running backend tests..."
    # Build locally
    cd backend && RUSTFLAGS="-A warnings" cargo test --no-run && cd ..

    # Run in container
    "$SUDO" docker compose -f compose.backend.yaml up --no-attach db --build
    ;;
  frontend)
    echo "Running frontend unit & component tests..."
    pushd frontend >/dev/null
    npm run test
    popd >/dev/null
    ;;

  e2e)
    echo "Running E2E tests with full stack..."

    # Use docker compose to (re)build images and run the e2e stack.
    # This ensures images are rebuilt when Dockerfiles or contexts change.
    "$SUDO" docker compose -f compose.e2e.yaml up --build --exit-code-from playwright --remove-orphans playwright
    EXIT_CODE=$?
    "$SUDO" docker compose -f compose.e2e.yaml down
    exit $EXIT_CODE
    ;;

  dev)
    echo "Starting full development stack..."
    "$SUDO" docker compose -f compose.dev.yaml up --build --remove-orphans --no-attach db --no-attach pgadmin
    ;;

  prod)
    echo "Starting production compose stack..."
    "$SUDO" docker compose -f compose.prod.yaml up --build
    ;;

  *)
    echo "Unknown command: $CMD" >&2
    usage
    exit 2
    ;;
esac

exit 0
