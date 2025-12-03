#!/bin/bash
# APIGrok Usage Examples

echo "=== Basic GET Request ==="
apigrok https://httpbin.org/get

echo -e "\n=== GET with Query Parameters ==="
apigrok "https://httpbin.org/get?foo=bar&baz=qux"

echo -e "\n=== POST with JSON ==="
apigrok https://httpbin.org/post post -j '{"name":"John","age":30}'

echo -e "\n=== POST with Form Data ==="
apigrok https://httpbin.org/post post -d "field1=value1&field2=value2"

echo -e "\n=== PUT Request ==="
apigrok https://httpbin.org/put put -j '{"updated":"data"}'

echo -e "\n=== DELETE Request ==="
apigrok https://httpbin.org/delete delete

echo -e "\n=== Custom Headers ==="
apigrok https://httpbin.org/headers -H "X-Custom-Header: MyValue" -H "Authorization: Bearer token123"

echo -e "\n=== Verbose Output ==="
apigrok https://httpbin.org/get -v

echo -e "\n=== Force HTTP/1.1 ==="
apigrok https://httpbin.org/get --http1

echo -e "\n=== Force HTTP/2 ==="
apigrok https://httpbin.org/get --http2

echo -e "\n=== User Agent ==="
apigrok https://httpbin.org/user-agent -H "User-Agent: MyCustomClient/1.0"

echo -e "\n=== Response Status Codes ==="
apigrok https://httpbin.org/status/200
apigrok https://httpbin.org/status/404
apigrok https://httpbin.org/status/500

echo -e "\n=== Authentication ==="
apigrok https://httpbin.org/bearer -H "Authorization: Bearer mytoken123"

echo -e "\n=== Timeout Example (5 seconds) ==="
apigrok https://httpbin.org/delay/3 -t 5

echo -e "\n=== WebSocket Echo ==="
apigrok wss://echo.websocket.org --ws -d "Hello from APIGrok"
