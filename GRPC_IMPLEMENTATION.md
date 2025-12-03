# gRPC Implementation ✅

## Status: Fully Implemented

Successfully implemented **complete dynamic gRPC support** with:
- ✅ Proto file parsing
- ✅ Server reflection
- ✅ JSON to protobuf conversion
- ✅ All streaming modes (unary, server, client, bidirectional)
- ✅ Service discovery

## What Was Done

### 1. Dependencies Added
- `tonic` 0.12 - gRPC client framework
- `tonic-reflection` 0.12 - gRPC reflection support
- `prost` 0.13 - Protocol Buffers
- `prost-types` 0.13 - Well-known protobuf types
- `prost-reflect` 0.14 - Dynamic message support
- `protox` 0.7 - Proto file parsing without protoc

### 2. CLI Flags Added
- `--insecure` flag for non-TLS connections
- `--proto` flag for specifying .proto files (mutually exclusive with --reflection)
- `--reflection` flag for using server reflection instead of proto files
- `--stream-input <file>` flag for client streaming (JSON per line)
- `--list-services` flag to list all available services via reflection
- Works with gRPC and HTTP protocols

### 3. URL Parsing
- Format: `grpc://host:port/Service/Method`
- Examples:
  - `grpc://localhost:50051/helloworld.Greeter/SayHello`
  - `grpc://api.example.com:443/package.Service/Method`

### 4. Connection Management
- Secure connections (HTTPS/TLS) by default
- Insecure connections (HTTP) with `--insecure` flag
- Proper error handling and user feedback

### 5. Proto File Support
- Parse .proto files using `protox`
- Build FileDescriptorSet at runtime
- No need for protoc binary

### 6. Dynamic Message Construction
- Custom DynamicCodec for tonic
- JSON to protobuf conversion with full type support
- Protobuf to JSON conversion for responses
- Support for all protobuf types (scalars, messages, enums, repeated, maps)

### 7. Streaming Support
- **Unary**: Single request → single response (standard RPC)
- **Server streaming**: Single request → multiple responses
- **Client streaming**: Multiple requests → single response (via --stream-input file)
- **Bidirectional streaming**: Concurrent send/receive streams
- Automatic detection of streaming type from proto/reflection
- Stream input from file (one JSON per line)

### 8. Server Reflection
- Query server for FileDescriptorSet without proto files
- Bidirectional streaming communication with reflection API
- FileDescriptorProto parsing and descriptor pool building
- List all available services with --list-services

### 9. Method Invocation
- Dynamic method calls for all streaming types
- Automatic service/method discovery from proto or reflection
- Helpful error messages with available options
- JSON array output for streaming responses

## Current Capabilities

✅ **Fully Implemented:**
- URL parsing and validation
- Endpoint extraction (host:port)
- Service and method extraction
- Secure connection setup (TLS)
- Insecure connection setup (no TLS)
- Proto file parsing (--proto flag)
- Server reflection (--reflection flag)
- Service listing (--list-services)
- FileDescriptorSet building
- DescriptorPool creation
- JSON to protobuf conversion
- Protobuf to JSON conversion
- Dynamic message construction
- **Unary method invocation**
- **Server streaming** (single request → stream of responses)
- **Client streaming** (stream of requests → single response)
- **Bidirectional streaming** (concurrent request/response streams)
- Stream input from file (--stream-input)
- Service/method discovery from proto files or reflection
- Clear error messages with suggestions

🚧 **Future Enhancements:**
- Proto import paths configuration
- Multiple proto file support
- Custom metadata/headers per-call
- Compression support

## Usage Examples

### Test Connection Only (No Proto File)
```bash
# Tests connection and shows available features
apigrok grpc://localhost:50051/helloworld.Greeter/SayHello --grpc --insecure
```

### Full Dynamic gRPC Call (With Proto File)
```bash
# Make actual gRPC call with JSON request
apigrok grpc://localhost:50051/helloworld.Greeter/SayHello \
  --grpc --insecure \
  --proto test/helloworld.proto \
  -j '{"name":"World"}'
```

### Secure (TLS) Connection
```bash
apigrok grpc://api.example.com:443/package.Service/Method \
  --grpc \
  --proto service.proto \
  -j '{"request":"data"}'
```

