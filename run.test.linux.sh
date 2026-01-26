cd backend && cargo test --no-run && cd ..

sudo docker compose -f compose.test.yaml up --no-attach db --remove-orphans