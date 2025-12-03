// gRPC test server with reflection support
// Run with: cargo run --bin grpc_test_server

use tonic::{transport::Server, Request, Response, Status};
use tonic_reflection::server::Builder as ReflectionBuilder;

// Include generated proto code
pub mod helloworld {
    tonic::include_proto!("helloworld");
    pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!("helloworld_descriptor.bin");
}

pub mod streaming {
    tonic::include_proto!("streaming");
    pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!("streaming_descriptor.bin");
}

use helloworld::greeter_server::{Greeter, GreeterServer};
use helloworld::{HelloRequest, HelloReply};

use streaming::stream_service_server::{StreamService, StreamServiceServer};
use streaming::{StreamRequest, StreamResponse};

// Greeter service implementation
#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got request from {:?}", request.remote_addr());

        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }
}

// StreamService implementation
#[derive(Default)]
pub struct MyStreamService {}

#[tonic::async_trait]
impl StreamService for MyStreamService {
    // Unary
    async fn unary(
        &self,
        request: Request<StreamRequest>,
    ) -> Result<Response<StreamResponse>, Status> {
        let req = request.into_inner();
        let reply = StreamResponse {
            reply: format!("Unary response to: {}", req.message),
            sequence: req.sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        Ok(Response::new(reply))
    }

    // Server streaming
    type ServerStreamStream = tokio_stream::wrappers::ReceiverStream<Result<StreamResponse, Status>>;

    async fn server_stream(
        &self,
        request: Request<StreamRequest>,
    ) -> Result<Response<Self::ServerStreamStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        tokio::spawn(async move {
            for i in 1..=5 {
                let response = StreamResponse {
                    reply: format!("Server stream {} for: {}", i, req.message),
                    sequence: i,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                };

                if tx.send(Ok(response)).await.is_err() {
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    // Client streaming
    async fn client_stream(
        &self,
        request: Request<tonic::Streaming<StreamRequest>>,
    ) -> Result<Response<StreamResponse>, Status> {
        let mut stream = request.into_inner();
        let mut count = 0;
        let mut messages = Vec::new();

        while let Some(req) = stream.message().await? {
            count += 1;
            messages.push(req.message);
        }

        let reply = StreamResponse {
            reply: format!("Received {} messages: {}", count, messages.join(", ")),
            sequence: count,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        Ok(Response::new(reply))
    }

    // Bidirectional streaming
    type BidirectionalStream = tokio_stream::wrappers::ReceiverStream<Result<StreamResponse, Status>>;

    async fn bidirectional(
        &self,
        request: Request<tonic::Streaming<StreamRequest>>,
    ) -> Result<Response<Self::BidirectionalStream>, Status> {
        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        tokio::spawn(async move {
            let mut count = 0;
            while let Some(req) = stream.message().await.transpose() {
                match req {
                    Ok(req) => {
                        count += 1;
                        let response = StreamResponse {
                            reply: format!("Bidirectional {} for: {}", count, req.message),
                            sequence: count,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                        };

                        if tx.send(Ok(response)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    let greeter = MyGreeter::default();
    let stream_service = MyStreamService::default();

    // Build reflection service descriptor
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(helloworld::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(streaming::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    println!("gRPC test server listening on {}", addr);
    println!("\nServices:");
    println!("  - helloworld.Greeter");
    println!("    - SayHello (unary)");
    println!("  - streaming.StreamService");
    println!("    - Unary (unary)");
    println!("    - ServerStream (server streaming)");
    println!("    - ClientStream (client streaming)");
    println!("    - Bidirectional (bidirectional streaming)");
    println!("  - grpc.reflection.v1.ServerReflection (reflection)");

    Server::builder()
        .add_service(reflection_service)
        .add_service(GreeterServer::new(greeter))
        .add_service(StreamServiceServer::new(stream_service))
        .serve(addr)
        .await?;

    Ok(())
}
