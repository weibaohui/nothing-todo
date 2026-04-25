.PHONY: setup build clean run dev watch kill-port install start stop

# Setup: install all dependencies for frontend and backend
setup:
	@echo "=== Setting up aietodo ==="
	@echo ""
	@echo "[1/4] Checking Rust toolchain..."
	@which rustc > /dev/null 2>&1 || (echo "Installing Rust..." && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y)
	@source $$HOME/.cargo/env 2>/dev/null || true
	@echo "  Rust: $$(rustc --version 2>/dev/null || echo 'NOT FOUND')"
	@echo ""
	@echo "[2/4] Checking Node.js..."
	@echo "  Node: $$(node --version 2>/dev/null || echo 'NOT FOUND')"
	@echo "  npm:  $$(npm --version 2>/dev/null || echo 'NOT FOUND')"
	@echo ""
	@echo "[3/4] Installing frontend dependencies..."
	cd frontend && npm install
	@echo ""
	@echo "[4/4] Pre-compiling Rust backend (downloads deps)..."
	cd backend && source $$HOME/.cargo/env 2>/dev/null && cargo fetch
	@echo ""
	@echo "=== Setup complete! ==="
	@echo "Run 'make dev'    to start development (frontend + backend)"
	@echo "Run 'make watch'  to start with hot reload"
	@echo "Run 'make build'  to build for production"
	@echo "Run 'make install' to build and install binary to ~/.local/bin"

# Install the built binary to ~/.local/bin
install: stop build
	@mkdir -p $$HOME/.local/bin
	@rm -f $$HOME/.local/bin/aitodo
	@cp backend/target/release/aitodo $$HOME/.local/bin/
	@echo "Installed to $$HOME/.local/bin/aitodo"
	@echo "Make sure $$HOME/.local/bin is in your PATH"

# Stop the aitodo binary
stop:
	-@if [ -f ~/.aitodo/run.pid ]; then \
		pid=$$(cat ~/.aitodo/run.pid); \
		kill -9 $$pid 2>/dev/null && echo "Killed process $$pid" || echo "Process $$pid not running"; \
		rm -f ~/.aitodo/run.pid; \
	else \
		pkill -9 -f "^aitodo$$" 2>/dev/null || echo "aitodo process not running"; \
	fi
	@sleep 1

# Start the aitodo binary (after installing)
start: install
	@mkdir -p $$HOME/.aitodo
	@( $$HOME/.local/bin/aitodo >> $$HOME/.aitodo/run.log 2>&1 & echo $$! > $$HOME/.aitodo/run.pid )
	@echo "aitodo started (PID: $$(cat $$HOME/.aitodo/run.pid)), logs: ~/.aitodo/run.log"

# Restart: clean install and start fresh
restart:
	-@if [ -f ~/.aitodo/run.pid ]; then \
		pid=$$(cat ~/.aitodo/run.pid); \
		kill -9 $$pid 2>/dev/null && echo "Killed process $$pid" || echo "Process $$pid not running"; \
		rm -f ~/.aitodo/run.pid; \
	fi
	-@pkill -9 -f "^aitodo$$" 2>/dev/null || true
	@sleep 1
	@rm -f $$HOME/.local/bin/aitodo
	@cd frontend && npm run build
	@cd backend && cargo build --release
	@mkdir -p $$HOME/.local/bin
	@cp backend/target/release/aitodo $$HOME/.local/bin/
	@( $$HOME/.local/bin/aitodo >> $$HOME/.aitodo/run.log 2>&1 & echo $$! > $$HOME/.aitodo/run.pid )
	@echo "aitodo rebuilt and started (PID: $$(cat $$HOME/.aitodo/run.pid))"

# Kill processes on ports used by dev servers
kill-port:
	@fuser -k 8088/tcp 2>/dev/null || true
	@fuser -k 5173/tcp 2>/dev/null || true

# Build frontend and embed into Rust binary
build:
	cd frontend && npm run build
	cd backend && cargo build --release

# Clean all build artifacts
clean:
	rm -rf frontend/dist
	rm -rf backend/target

# Run the server (after build)
run:
	./backend/target/release/aitodo

# Development mode (both frontend and backend, one-shot)
dev: kill-port build
	(cd backend && RUST_LOG=info cargo run) &
	@echo "Frontend: http://localhost:5173"
	@echo "Backend:  http://localhost:8088"

# Watch mode - frontend hot reload + backend auto-reload
watch: kill-port
	(cd frontend && npm run dev) &
	@(cd backend && RUST_BACKTRACE=1 RUST_LOG=info cargo watch -x run 2>&1 | tee ../backend.log) &
	@echo "Frontend: http://localhost:5173"
	@echo "Backend:  http://localhost:8088 (watching for changes...)"
	@echo "Backend logs: tail -f backend.log"
	@echo ""
	@echo "Press Ctrl+C to stop"
