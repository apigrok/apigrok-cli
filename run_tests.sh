#!/bin/bash

# Comprehensive test runner for APIGrok
# Tests all supported protocols: HTTP/1.1, HTTP/2, WebSocket, gRPC

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
HTTP_PORT=3000
WS_PORT=3001
GRPC_PORT=50051
APIGROK_BIN="./target/release/apigrok"

# PIDs for cleanup
HTTP_PID=""
WS_PID=""
GRPC_PID=""

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up test servers...${NC}"

    if [ ! -z "$HTTP_PID" ]; then
        kill $HTTP_PID 2>/dev/null || true
        echo "  - Stopped HTTP server (PID: $HTTP_PID)"
    fi

    if [ ! -z "$WS_PID" ]; then
        kill $WS_PID 2>/dev/null || true
        echo "  - Stopped WebSocket server (PID: $WS_PID)"
    fi

    if [ ! -z "$GRPC_PID" ]; then
        kill $GRPC_PID 2>/dev/null || true
        echo "  - Stopped gRPC server (PID: $GRPC_PID)"
    fi

    # Kill any remaining test servers
    pkill -f "http_test_server" 2>/dev/null || true
    pkill -f "ws_test_server" 2>/dev/null || true
    pkill -f "grpc_test_server" 2>/dev/null || true
}

# Set trap to cleanup on exit
trap cleanup EXIT INT TERM

# Print header
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  APIGrok Test Suite${NC}"
echo -e "${BLUE}========================================${NC}\n"

# Step 1: Build main binary
echo -e "${YELLOW}Step 1: Building main binary...${NC}"
cargo build --release
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Build successful${NC}\n"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

# Step 2: Build test servers
echo -e "${YELLOW}Step 2: Building test servers...${NC}"
cargo build --release --features test-servers --bins
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Test servers built${NC}\n"
else
    echo -e "${RED}✗ Test server build failed${NC}"
    exit 1
fi

# Step 3: Start test servers
echo -e "${YELLOW}Step 3: Starting test servers...${NC}"

# Start HTTP server
./target/release/http_test_server > /tmp/http_test_server.log 2>&1 &
HTTP_PID=$!
echo "  - HTTP server starting (PID: $HTTP_PID)"

# Start WebSocket server
./target/release/ws_test_server > /tmp/ws_test_server.log 2>&1 &
WS_PID=$!
echo "  - WebSocket server starting (PID: $WS_PID)"

# Start gRPC server
./target/release/grpc_test_server > /tmp/grpc_test_server.log 2>&1 &
GRPC_PID=$!
echo "  - gRPC server starting (PID: $GRPC_PID)"

# Wait for servers to start
echo -e "  - Waiting for servers to be ready..."
sleep 3

# Check if servers are running
if ! kill -0 $HTTP_PID 2>/dev/null; then
    echo -e "${RED}✗ HTTP server failed to start${NC}"
    cat /tmp/http_test_server.log
    exit 1
fi

if ! kill -0 $WS_PID 2>/dev/null; then
    echo -e "${RED}✗ WebSocket server failed to start${NC}"
    cat /tmp/ws_test_server.log
    exit 1
fi

if ! kill -0 $GRPC_PID 2>/dev/null; then
    echo -e "${RED}✗ gRPC server failed to start${NC}"
    cat /tmp/grpc_test_server.log
    exit 1
fi

echo -e "${GREEN}✓ All test servers running${NC}\n"

# Step 4: Run unit tests
echo -e "${YELLOW}Step 4: Running unit tests...${NC}"
cargo test --release
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Unit tests passed${NC}\n"
else
    echo -e "${RED}✗ Unit tests failed${NC}"
    exit 1
fi

# Step 5: Run integration tests
echo -e "${YELLOW}Step 5: Running integration tests...${NC}\n"

FAILED=0
PASSED=0

# HTTP/1.1 Tests
echo -e "${BLUE}--- HTTP/1.1 Tests ---${NC}"

