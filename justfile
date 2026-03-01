# Default: run all checks (same as CI)
default: check

# Run all CI checks (dashboard must build first for rust-embed)
check: dashboard-build fmt clippy test

# Format check
fmt:
    cargo fmt --all -- --check

# Format fix
fmt-fix:
    cargo fmt --all

# Clippy lint
clippy:
    cargo clippy --workspace -- -D warnings

# Run tests
test:
    cargo test --workspace

# Build dashboard frontend
dashboard-build:
    npm run build --prefix dashboard

# Install dashboard dependencies
dashboard-install:
    npm ci --prefix dashboard

# Dev build
build:
    cargo build --workspace

# Release build
release:
    cargo build --release

# Run daemon in foreground (for dev)
dev:
    cargo run -- daemon start --foreground

# Clean all build artifacts
clean:
    cargo clean
    rm -rf dashboard/dist

# Setup: install deps + git hooks
setup:
    git config core.hooksPath .githooks
    npm ci --prefix dashboard
    @echo "Done. Git hooks and dashboard deps installed."
