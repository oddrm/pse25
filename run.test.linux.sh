# Build locally
cd backend && RUSTFLAGS="-A warnings" cargo test --no-run && cd ..

# Run in container
sudo docker compose -f compose.test.yaml up --no-attach db --remove-orphans