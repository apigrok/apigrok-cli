# ALPN Auto-Negotiation Implementation ✅

## What Was Done

Successfully implemented ALPN (Application-Layer Protocol Negotiation) for automatic protocol selection between HTTP/2 and HTTP/1.1.

## Changes Made

### 1. New Files Created

**src/protocols/auto.rs:**
- New `AutoClient` that uses ALPN for protocol negotiation
- Enables both HTTP/1.1 and HTTP/2 via `enable_all_versions()`
- Lets server decide which protocol to use
- No retry logic needed - single connection with ALPN

### 2. Files Modified

**src/protocols/mod.rs:**
- Added `pub mod auto;` to export AutoClient

**src/main.rs:**
- Imported `AutoClient`
- Updated Auto protocol case to use `AutoClient::new()`
- Kept `Http1Client` and `Http2Client` for explicit overrides

**Documentation Updates:**
- README.md: Added Protocol Selection section explaining ALPN behavior
- IMPLEMENTATION_NOTES.md: Added ALPN auto-negotiation to fully implemented features
- QUICKSTART.md: Updated with ALPN information

## How It Works

### ALPN Negotiation
1. Client advertises support for both `h2` (HTTP/2) and `http/1.1`
2. Server chooses the best protocol it supports
3. Connection established with negotiated protocol
4. Single TLS handshake - no retries needed

### User Control
```bash
# Auto ALPN (negotiates HTTP/2 or HTTP/1.1)
apigrok https://example.com/api

# Force HTTP/1.1 (no negotiation)
apigrok https://example.com/api --http1

# Force HTTP/2 (fails if server doesn't support it)
apigrok https://example.com/api --http2
```

## Testing Results

### Default Behavior (ALPN)
- ✅ Google: Negotiated HTTP/2.0
- ✅ HTTPBin: Negotiated HTTP/2.0
- Modern servers automatically select HTTP/2

### Explicit Overrides
- ✅ `--http1` forces HTTP/1.1
- ✅ `--http2` forces HTTP/2.0
- ✅ Overrides work as expected

## Benefits

1. **Better Performance by Default**: HTTP/2 when available
2. **Backward Compatible**: Falls back to HTTP/1.1 for older servers
3. **User Control**: Can force specific protocol when needed
4. **Standard Compliant**: Uses industry-standard ALPN
5. **Single Connection**: No retry overhead

## Technical Details

**ALPN Process:**
```
Client Hello (TLS)
├── Supported Protocols: [h2, http/1.1]
└── Server chooses: h2

Connection Established: HTTP/2.0
```

**rustls Configuration:**
```rust
HttpsConnectorBuilder::new()
    .with_native_roots()
    .https_or_http()
    .enable_all_versions()  // Enables HTTP/1.1 and HTTP/2
    .build()
```

## Next Steps

With ALPN complete, future enhancements could include:
1. HTTP/3 with QUIC (requires different ALPN tokens)
2. Verbose mode showing which protocol was negotiated
3. Statistics on protocol selection
4. gRPC support (uses HTTP/2 under the hood)
