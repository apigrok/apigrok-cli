// Simple gRPC test server for testing APIGrok
// To run:
//   1. cargo install --bin tonic_build
//   2. protoc --rust_out=. helloworld.proto
//   3. rustc test_server.rs && ./test_server

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "[::1]:50051".parse()?;

    println!("gRPC test server listening on {}", addr);
    println!("Test with: apigrok grpc://localhost:50051/helloworld.Greeter/SayHello --grpc --insecure --proto test/helloworld.proto -j '{{\"name\":\"World\"}}'");

    // Note: This is a placeholder. A full implementation would require:
    // - tonic server setup
    // - Service implementation
    // - Proto compilation with tonic-build

    println!("\nFor a full test server, use grpcurl or create a proper tonic server");
    println!("Example grpcurl test server:");
    println!("  docker run -p 50051:50051 fullstorydev/grpcurl -plaintext -import-path /proto -proto helloworld.proto localhost:50051 describe");

    Ok(())
}
