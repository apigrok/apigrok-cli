# Testing Guide for APIGrok

This document describes the test infrastructure for APIGrok and how to run the comprehensive test suite.

## Overview

APIGrok includes a complete test suite that validates all supported protocols:
- HTTP/1.1
- HTTP/2
- WebSocket
- gRPC (with all streaming modes and reflection)

The test suite consists of:
- **Unit tests**: Basic functional tests for parsing and validation
- **Integration tests**: End-to-end tests with real test servers
- **Test servers**: Dedicated servers for each protocol
- **Test runner**: Automated script that orchestrates all tests

## Running Tests

### Quick Start

Run the complete test suite with a single command:

```bash
./run_tests.sh
```

This will:
1. Build the main apigrok binary
2. Build all test servers
3. Start test servers in the background
4. Run unit tests
5. Run integration tests for all protocols
6. Clean up and display results

### Manual Testing

You can also run components individually:

#### Build only
```bash
cargo build --release
cargo build --release --features test-servers --bins
```

#### Run unit tests only
```bash
cargo test --release
```

#### Start test servers manually
```bash
# HTTP server (port 3000)
./target/release/http_test_server

# WebSocket server (port 3001)
./target/release/ws_test_server

# gRPC server (port 50051)
./target/release/grpc_test_server
```

#### Run individual integration tests
```bash
# HTTP/1.1 GET request
./target/release/apigrok http://127.0.0.1:3000/get --http1

# WebSocket echo
./target/release/apigrok ws://127.0.0.1:3001 --ws -j '{"message":"test"}'

# gRPC with reflection
./target/release/apigrok grpc://localhost:50051/helloworld.Greeter/SayHello \
  --grpc --insecure --reflection -j '{"name":"World"}'
```

## Test Coverage

### HTTP/1.1 Tests (3 tests)
- GET request
- POST with JSON body
- Custom headers

### HTTP/2 Tests (2 tests)
- GET with HTTP/2 flag
- ALPN negotiation

### WebSocket Tests (1 test)
- Echo test with JSON message

### gRPC Tests (7 tests)
- Connection test
- Proto file loading
- Server reflection
- List services
- Server streaming
- Client streaming
- Bidirectional streaming

**Total: 13 integration tests**

## Test Architecture

### Test Servers

Test servers are built as separate binaries with the `test-servers` feature:

```toml
[[bin]]
name = "http_test_server"
path = "tests/servers/http_server.rs"
required-features = ["test-servers"]
```

**HTTP Server** (`tests/servers/http_server.rs`):
- Supports HTTP/1.1 and HTTP/2 via ALPN
- Endpoints: /get, /post, /headers, /status/:code, /health
- Port: 3000

**WebSocket Server** (`tests/servers/ws_server.rs`):
- Echoes text messages with "Echo: " prefix
- Handles ping/pong
- Port: 3001

**gRPC Server** (`tests/servers/grpc_server.rs`):
- Implements helloworld.Greeter service
- Implements streaming.StreamService (all streaming modes)
- Supports server reflection
- Port: 50051

### Unit Tests

Unit tests are located in `tests/test_grpc_parsing.rs` and validate:
- gRPC URL parsing
- URL component extraction
- Invalid URL handling

### Integration Test Script

The test runner (`run_tests.sh`) provides:
- Automated build process
- Server lifecycle management
- Color-coded test output
- Cleanup on exit (via trap)
- Exit codes (0 = success, 1 = failure)

## Adding New Tests

### Adding a Unit Test

Edit `tests/test_grpc_parsing.rs` or create a new test file:

```rust
#[test]
fn test_my_feature() {
    // Test code here
    assert_eq!(result, expected);
}
```

### Adding an Integration Test

Edit `run_tests.sh` and add your test in the appropriate section:

```bash
echo -n "  Testing my feature... "
RESPONSE=$(${APIGROK_BIN} <your-command> 2>&1)
if echo "$RESPONSE" | grep -q "Expected Pattern"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi
```

### Adding a Test Server Endpoint

Edit the relevant server file (e.g., `tests/servers/http_server.rs`):

```rust
(&Method::GET, "/my-endpoint") => {
    let body = serde_json::json!({
        "message": "response"
    });
    *response.body_mut() = Full::new(Bytes::from(
        serde_json::to_string_pretty(&body).unwrap()
    ));
}
```

## Test Proto Files

Proto files for gRPC testing are located in the `test/` directory:

- `test/helloworld.proto`: Simple unary RPC
- `test/streaming.proto`: All streaming modes
- `test/stream_input.json`: Sample input for client streaming

## Continuous Integration

The test suite is designed to run in CI/CD environments:

- Exit code 0 on success, 1 on failure
- Self-contained (builds everything needed)
- Automatic cleanup (even on interruption)
- Color output (can be disabled for CI logs)

## Troubleshooting

### Port Already in Use

If tests fail to start servers:
```bash
# Kill any running test servers
pkill -f test_server
```

### Build Failures

Ensure protobuf compiler is installed:
```bash
# macOS
brew install protobuf

# Ubuntu/Debian
apt-get install protobuf-compiler
```

### Test Timeouts

Individual tests have a default timeout. Increase if needed by modifying the test script.

## Test Maintenance

When adding new features:
1. Add corresponding test server endpoints if needed
2. Add integration tests to `run_tests.sh`
3. Update this documentation
4. Verify all tests pass: `./run_tests.sh`

## Performance

The complete test suite typically runs in:
- Build time: ~10-20 seconds (cold)
- Test execution: ~5-10 seconds
- Total: ~15-30 seconds

Individual protocol tests execute in milliseconds.
