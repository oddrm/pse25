# Project README

Developer test runner notes

Running tests

 - Linux / macOS (Bash):
	 - Use `./run.tests.sh backend|frontend|e2e|all`

 - Windows (recommended):
	 - Use WSL2 and run the Bash script from the WSL environment for parity:

```bash
# from WSL in the repository directory
./run.tests.sh backend
```

Notes

 - Ensure `cargo`, `npm`, and Docker Desktop are installed and available in PATH (or installed in WSL).
 - The scripts expect Docker Compose v2 (`docker compose`). If you have legacy compose, adjust commands accordingly.
