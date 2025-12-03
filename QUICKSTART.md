# APIGrok Quick Start

## Build

```bash
cargo build --release
```

## Basic Usage

### Simple GET (Auto ALPN Negotiation)
```bash
./target/release/apigrok https://httpbin.org/get
# Automatically negotiates HTTP/2 or HTTP/1.1 via ALPN
```

### POST with JSON
```bash
./target/release/apigrok https://httpbin.org/post post -j '{"name":"test"}'
```

### With Headers
```bash
./target/release/apigrok https://httpbin.org/headers -H "Authorization: Bearer token"
```

### Verbose Mode
```bash
./target/release/apigrok https://httpbin.org/get -v
```

## Install Globally

```bash
cargo install --path .
```

Then use as:
```bash
apigrok https://api.example.com/endpoint
```

## Supported Protocols

- ✅ **ALPN Auto-Negotiation** - Default behavior (no flag needed)
  - Automatically negotiates HTTP/2 or HTTP/1.1 with the server
- ✅ **HTTP/1.1** - `--http1` (force HTTP/1.1)
- ✅ **HTTP/2** - `--http2` (force HTTP/2)
- 🚧 **HTTP/3** - `--http3` (not yet implemented)
- ✅ **WebSocket** - `--ws` (basic support)
- ✅ **gRPC** - `--grpc` (complete support: all streaming modes + reflection)
  - Use `--insecure` for non-TLS gRPC connections
  - Use `--proto <file>` for proto file-based calls
  - Use `--reflection` for server reflection (no proto files needed)
  - Use `--list-services` to discover available services
  - Use `--stream-input <file>` for client/bidirectional streaming

## gRPC Examples

### Connection Test
```bash
./target/release/apigrok grpc://localhost:50051/helloworld.Greeter/SayHello --grpc --insecure
```

### Unary Call with Proto File
```bash
./target/release/apigrok grpc://localhost:50051/helloworld.Greeter/SayHello \
  --grpc --insecure \
  --proto test/helloworld.proto \
  -j '{"name":"World"}'
```

### Using Server Reflection
```bash
./target/release/apigrok grpc://localhost:50051/helloworld.Greeter/SayHello \
  --grpc --insecure \
  --reflection \
  -j '{"name":"World"}'
```

### List Services
```bash
./target/release/apigrok grpc://localhost:50051 \
  --grpc --insecure \
  --list-services
```

### Server Streaming
```bash
./target/release/apigrok grpc://localhost:50051/streaming.StreamService/ServerStream \
  --grpc --insecure \
  --proto test/streaming.proto \
  -j '{"message":"Start"}'
```

### Client Streaming
```bash
./target/release/apigrok grpc://localhost:50051/streaming.StreamService/ClientStream \
  --grpc --insecure \
  --proto test/streaming.proto \
  --stream-input test/stream_input.json
```

## Examples

See `examples/usage.sh` for comprehensive examples.
