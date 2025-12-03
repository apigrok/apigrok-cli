# Implementation Notes

## Current Status

### ✅ Fully Implemented
- **HTTP/1.1**: Complete implementation with TLS support
- **HTTP/2**: Complete implementation with TLS and ALPN support (using rustls)
- **ALPN Auto-Negotiation**: AutoClient that negotiates HTTP/2 or HTTP/1.1 with the server
- **Protocol Override**: Explicit --http1 and --http2 flags to force specific protocols
- **CLI Parsing**: Full argument parsing with clap
- **Response Formatting**: Colorized JSON output
- **Request Building**: Headers, body, method support
- **Error Handling**: Comprehensive error types

### ⚠️ Partially Implemented
- **WebSocket**: Basic connection support (send/receive single message)

### ✅ Fully Implemented (gRPC Complete)
- **gRPC**: Complete dynamic gRPC support with all features
  - URL parsing: grpc://host:port/Service/Method ✅
  - Connection (TLS and non-TLS): ✅
  - --insecure flag support: ✅
  - --proto flag for proto files: ✅
  - --reflection flag for server reflection: ✅
  - --list-services for service discovery: ✅
  - --stream-input for client/bidirectional streaming: ✅
  - Proto file parsing (protox, no protoc needed): ✅
  - Server reflection (bidirectional streaming): ✅
  - FileDescriptorSet building: ✅
  - DescriptorPool for type lookups: ✅
  - Custom DynamicCodec for prost-reflect: ✅
  - JSON to protobuf conversion (all types): ✅
  - Protobuf to JSON conversion (all types): ✅
  - **Unary method invocation**: ✅
  - **Server streaming**: ✅
  - **Client streaming**: ✅
  - **Bidirectional streaming**: ✅
  - Stream input from file (JSON per line): ✅
  - Service/method discovery: ✅
  - Helpful error messages: ✅

### 🚧 Not Yet Implemented
- **HTTP/3**: Placeholder only (requires QUIC/quinn integration)

## Known Limitations

1. **WebSocket**: Current implementation only sends one message and receives one response. A full implementation would need:
   - Interactive mode
   - Multiple message support
   - Ping/pong handling
   - Proper connection lifecycle management

2. **HTTP/3**: Requires:
   - QUIC protocol setup with quinn
   - Certificate handling
   - H3 request/response handling

## Architecture Decisions

- **Trait-based protocols**: Each protocol implements the `Protocol` trait for clean separation
- **Async/await**: Using Tokio runtime for all async operations
- **Low-level libraries**: Using hyper directly instead of higher-level HTTP clients
- **Modular design**: Easy to add new protocols or extend existing ones

## Future Improvements

1. Implement full HTTP/3 support
2. Add interactive WebSocket mode
3. Add request/response history
4. Add authentication helpers (OAuth, JWT, etc.)
5. Add TUI mode for interactive exploration
6. Add request collections/workspaces
7. Add OpenAPI/Swagger integration
8. gRPC enhancements:
   - Proto import paths configuration
   - Interactive bidirectional streaming mode
   - Custom metadata/headers per-call
   - Connection pooling and persistence