### Complex Nested Messages
```bash
apigrok grpc://localhost:50051/user.UserService/CreateUser \
  --grpc --insecure \
  --proto user.proto \
  -j '{
    "user": {
      "name": "John Doe",
      "email": "john@example.com",
      "age": 30,
      "tags": ["developer", "rust"]
    }
  }'
```

### Server Streaming
```bash
# Server sends multiple responses for a single request
apigrok grpc://localhost:50051/streaming.StreamService/ServerStream \
  --grpc --insecure \
  --proto test/streaming.proto \
  -j '{"message": "Start streaming", "sequence": 1}'
```

### Client Streaming
```bash
# Client sends multiple requests from file, gets single response
apigrok grpc://localhost:50051/streaming.StreamService/ClientStream \
  --grpc --insecure \
  --proto test/streaming.proto \
  --stream-input test/stream_input.json
```

### Bidirectional Streaming
```bash
# Both client and server stream messages
apigrok grpc://localhost:50051/streaming.StreamService/Bidirectional \
  --grpc --insecure \
  --proto test/streaming.proto \
  --stream-input test/stream_input.json
```

### Using Server Reflection (No Proto File)
```bash
# Query server for service definition and make call
apigrok grpc://localhost:50051/helloworld.Greeter/SayHello \
  --grpc --insecure \
  --reflection \
  -j '{"name":"World"}'
```

### List Available Services
```bash
# Discover all services on the server
apigrok grpc://localhost:50051 \
  --grpc --insecure \
  --list-services
```

## Example Output

### Connection Test (No Proto File)
```
gRPC 200 Connection OK (15ms)

✓ gRPC Connection Established

Endpoint: localhost:50051
Service: helloworld.Greeter
Method: SayHello
Mode: insecure (http)

STATUS: Connection Test Complete ✓

To make dynamic gRPC calls, use --proto flag:

$ apigrok grpc://localhost:50051/helloworld.Greeter/SayHello \
  --grpc --insecure \
  --proto helloworld.proto \
  -j '{"name":"World"}'

✅ IMPLEMENTED:
- URL parsing (grpc://host:port/Service/Method)
- Connection management (secure/insecure)
- TLS and non-TLS support
- Proto file parsing with --proto flag
- Custom DynamicCodec for prost-reflect messages
- JSON to protobuf conversion
- Dynamic method invocation
- Full unary gRPC support

ALTERNATIVE TOOLS:
- grpcurl: Full reflection + proto file support
- Postman: GUI with gRPC support
- BloomRPC: Desktop GUI client
```

### Successful gRPC Call (With Proto File)
```
gRPC 200 OK (23ms)

✓ gRPC Call Successful

Endpoint: localhost:50051
Service: helloworld.Greeter
Method: SayHello
Mode: insecure (http)
Time: 23ms

Response:
{
  "message": "Hello World"
}
```

### Error: Service Not Found in Proto
```
Error: Protocol error: Service 'helloworld.Greet' not found in proto file. Available services: helloworld.Greeter
```

### Error: Method Not Found
```
Error: Protocol error: Method 'SayHi' not found in service 'helloworld.Greeter'. Available methods: SayHello
```

### Server Streaming Output
```
gRPC 200 OK (145ms)

✓ gRPC Call Successful

Endpoint: localhost:50051
Service: streaming.StreamService
Method: ServerStream
Mode: insecure (http)
Streaming: server_streaming
Messages: 5
Time: 145ms

Response:
[
  {
    "reply": "Response 1",
    "sequence": 1,
    "timestamp": 1234567890
  },
  {
    "reply": "Response 2",
    "sequence": 2,
    "timestamp": 1234567891
  },
  ...
]
```

### List Services Output
```
gRPC 200 OK (23ms)

✓ Services Available via Reflection

Endpoint: localhost:50051
Mode: insecure (http)
Time: 23ms

Services (3):
  - helloworld.Greeter
  - streaming.StreamService
  - grpc.reflection.v1alpha.ServerReflection
```

### Using Reflection Output
```
gRPC 200 OK (67ms)

✓ gRPC Call Successful

Endpoint: localhost:50051
Service: helloworld.Greeter
Method: SayHello
Mode: insecure (http)
Streaming: unary
Time: 67ms

Response:
{
  "message": "Hello World"
}
```

