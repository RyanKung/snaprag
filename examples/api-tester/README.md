# 🦀 SnapRAG API Tester

A beautiful Rust/Yew-based web application for testing SnapRAG API endpoints with MetaMask integration.

## Features

- ✅ **Pure Rust** - Built with Yew, runs as WebAssembly
- 💳 **MetaMask Integration** - Connect wallet for payment-enabled endpoints
- 🎨 **Beautiful UI** - Modern, responsive design
- 📡 **All Endpoints** - Test all SnapRAG API endpoints
- 🔒 **x402 Payment** - Test payment-required endpoints (coming soon)
- 📊 **Response Viewer** - Pretty-printed JSON with headers

## Prerequisites

```bash
# Install Trunk (Rust WASM bundler)
cargo install trunk

# Add WASM target
rustup target add wasm32-unknown-unknown
```

## Quick Start

### 1. Start SnapRAG API Server

```bash
# From project root
cd /Users/ryan/Dev/farcaster/snaprag
cargo run -- serve api --host 127.0.0.1 --port 3000
```

### 2. Start API Tester

```bash
# In another terminal
cd examples/api-tester
trunk serve --open
```

The tester will open at `http://127.0.0.1:8080`

## Usage

### Testing Free Endpoints

1. Select an endpoint from the sidebar (e.g., "Health Check")
2. Click "Send Request"
3. View the response

### Testing with MetaMask

1. Click "Connect MetaMask" in the wallet section
2. Approve the connection in MetaMask
3. Select a payment-required endpoint (Basic/Premium/Enterprise tier)
4. Click "Send Request"
5. (Payment flow coming soon)

## Endpoint Tiers

- 🟢 **Free** - No payment required (health, stats, MCP)
- 🔵 **Basic** - 0.001 USDC (profiles, get profile)
- 🟣 **Premium** - 0.01 USDC (search)
- 🟠 **Enterprise** - 0.1 USDC (RAG query)

## Development

### Project Structure

```
api-tester/
├── src/
│   ├── main.rs         # Main Yew application
│   ├── api.rs          # API calling logic
│   └── wallet.rs       # MetaMask integration
├── js/
│   └── wallet.js       # JavaScript wallet bridge
├── index.html          # HTML template
├── Cargo.toml          # Rust dependencies
└── Trunk.toml          # Build configuration
```

### Building

```bash
# Development build
trunk build

# Production build
trunk build --release

# Output goes to dist/
```

### Testing

```bash
# Run with live reload
trunk serve

# Or specify port
trunk serve --port 8080
```

## Technologies

- **Yew** - Rust/WebAssembly frontend framework
- **wasm-bindgen** - Rust-JavaScript interop
- **web-sys** - Web API bindings
- **MetaMask** - Ethereum wallet (via window.ethereum)
- **Trunk** - WASM web application bundler

## Browser Compatibility

- ✅ Chrome/Edge (recommended)
- ✅ Firefox
- ✅ Safari
- ⚠️ Requires MetaMask extension for wallet features

## Troubleshooting

### "MetaMask not detected"
- Install [MetaMask extension](https://metamask.io/)
- Refresh the page after installation

### "Failed to fetch"
- Ensure SnapRAG API server is running at `http://127.0.0.1:3000`
- Check the API Base URL in the tester

### CORS errors
- API server should have CORS enabled (it does by default)
- Check browser console for details

## Future Features

- [ ] x402 payment flow integration
- [ ] EIP-712 signature creation
- [ ] Payment history tracking
- [ ] Request/response history
- [ ] Export as cURL commands
- [ ] Request collections/favorites
- [ ] Dark mode

## License

Same as SnapRAG project

---

**Built with 🦀 Rust and ❤️ for the Farcaster community**