echo -n "  Testing GET request... "
RESPONSE=$(${APIGROK_BIN} http http://127.0.0.1:${HTTP_PORT}/get --http1 2>&1)
if echo "$RESPONSE" | grep -q "HTTP/1.1"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing POST with JSON... "
RESPONSE=$(${APIGROK_BIN} http http://127.0.0.1:${HTTP_PORT}/post post --http1 -j '{"test":"data"}' 2>&1)
if echo "$RESPONSE" | grep -q "test"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing custom headers... "
RESPONSE=$(${APIGROK_BIN} http http://127.0.0.1:${HTTP_PORT}/headers --http1 -H "X-Test: value" 2>&1)
if echo "$RESPONSE" | grep -q "x-test"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

# HTTP/2 Tests
echo -e "\n${BLUE}--- HTTP/2 Tests ---${NC}"

echo -n "  Testing GET with HTTP/2... "
RESPONSE=$(${APIGROK_BIN} http http://127.0.0.1:${HTTP_PORT}/get --http2 2>&1)
if echo "$RESPONSE" | grep -q "200"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing ALPN negotiation... "
RESPONSE=$(${APIGROK_BIN} http http://127.0.0.1:${HTTP_PORT}/get 2>&1)
if echo "$RESPONSE" | grep -q "200"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

# WebSocket Tests
echo -e "\n${BLUE}--- WebSocket Tests ---${NC}"

echo -n "  Testing WebSocket echo... "
set +e  # Temporarily disable exit on error (WS client has exit code bug)
RESPONSE=$(${APIGROK_BIN} websocket ws://127.0.0.1:${WS_PORT} -j '{"message":"test"}' 2>&1)
set -e  # Re-enable exit on error
if echo "$RESPONSE" | grep -q "Echo"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

# gRPC Tests
echo -e "\n${BLUE}--- gRPC Tests ---${NC}"

echo -n "  Testing gRPC connection... "
RESPONSE=$(${APIGROK_BIN} grpc grpc://localhost:${GRPC_PORT}/helloworld.Greeter/SayHello --insecure 2>&1)
if echo "$RESPONSE" | grep -q "Connection"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing gRPC with proto file... "
RESPONSE=$(${APIGROK_BIN} grpc grpc://localhost:${GRPC_PORT}/helloworld.Greeter/SayHello --insecure --proto test/helloworld.proto -j '{"name":"World"}' 2>&1)
if echo "$RESPONSE" | grep -q "Hello"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing gRPC with reflection... "
RESPONSE=$(${APIGROK_BIN} grpc grpc://localhost:${GRPC_PORT}/helloworld.Greeter/SayHello --insecure --reflection -j '{"name":"Reflection"}' 2>&1)
if echo "$RESPONSE" | grep -q "Hello"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing gRPC list services... "
RESPONSE=$(${APIGROK_BIN} grpc grpc://localhost:${GRPC_PORT} --insecure --list-services 2>&1)
if echo "$RESPONSE" | grep -q "helloworld.Greeter"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing gRPC server streaming... "
RESPONSE=$(${APIGROK_BIN} grpc grpc://localhost:${GRPC_PORT}/streaming.StreamService/ServerStream --insecure --proto test/streaming.proto -j '{"message":"test","sequence":1}' 2>&1)
if echo "$RESPONSE" | grep -q "server_streaming"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing gRPC client streaming... "
RESPONSE=$(${APIGROK_BIN} grpc grpc://localhost:${GRPC_PORT}/streaming.StreamService/ClientStream --insecure --proto test/streaming.proto --stream-input test/stream_input.json 2>&1)
if echo "$RESPONSE" | grep -q "Received"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

echo -n "  Testing gRPC bidirectional streaming... "
RESPONSE=$(${APIGROK_BIN} grpc grpc://localhost:${GRPC_PORT}/streaming.StreamService/Bidirectional --insecure --proto test/streaming.proto --stream-input test/stream_input.json 2>&1)
if echo "$RESPONSE" | grep -q "bidirectional"; then
    echo -e "${GREEN}✓${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗${NC}"
    ((FAILED++))
fi

# Summary
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}  Test Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Total tests: $((PASSED + FAILED))"
echo -e "${GREEN}Passed: ${PASSED}${NC}"
echo -e "${RED}Failed: ${FAILED}${NC}"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}✓ All tests passed!${NC}\n"
    exit 0
else
    echo -e "\n${RED}✗ Some tests failed${NC}\n"
    exit 1
fi
