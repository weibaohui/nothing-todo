.PHONY: setup build clean run dev watch kill-port install start stop cross-build cross-list

# Setup: install all dependencies for frontend and backend
setup:
	@echo "=== Setting up ntd ==="
	@echo ""
	@echo "[1/5] Checking Rust toolchain..."
	@which rustc > /dev/null 2>&1 || (echo "Installing Rust..." && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y)
	@source $$HOME/.cargo/env 2>/dev/null || true
	@echo "  Rust: $$(rustc --version 2>/dev/null || echo 'NOT FOUND')"
	@echo ""
	@echo "[2/5] Checking Node.js..."
	@echo "  Node: $$(node --version 2>/dev/null || echo 'NOT FOUND')"
	@echo "  npm:  $$(npm --version 2>/dev/null || echo 'NOT FOUND')"
	@echo ""
	@echo "[3/5] Installing frontend dependencies..."
	cd frontend && npm install
	@echo ""
	@echo "[4/5] Pre-compiling Rust backend (downloads deps)..."
	cd backend && source $$HOME/.cargo/env 2>/dev/null && cargo fetch
	@echo ""
	@echo "[5/5] Installing dev tools (cargo-watch)..."
	@source $$HOME/.cargo/env 2>/dev/null; which cargo-watch > /dev/null 2>&1 || cargo install cargo-watch
	@echo ""
	@echo "[OPT] Installing cross-build tool (cross)..."
	@source $$HOME/.cargo/env 2>/dev/null; which cross > /dev/null 2>&1 || cargo install cross --locked
	@echo ""
	@echo "=== Setup complete! ==="
	@echo "Run 'make dev'       to start development (frontend + backend)"
	@echo "Run 'make watch'     to start with hot reload"
	@echo "Run 'make build'     to build for production"
	@echo "Run 'make cross-build' to build for win/mac/linux x86+arm"
	@echo "Run 'make install'   to build and install binary to ~/.local/bin"

# Install the built binary to ~/.local/bin
install:  build
	@mkdir -p $$HOME/.local/bin
	@rm -f $$HOME/.local/bin/ntd
	@cp backend/target/release/ntd $$HOME/.local/bin/
	@echo "Installed to $$HOME/.local/bin/ntd"
	@echo "Make sure $$HOME/.local/bin is in your PATH"

# Stop the ntd binary
stop:
	-@if [ -f ~/.ntd/run.pid ]; then \
		pid=$$(cat ~/.ntd/run.pid); \
		kill -9 $$pid 2>/dev/null && echo "Killed process $$pid" || echo "Process $$pid not running"; \
		rm -f ~/.ntd/run.pid; \
	fi
	-@pkill -9 -x ntd 2>/dev/null && echo "Killed ntd processes" || true
	@sleep 1

# Start the ntd binary (after installing)
start: install
	@mkdir -p $$HOME/.ntd
	@( $$HOME/.local/bin/ntd >> $$HOME/.ntd/run.log 2>&1 & echo $$! > $$HOME/.ntd/run.pid )
	@echo "ntd started (PID: $$(cat $$HOME/.ntd/run.pid)), logs: ~/.ntd/run.log"

# Restart: clean install and start fresh
restart: stop install
	-@if [ -f ~/.ntd/run.pid ]; then \
		pid=$$(cat ~/.ntd/run.pid); \
		kill -9 $$pid 2>/dev/null && echo "Killed process $$pid" || echo "Process $$pid not running"; \
		rm -f ~/.ntd/run.pid; \
	fi
	-@pkill -9 -x ntd 2>/dev/null || true
	@sleep 1
	@rm -f $$HOME/.local/bin/ntd
	@cd frontend && npm run build
	@cd backend && cargo build --release
	@mkdir -p $$HOME/.local/bin
	@cp backend/target/release/ntd $$HOME/.local/bin/
	@( $$HOME/.local/bin/ntd >> $$HOME/.ntd/run.log 2>&1 & echo $$! > $$HOME/.ntd/run.pid )
	@echo "ntd rebuilt and started (PID: $$(cat $$HOME/.ntd/run.pid))"

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
	./backend/target/release/ntd

# Development mode - frontend hot reload + backend auto-reload
dev: kill-port
	(cd frontend && npm run dev) &
	(cd backend && RUST_BACKTRACE=1 RUST_LOG=info cargo watch -x run 2>&1 | tee ../backend.log) &
	@echo "Frontend: http://localhost:5173"
	@echo "Backend:  http://localhost:8088 (watching for changes...)"
	@echo "Backend logs: tail -f backend.log"
	@echo ""
	@echo "Press Ctrl+C to stop"

# Cross-build for Windows (x86_64 + i686), macOS (x86_64 + aarch64), Linux (x86_64 + aarch64)
cross-build:
	@echo "=== Cross-building ntd for win/mac/linux x86+arm ==="
	@mkdir -p backend/target/cross
	@echo ""
	@echo "[1/6] Building: x86_64-pc-windows-gnu"
	@cd backend && cross build --release --bin ntd --target x86_64-pc-windows-gnu --force-non-host
	@mv backend/target/x86_64-pc-windows-gnu/release/ntd.exe backend/target/cross/ntd-x86_64-pc-windows-gnu.exe
	@echo ""
	@echo "[2/6] Building: i686-pc-windows-gnu"
	@cd backend && cross build --release --bin ntd --target i686-pc-windows-gnu --force-non-host
	@mv backend/target/i686-pc-windows-gnu/release/ntd.exe backend/target/cross/ntd-i686-pc-windows-gnu.exe
	@echo ""
	@echo "[3/6] Building: x86_64-apple-darwin"
	@cd backend && cross build --release --bin ntd --target x86_64-apple-darwin
	@mv backend/target/x86_64-apple-darwin/release/ntd backend/target/cross/ntd-x86_64-apple-darwin
	@echo ""
	@echo "[4/6] Building: aarch64-apple-darwin"
	@cd backend && cross build --release --bin ntd --target aarch64-apple-darwin
	@mv backend/target/aarch64-apple-darwin/release/ntd backend/target/cross/ntd-aarch64-apple-darwin
	@echo ""
	@echo "[5/6] Building: x86_64-unknown-linux-gnu"
	@cd backend && cross build --release --bin ntd --target x86_64-unknown-linux-gnu
	@mv backend/target/x86_64-unknown-linux-gnu/release/ntd backend/target/cross/ntd-x86_64-unknown-linux-gnu
	@echo ""
	@echo "[6/6] Building: aarch64-unknown-linux-gnu"
	@cd backend && cross build --release --bin ntd --target aarch64-unknown-linux-gnu
	@mv backend/target/aarch64-unknown-linux-gnu/release/ntd backend/target/cross/ntd-aarch64-unknown-linux-gnu
	@echo ""
	@echo "=== Cross-build complete ==="
	@ls -lh backend/target/cross/

# List cross-build targets
cross-list:
	@echo "Cross-build targets:"
	@echo "  Windows:  x86_64-pc-windows-gnu, i686-pc-windows-gnu"
	@echo "  macOS:    x86_64-apple-darwin, aarch64-apple-darwin"
	@echo "  Linux:    x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu"
	@echo ""
	@echo "Built binaries: backend/target/cross/"