## Technical Implementation

### Files Created/Modified

**src/protocols/grpc.rs:**
- `GrpcClient` struct with full dynamic support
- `DynamicCodec`, `DynamicEncoder`, `DynamicDecoder` - Custom codec for prost-reflect
- `parse_grpc_url()` - Parses gRPC URLs
- `create_channel()` - Creates secure/insecure channels
- `parse_proto_file()` - Parses .proto files using protox
- `json_to_dynamic_message()` - Converts JSON to DynamicMessage
- `populate_message_from_json()` - Recursive message population
- `json_value_to_prost_value()` - Type conversion
- `prost_value_to_json()` - Protobuf to JSON conversion
- `dynamic_message_to_json()` - DynamicMessage to JSON
- `invoke_method()` - Dynamic unary method invocation
- `execute()` - Main execution with connection test mode
- `execute_dynamic()` - Full dynamic gRPC call execution

**src/cli.rs:**
- Added `--insecure` boolean flag
- Added `--proto` optional string flag

**src/request.rs:**
- Added `insecure` field
- Added `proto` field
- Updated `normalize_url()` to handle `grpc://`

**Cargo.toml:**
- Added gRPC dependencies: tonic, prost, prost-reflect, protox

**test/helloworld.proto:**
- Example proto file for testing

## Implementation Achievements

### ✅ Solved Challenges

#### 1. Proto Descriptor Parsing
- Used `protox` for runtime proto file compilation
- No protoc binary dependency required
- FileDescriptorSet built at runtime
- DescriptorPool for type lookups

#### 2. Dynamic Message Construction
- Full JSON to protobuf conversion
- Support for all protobuf scalar types
- Nested messages, repeated fields, maps
- Enum handling (by name and number)
- Base64 encoding for bytes fields

#### 3. Custom Tonic Codec
- Implemented Codec trait for DynamicMessage
- DynamicEncoder for serialization
- DynamicDecoder for deserialization
- Compatible with tonic's unary client

#### 4. Service/Method Discovery
- Parse proto files to find services
- List available methods
- Helpful error messages with suggestions
- Type-safe method descriptors

## Comparison with grpcurl

**grpcurl** (Go-based tool):
- ✅ Server reflection
- ✅ Dynamic invocation
- ✅ Proto file support
- ✅ Streaming
- ❌ Requires separate installation
- ❌ gRPC-only

**APIGrok:**
- ✅ Server reflection (--reflection flag)
- ✅ Dynamic invocation (all streaming types)
- ✅ Proto file support (--proto flag)
- ✅ Streaming (unary, server, client, bidirectional)
- ✅ Service discovery (--list-services)
- ✅ Integrated multi-protocol tool (HTTP/1.1, HTTP/2, WebSocket, gRPC)
- ✅ No protoc dependency
- ✅ Simple JSON interface
- ✅ Stream input from files

## Future Enhancements

### Potential Features

1. **Enhanced Proto Support**
   - Multiple proto file imports
   - Custom import paths (--proto-path flag)
   - Well-known types optimization
   - Proto file caching for reflection results

2. **Advanced Features**
   - Custom metadata/headers per-call
   - Deadline/timeout per-call (beyond global timeout)
   - Compression support (gzip, etc.)
   - Keep-alive configuration
   - Retry policies

3. **Interactive Mode**
   - Real-time bidirectional streaming with manual input
   - Interactive shell for gRPC exploration
   - Tab completion for services/methods

4. **Performance**
   - Connection pooling
   - Persistent connections across calls
   - Multiplexing support

## Summary

APIGrok now provides **complete dynamic gRPC support** with:
- ✅ Proto file parsing (no protoc needed)
- ✅ Server reflection (no proto files needed)
- ✅ JSON to protobuf conversion
- ✅ Dynamic method invocation (all streaming types)
- ✅ Service discovery
- ✅ Secure and insecure connections
- ✅ Stream input from files
- ✅ Helpful error messages

**APIGrok is now feature-complete for gRPC**, matching or exceeding the capabilities of specialized tools like grpcurl, while also supporting HTTP/1.1, HTTP/2, HTTP/3, and WebSocket protocols in a single unified CLI tool.

This implementation successfully provides a comprehensive multi-protocol API testing tool with first-class gRPC support.
