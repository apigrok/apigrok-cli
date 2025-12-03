# APIGrok

A powerful, low-level multi-protocol API CLI tool built with Rust. APIGrok supports HTTP/1.1, HTTP/2, HTTP/3, WebSocket, and gRPC protocols from a single command-line interface.

## Features

- 🚀 **Multi-Protocol Support**: HTTP/1.1, HTTP/2, HTTP/3 (coming soon), WebSocket, and gRPC (coming soon)
- ⚡ **Low-Level Implementation**: Built on low-level libraries for maximum control and performance
- 🎨 **Colorized Output**: Syntax-highlighted JSON responses
- 📊 **Verbose Mode**: View detailed request/response headers
- 🔧 **Flexible Configuration**: Multiple output formats, custom headers, timeouts
- 🌐 **ALPN Protocol Negotiation**: Automatically negotiates HTTP/2 or HTTP/1.1 with the server

## Installation

### From Source

```bash
git clone <repository-url>
cd apigrok
cargo build --release
cargo install --path .
```

The binary will be available as `apigrok`.

## Usage

```bash
apigrok [OPTIONS] <URL> [METHOD]
```

### Arguments

- `<URL>` - Target URL (required)
- `[METHOD]` - HTTP method: get, post, put, delete, patch, head, options (default: get)

### Options

- `-H, --header <KEY:VALUE>` - Add custom headers (can be specified multiple times)
- `-d, --data <DATA>` - Request body data
- `-j, --json <JSON>` - JSON request body (sets Content-Type automatically)
- `--http1` - Force HTTP/1.1
- `--http2` - Force HTTP/2
- `--http3` - Force HTTP/3
- `--ws` - Use WebSocket
- `--grpc` - Use gRPC
- `--insecure` - Use insecure connection (no TLS) - for gRPC and HTTP
- `-v, --verbose` - Show request/response headers
- `-L, --location` - Follow redirects
- `-t, --timeout <SECONDS>` - Request timeout (default: 30)
- `-o, --output <FORMAT>` - Output format: auto, json, plain (default: auto)

### Protocol Selection

By default, APIGrok uses **ALPN (Application-Layer Protocol Negotiation)** to automatically negotiate the best protocol with the server:

- **No flag**: ALPN negotiation (usually HTTP/2 for modern servers, HTTP/1.1 for older ones)
- **`--http1`**: Force HTTP/1.1 (no negotiation)
- **`--http2`**: Force HTTP/2 (fails if server doesn't support it)

```bash
# ALPN auto-negotiation (recommended)
apigrok https://api.example.com/users

# Force HTTP/1.1
apigrok https://api.example.com/users --http1

# Force HTTP/2
apigrok https://api.example.com/users --http2
```

## Examples

### Basic GET Request

```bash
apigrok https://api.example.com/users
```

### GET with Custom Headers

```bash
apigrok https://api.example.com/users -H "Authorization: Bearer token123" -v
```

### POST with JSON Data

```bash
apigrok https://api.example.com/users post -j '{"name":"John","email":"john@example.com"}'
```

### PUT with Custom Headers

```bash
apigrok https://api.example.com/users/1 put \
  -j '{"name":"Jane"}' \
  -H "Authorization: Bearer token123" \
  -H "X-Custom-Header: value"
```

### Force HTTP/2

```bash
apigrok https://api.example.com/users --http2
```

### WebSocket Connection

```bash
apigrok wss://echo.websocket.org --ws -d "Hello WebSocket"
```

### Verbose Mode with Timeout

```bash
apigrok https://api.example.com/slow-endpoint -v -t 60
```

### DELETE Request

```bash
apigrok https://api.example.com/users/1 delete -H "Authorization: Bearer token123"
```

## Output

APIGrok provides colorized, formatted output with:

- **Status Line**: Protocol version, status code, status text, and response time
- **Headers**: (shown with `-v` flag)
- **Body**: Automatically formatted JSON (when detected) or plain text

Example output:

```
HTTP/1.1 200 OK (245ms)

{
  "id": 1,
  "name": "John Doe",
  "email": "john@example.com"
}
```

## Protocol Support

### HTTP/1.1 ✅

Fully implemented with TLS support.

```bash
apigrok https://api.example.com/users --http1
```

### HTTP/2 ✅

Fully implemented with TLS and ALPN support using rustls.

```bash
apigrok https://api.example.com/users --http2
```

### HTTP/3 🚧

Coming soon. Requires QUIC protocol implementation.

```bash
apigrok https://api.example.com/users --http3
```

### WebSocket ✅

Fully implemented for basic WebSocket connections.

```bash
apigrok wss://echo.websocket.org --ws -d "Hello"
```

### gRPC ⚠️

Partially implemented. URL parsing, connection (secure/insecure), and basic infrastructure in place.

**Current Status:**
- ✅ URL parsing (`grpc://host:port/Service/Method`)
- ✅ Secure and insecure connections (`--insecure` flag)
- 🚧 Dynamic method invocation (requires reflection API)
- 🚧 JSON to protobuf conversion

**Usage:**
```bash
# Test connection (secure)
apigrok grpc://api.example.com:443/package.Service/Method --grpc

# Test connection (insecure)
apigrok grpc://localhost:50051/helloworld.Greeter/SayHello --grpc --insecure
```

Full dynamic gRPC calls require server reflection API and dynamic message construction. For production use, consider `grpcurl`.

## Architecture

APIGrok is built with a modular architecture:

```
src/
├── main.rs           # Entry point and protocol routing
├── cli.rs            # Command-line argument parsing
├── error.rs          # Error types and handling
├── request.rs        # Request builder
├── response.rs       # Response formatter with colorization
└── protocols/
    ├── mod.rs        # Protocol trait definition
    ├── http1.rs      # HTTP/1.1 implementation
    ├── http2.rs      # HTTP/2 implementation
    ├── http3.rs      # HTTP/3 placeholder
    ├── websocket.rs  # WebSocket implementation
    └── grpc.rs       # gRPC placeholder
```

## Dependencies

- `clap` - CLI parsing
- `tokio` - Async runtime
- `hyper` - HTTP/1.1 & HTTP/2
- `hyper-tls` - TLS support
- `tokio-tungstenite` - WebSocket
- `tonic` - gRPC client
- `colored` - Terminal colors
- `serde_json` - JSON parsing

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running in Development

```bash
cargo run -- https://httpbin.org/get
```

## Contributing

Contributions are welcome! Areas for improvement:

- Complete HTTP/3 implementation
- Complete gRPC implementation with reflection API
- Add request/response history
- Add authentication helpers (OAuth, JWT, Basic Auth)
- Add support for request collections
- Add interactive TUI mode
- Add code generation from responses
- Add OpenAPI/Swagger integration

## License

[Add your license here]

## Acknowledgments

Built with Rust and inspired by tools like `curl`, `httpie`, and `grpcurl`.
