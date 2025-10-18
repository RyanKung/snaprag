.PHONY: help install dev build clean

help:
	@echo "SnapRAG API Tester - Makefile"
	@echo ""
	@echo "Commands:"
	@echo "  make install    - Install dependencies (Trunk)"
	@echo "  make dev        - Start development server"
	@echo "  make build      - Build for production"
	@echo "  make clean      - Clean build artifacts"

install:
	@echo "📦 Installing dependencies..."
	@command -v trunk >/dev/null 2>&1 || cargo install trunk
	@rustup target add wasm32-unknown-unknown
	@echo "✅ Done!"

dev:
	@echo "🚀 Starting development server..."
	@echo "   API Tester: http://127.0.0.1:8080"
	@trunk serve --open

build:
	@echo "🔨 Building for production..."
	@trunk build --release
	@echo "✅ Build complete! Output in dist/"

clean:
	@echo "🧹 Cleaning..."
	@rm -rf dist/
	@cargo clean
	@echo "✅ Clean complete!"

