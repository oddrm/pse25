#!/bin/bash
# Build locally (fast with local cargo cache)
cd backend && cargo build && cd ..

# Run in container (provides database and /data directory)
sudo docker compose -f compose.dev.yaml up --no-attach db --no-attach pgadmin --remove-orphans