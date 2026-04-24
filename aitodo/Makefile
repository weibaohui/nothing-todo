.PHONY: build clean run dev watch

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
	./backend/target/release/todo-executor

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
