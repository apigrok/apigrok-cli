use crate::error::{ApiGrokError, Result};
use crate::protocols::Protocol;
use crate::request::Request;
use crate::response::Response;
use futures_util::StreamExt;
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, ReflectMessage, Value};
use prost_types::{FileDescriptorProto, FileDescriptorSet};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::transport::{Channel, Endpoint};
use tonic_reflection::pb::v1::{
    server_reflection_client::ServerReflectionClient,
    server_reflection_request::MessageRequest,
    server_reflection_response::MessageResponse,
    ServerReflectionRequest,
};
use bytes::{Buf, BufMut};

pub struct GrpcClient;

/// Codec for dynamic gRPC messages using prost-reflect
struct DynamicCodec {
    descriptor_pool: DescriptorPool,
    message_name: String,
}

impl DynamicCodec {
    fn new(descriptor_pool: DescriptorPool, message_name: String) -> Self {
        Self {
            descriptor_pool,
            message_name,
        }
    }
}

impl Codec for DynamicCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = DynamicEncoder;
    type Decoder = DynamicDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        DynamicEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        DynamicDecoder {
            descriptor_pool: self.descriptor_pool.clone(),
            message_name: self.message_name.clone(),
        }
    }
}

struct DynamicEncoder;

impl Encoder for DynamicEncoder {
    type Item = DynamicMessage;
    type Error = tonic::Status;

    fn encode(&mut self, item: Self::Item, buf: &mut EncodeBuf<'_>) -> std::result::Result<(), Self::Error> {
        let bytes = item.encode_to_vec();
        buf.put_slice(&bytes);
        Ok(())
    }
}

struct DynamicDecoder {
    descriptor_pool: DescriptorPool,
    message_name: String,
}

impl Decoder for DynamicDecoder {
    type Item = DynamicMessage;
    type Error = tonic::Status;

    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> std::result::Result<Option<Self::Item>, Self::Error> {
        if !buf.has_remaining() {
            return Ok(None);
        }

        let chunk = buf.chunk();
        let message_descriptor = self
            .descriptor_pool
            .get_message_by_name(&self.message_name)
            .ok_or_else(|| {
                tonic::Status::internal(format!("Message type not found: {}", self.message_name))
            })?;

        let message = DynamicMessage::decode(message_descriptor, chunk).map_err(|e| {
            tonic::Status::internal(format!("Failed to decode message: {}", e))
        })?;

        buf.advance(chunk.len());
        Ok(Some(message))
    }
}

impl GrpcClient {
    pub fn new() -> Self {
        Self
    }

    /// Parse gRPC URL format: grpc://host:port/package.Service/Method
    fn parse_grpc_url(url: &str) -> Result<(String, String, String)> {
        let url = url
            .strip_prefix("grpc://")
            .ok_or_else(|| ApiGrokError::InvalidUrl("gRPC URL must start with grpc://".to_string()))?;

        let parts: Vec<&str> = url.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(ApiGrokError::InvalidUrl(
                "gRPC URL format: grpc://host:port/Service/Method".to_string(),
            ));
        }

        let endpoint = parts[0].to_string();
        let path_parts: Vec<&str> = parts[1].split('/').collect();

        if path_parts.len() != 2 {
            return Err(ApiGrokError::ProtocolError(
                "gRPC path format: Service/Method".to_string(),
            ));
        }

        let service = path_parts[0].to_string();
        let method = path_parts[1].to_string();

        Ok((endpoint, service, method))
    }

    async fn create_channel(endpoint: &str, insecure: bool) -> Result<Channel> {
        let uri = if insecure {
            format!("http://{}", endpoint)
        } else {
            format!("https://{}", endpoint)
        };

        let channel = Endpoint::from_shared(uri)
            .map_err(|e| ApiGrokError::InvalidUrl(e.to_string()))?
            .connect()
            .await
            .map_err(|e| ApiGrokError::NetworkError(e.to_string()))?;

        Ok(channel)
    }

    /// Parse proto file to FileDescriptorSet
    fn parse_proto_file(proto_path: &str) -> Result<FileDescriptorSet> {
        // Check if file exists
        let path = Path::new(proto_path);
        if !path.exists() {
            return Err(ApiGrokError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Proto file not found: {}", proto_path),
            )));
        }

        // Parse proto file using protox
        let file_descriptor_set = protox::compile([proto_path], ["."])
            .map_err(|e| ApiGrokError::ProtocolError(format!("Failed to parse proto file: {}", e)))?;

        Ok(file_descriptor_set)
    }

    /// Query server reflection to get FileDescriptorSet for a service
    async fn query_reflection(
        channel: Channel,
        service_name: &str,
    ) -> Result<FileDescriptorSet> {
        // Create reflection client
        let mut client = ServerReflectionClient::new(channel);

        // Build request for file containing the service symbol
        let request = ServerReflectionRequest {
            host: String::new(),
            message_request: Some(MessageRequest::FileContainingSymbol(
                service_name.to_string(),
            )),
        };

        // Send request via bidirectional stream
        let stream = tokio_stream::once(request);
        let mut response_stream = client
            .server_reflection_info(stream)
            .await
            .map_err(|e| ApiGrokError::ProtocolError(format!(
                "Failed to connect to reflection service: {}. Make sure the server has reflection enabled.",
                e
            )))?
            .into_inner();

        // Collect FileDescriptorProto from responses
        let mut file_descriptor_protos = Vec::new();

        while let Some(response_result) = response_stream.next().await {
            let response = response_result
                .map_err(|e| ApiGrokError::ProtocolError(format!("Reflection stream error: {}", e)))?;

            match response.message_response {
                Some(MessageResponse::FileDescriptorResponse(desc)) => {
                    for proto_bytes in desc.file_descriptor_proto {
                        let file_proto = FileDescriptorProto::decode(proto_bytes.as_slice())
                            .map_err(|e| ApiGrokError::ProtocolError(format!(
                                "Failed to decode FileDescriptorProto: {}",
                                e
                            )))?;
                        file_descriptor_protos.push(file_proto);
                    }
                }
                Some(MessageResponse::ErrorResponse(err)) => {
                    return Err(ApiGrokError::ProtocolError(format!(
                        "Reflection error: {} (code: {})",
                        err.error_message, err.error_code
                    )));
                }
                _ => {}
            }
        }

        if file_descriptor_protos.is_empty() {
            return Err(ApiGrokError::ProtocolError(format!(
                "No file descriptors returned for service '{}'",
                service_name
            )));
        }

        // Build FileDescriptorSet
        let mut file_descriptor_set = FileDescriptorSet::default();
        file_descriptor_set.file = file_descriptor_protos;

        Ok(file_descriptor_set)
    }

    /// List all services available via reflection
    async fn list_services(channel: Channel) -> Result<Vec<String>> {
        let mut client = ServerReflectionClient::new(channel);

        let request = ServerReflectionRequest {
            host: String::new(),
            message_request: Some(MessageRequest::ListServices(String::new())),
        };

        let stream = tokio_stream::once(request);
        let mut response_stream = client
            .server_reflection_info(stream)
            .await
            .map_err(|e| ApiGrokError::ProtocolError(format!(
                "Failed to connect to reflection service: {}",
                e
            )))?
            .into_inner();

        let mut services = Vec::new();

        while let Some(response_result) = response_stream.next().await {
            let response = response_result
                .map_err(|e| ApiGrokError::ProtocolError(format!("Reflection stream error: {}", e)))?;

            match response.message_response {
                Some(MessageResponse::ListServicesResponse(list)) => {
                    for service in list.service {
                        services.push(service.name);
                    }
                }
                Some(MessageResponse::ErrorResponse(err)) => {
                    return Err(ApiGrokError::ProtocolError(format!(
                        "Reflection error: {}",
                        err.error_message
                    )));
                }
                _ => {}
            }
        }

        Ok(services)
    }

    /// Convert JSON to DynamicMessage
    fn json_to_dynamic_message(
        json_str: &str,
        descriptor_pool: &DescriptorPool,
        message_name: &str,
    ) -> Result<DynamicMessage> {
        let json_value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| ApiGrokError::ParseError(format!("Invalid JSON: {}", e)))?;

        let message_descriptor = descriptor_pool
            .get_message_by_name(message_name)
            .ok_or_else(|| {
                ApiGrokError::ProtocolError(format!("Message type not found: {}", message_name))
            })?;

        let mut message = DynamicMessage::new(message_descriptor);

        // Convert JSON to DynamicMessage fields
        Self::populate_message_from_json(&mut message, &json_value)?;

        Ok(message)
    }

    /// Populate DynamicMessage from JSON value
    fn populate_message_from_json(
        message: &mut DynamicMessage,
        json: &serde_json::Value,
    ) -> Result<()> {
        if let serde_json::Value::Object(obj) = json {
            let descriptor = message.descriptor();

            for (key, value) in obj {
                if let Some(field) = descriptor.get_field_by_name(key) {
                    let prost_value = Self::json_value_to_prost_value(value, &field)?;
                    message.set_field(&field, prost_value);
                }
            }
        }

        Ok(())
    }

    /// Convert serde_json::Value to prost_reflect::Value
    fn json_value_to_prost_value(
        json: &serde_json::Value,
        field: &prost_reflect::FieldDescriptor,
    ) -> Result<Value> {
        use prost_reflect::Kind;

        let value = match field.kind() {
            Kind::Double => Value::F64(json.as_f64().unwrap_or(0.0)),
            Kind::Float => Value::F32(json.as_f64().unwrap_or(0.0) as f32),
            Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
                Value::I32(json.as_i64().unwrap_or(0) as i32)
            }
            Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => Value::I64(json.as_i64().unwrap_or(0)),
            Kind::Uint32 | Kind::Fixed32 => Value::U32(json.as_u64().unwrap_or(0) as u32),
            Kind::Uint64 | Kind::Fixed64 => Value::U64(json.as_u64().unwrap_or(0)),
            Kind::Bool => Value::Bool(json.as_bool().unwrap_or(false)),
            Kind::String => Value::String(json.as_str().unwrap_or("").to_string()),
            Kind::Bytes => {
                let s = json.as_str().unwrap_or("");
                let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
                    .unwrap_or_default();
                Value::Bytes(bytes.into())
            }
            Kind::Message(msg_desc) => {
                let mut msg = DynamicMessage::new(msg_desc.clone());
                Self::populate_message_from_json(&mut msg, json)?;
                Value::Message(msg)
            }
            Kind::Enum(enum_desc) => {
                if let Some(s) = json.as_str() {
                    if let Some(enum_value) = enum_desc.get_value_by_name(s) {
                        Value::EnumNumber(enum_value.number())
                    } else {
                        Value::EnumNumber(0)
                    }
                } else {
                    Value::EnumNumber(json.as_i64().unwrap_or(0) as i32)
                }
            }
        };

        Ok(value)
    }

    /// Convert prost_reflect::Value to serde_json::Value
    fn prost_value_to_json(value: &Value) -> serde_json::Value {
        match value {
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::I32(i) => serde_json::Value::Number((*i).into()),
            Value::I64(i) => serde_json::Value::Number((*i).into()),
            Value::U32(u) => serde_json::Value::Number((*u).into()),
            Value::U64(u) => serde_json::Value::Number((*u).into()),
            Value::F32(f) => serde_json::Value::Number(
                serde_json::Number::from_f64(*f as f64).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
            Value::F64(f) => serde_json::Value::Number(
                serde_json::Number::from_f64(*f).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Bytes(b) => {
                serde_json::Value::String(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b))
            }
            Value::EnumNumber(n) => serde_json::Value::Number((*n).into()),
            Value::Message(msg) => Self::dynamic_message_to_json(msg),
            Value::List(list) => {
                serde_json::Value::Array(list.iter().map(Self::prost_value_to_json).collect())
            }
            Value::Map(map) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in map.iter() {
                    let key = match k {
                        prost_reflect::MapKey::Bool(b) => b.to_string(),
                        prost_reflect::MapKey::I32(i) => i.to_string(),
                        prost_reflect::MapKey::I64(i) => i.to_string(),
                        prost_reflect::MapKey::U32(u) => u.to_string(),
                        prost_reflect::MapKey::U64(u) => u.to_string(),
                        prost_reflect::MapKey::String(s) => s.clone(),
                    };
                    obj.insert(key, Self::prost_value_to_json(v));
                }
                serde_json::Value::Object(obj)
            }
        }
    }

    /// Convert DynamicMessage to JSON
    fn dynamic_message_to_json(message: &DynamicMessage) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        let descriptor = message.descriptor();

        for field in descriptor.fields() {
            if message.has_field(&field) {
                let value = message.get_field(&field);
                obj.insert(field.name().to_string(), Self::prost_value_to_json(&value));
            }
        }

        serde_json::Value::Object(obj)
    }

    /// Load streaming input from file (one JSON message per line)
    fn load_stream_input(
        file_path: &str,
        descriptor_pool: &DescriptorPool,
        message_type: &str,
    ) -> Result<Vec<DynamicMessage>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open(file_path)
            .map_err(|e| ApiGrokError::IoError(e))?;

        let reader = BufReader::new(file);
        let mut messages = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| ApiGrokError::IoError(e))?;
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            let msg = Self::json_to_dynamic_message(line, descriptor_pool, message_type)
                .map_err(|e| ApiGrokError::ParseError(format!(
                    "Line {}: {}",
                    line_num + 1,
                    e
                )))?;

            messages.push(msg);
        }

        if messages.is_empty() {
            return Err(ApiGrokError::ParseError(
                "Stream input file is empty".to_string()
            ));
        }

        Ok(messages)
    }

    /// Invoke a gRPC unary method dynamically
    async fn invoke_method(
        channel: Channel,
        service: &str,
        method: &str,
        request_msg: DynamicMessage,
        descriptor_pool: &DescriptorPool,
        response_type: &str,
    ) -> Result<DynamicMessage> {
        use tower::ServiceExt;  // For ready() method

        let path = format!("/{}/{}", service, method);

        let codec = DynamicCodec::new(descriptor_pool.clone(), response_type.to_string());
        let mut client = tonic::client::Grpc::new(channel);

        // Ensure the client is ready before making the request
        client.ready().await
            .map_err(|e| ApiGrokError::ProtocolError(format!("gRPC client not ready: {}", e)))?;

        let request = tonic::Request::new(request_msg);

        let response = client
            .unary(request, path.parse().unwrap(), codec)
            .await
            .map_err(|e| ApiGrokError::ProtocolError(format!("gRPC call failed: {}", e)))?;

        Ok(response.into_inner())
    }

    /// Invoke a gRPC server streaming method dynamically
    async fn invoke_server_streaming(
        channel: Channel,
        service: &str,
        method: &str,
        request_msg: DynamicMessage,
        descriptor_pool: &DescriptorPool,
        response_type: &str,
    ) -> Result<Vec<DynamicMessage>> {
        use tower::ServiceExt;  // For ready() method

        let path = format!("/{}/{}", service, method);

        let codec = DynamicCodec::new(descriptor_pool.clone(), response_type.to_string());
        let mut client = tonic::client::Grpc::new(channel);

        // Ensure the client is ready before making the request
        client.ready().await
            .map_err(|e| ApiGrokError::ProtocolError(format!("gRPC client not ready: {}", e)))?;

        let request = tonic::Request::new(request_msg);

        let mut response_stream = client
            .server_streaming(request, path.parse().unwrap(), codec)
            .await
            .map_err(|e| ApiGrokError::ProtocolError(format!("gRPC call failed: {}", e)))?
            .into_inner();

        // Collect all responses from the stream
        let mut responses = Vec::new();
        while let Some(response) = response_stream.next().await {
            let msg = response
                .map_err(|e| ApiGrokError::ProtocolError(format!("Stream error: {}", e)))?;
            responses.push(msg);
        }

        Ok(responses)
    }

    /// Invoke a gRPC client streaming method dynamically
    async fn invoke_client_streaming(
        channel: Channel,
        service: &str,
        method: &str,
        request_messages: Vec<DynamicMessage>,
        descriptor_pool: &DescriptorPool,
        response_type: &str,
    ) -> Result<DynamicMessage> {
        use tower::ServiceExt;  // For ready() method

        let path = format!("/{}/{}", service, method);

        let codec = DynamicCodec::new(descriptor_pool.clone(), response_type.to_string());
        let mut client = tonic::client::Grpc::new(channel);

        // Ensure the client is ready before making the request
        client.ready().await
            .map_err(|e| ApiGrokError::ProtocolError(format!("gRPC client not ready: {}", e)))?;

        // Create a stream from the vector of messages
        let stream = tokio_stream::iter(request_messages);
        let request = tonic::Request::new(stream);

        let response = client
            .client_streaming(request, path.parse().unwrap(), codec)
            .await
            .map_err(|e| ApiGrokError::ProtocolError(format!("gRPC call failed: {}", e)))?;

        Ok(response.into_inner())
    }

    /// Invoke a gRPC bidirectional streaming method dynamically
    async fn invoke_bidirectional_streaming(
        channel: Channel,
        service: &str,
        method: &str,
        request_messages: Vec<DynamicMessage>,
        descriptor_pool: &DescriptorPool,
        response_type: &str,
    ) -> Result<Vec<DynamicMessage>> {
        use tower::ServiceExt;  // For ready() method

        let path = format!("/{}/{}", service, method);

        let codec = DynamicCodec::new(descriptor_pool.clone(), response_type.to_string());
        let mut client = tonic::client::Grpc::new(channel);

        // Ensure the client is ready before making the request
        client.ready().await
            .map_err(|e| ApiGrokError::ProtocolError(format!("gRPC client not ready: {}", e)))?;

        // Create a stream from the vector of messages
        let stream = tokio_stream::iter(request_messages);
        let request = tonic::Request::new(stream);

        let mut response_stream = client
            .streaming(request, path.parse().unwrap(), codec)
            .await
            .map_err(|e| ApiGrokError::ProtocolError(format!("gRPC call failed: {}", e)))?
            .into_inner();

        // Collect all responses from the stream
        let mut responses = Vec::new();
        while let Some(response) = response_stream.next().await {
            let msg = response
                .map_err(|e| ApiGrokError::ProtocolError(format!("Stream error: {}", e)))?;
            responses.push(msg);
        }

        Ok(responses)
    }
}

#[async_trait::async_trait]
impl Protocol for GrpcClient {
    async fn execute(&self, request: &Request) -> Result<Response> {
        let start = Instant::now();

        // Parse the gRPC URL (for list-services, service/method can be dummy)
        let (endpoint, service, method) = if request.list_services {
            // For list services, we only need the endpoint
            let url = request.url.strip_prefix("grpc://")
                .ok_or_else(|| ApiGrokError::InvalidUrl("gRPC URL must start with grpc://".to_string()))?;
            let endpoint = url.split('/').next().unwrap_or(url).to_string();
            (endpoint, String::new(), String::new())
        } else {
            Self::parse_grpc_url(&request.url)?
        };

        // Create channel (secure or insecure)
        let channel = Self::create_channel(&endpoint, request.insecure).await?;

        // Handle list-services flag
        if request.list_services {
            let services = Self::list_services(channel.clone()).await?;
            let elapsed = start.elapsed().as_millis();

            let msg = format!(
                "✓ Services Available via Reflection\n\
                \n\
                Endpoint: {}\n\
                Mode: {}\n\
                Time: {}ms\n\
                \n\
                Services ({}):\n{}",
                endpoint,
                if request.insecure { "insecure (http)" } else { "secure (https)" },
                elapsed,
                services.len(),
                services.iter()
                    .map(|s| format!("  - {}", s))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            return Ok(Response::new(
                200,
                "OK".to_string(),
                HashMap::new(),
                msg.into_bytes(),
                "gRPC".to_string(),
                elapsed,
            ));
        }

        // Check if proto file or reflection is provided for dynamic invocation
        if request.proto.is_some() || request.reflection {
            // Dynamic gRPC call with proto file or reflection
            return self.execute_dynamic(
                channel,
                &endpoint,
                &service,
                &method,
                request,
                start,
            ).await;
        }

        // No proto file or reflection - connection test mode
        let msg = format!(
            "✓ gRPC Connection Established\n\
            \n\
            Endpoint: {}\n\
            Service: {}\n\
            Method: {}\n\
            Mode: {}\n\
            \n\
            STATUS: Connection Test Complete ✓\n\
            \n\
            To make dynamic gRPC calls:\n\
            \n\
            Using proto file:\n\
            $ apigrok grpc://localhost:50051/helloworld.Greeter/SayHello \\\n\
              --grpc --insecure \\\n\
              --proto helloworld.proto \\\n\
              -j '{{\"name\":\"World\"}}'\n\
            \n\
            Using server reflection:\n\
            $ apigrok grpc://localhost:50051/helloworld.Greeter/SayHello \\\n\
              --grpc --insecure \\\n\
              --reflection \\\n\
              -j '{{\"name\":\"World\"}}'\n\
            \n\
            List available services:\n\
            $ apigrok grpc://localhost:50051 --grpc --insecure --list-services\n\
            \n\
            ✅ IMPLEMENTED:\n\
            - URL parsing (grpc://host:port/Service/Method)\n\
            - Connection management (secure/insecure)\n\
            - TLS and non-TLS support\n\
            - Proto file parsing with --proto flag\n\
            - Server reflection with --reflection flag\n\
            - Service discovery with --list-services\n\
            - All streaming modes (unary, server, client, bidirectional)\n\
            - Custom DynamicCodec for prost-reflect messages\n\
            - JSON to protobuf conversion\n\
            - Dynamic method invocation\n\
            \n\
            ALTERNATIVE TOOLS:\n\
            - grpcurl: CLI with similar features\n\
            - Postman: GUI with gRPC support\n\
            - BloomRPC: Desktop GUI client",
            endpoint,
            service,
            method,
            if request.insecure { "insecure (http)" } else { "secure (https)" }
        );

        let elapsed = start.elapsed().as_millis();

        Ok(Response::new(
            200,
            "Connection OK".to_string(),
            HashMap::new(),
            msg.into_bytes(),
            "gRPC".to_string(),
            elapsed,
        ))
    }
}

impl GrpcClient {
    async fn execute_dynamic(
        &self,
        channel: Channel,
        endpoint: &str,
        service: &str,
        method: &str,
        request: &Request,
        start: Instant,
    ) -> Result<Response> {
        // Get FileDescriptorSet from proto file or reflection
        let file_descriptor_set = if let Some(proto_path) = &request.proto {
            // Parse proto file
            Self::parse_proto_file(proto_path)?
        } else if request.reflection {
            // Query server reflection
            Self::query_reflection(channel.clone(), service).await?
        } else {
            return Err(ApiGrokError::ProtocolError(
                "Either --proto or --reflection must be specified for dynamic gRPC calls".to_string()
            ));
        };

        // Build descriptor pool
        let descriptor_pool = DescriptorPool::decode(file_descriptor_set.encode_to_vec().as_slice())
            .map_err(|e| ApiGrokError::ProtocolError(format!("Failed to build descriptor pool: {}", e)))?;

        // Find service descriptor
        let service_descriptor = descriptor_pool
            .get_service_by_name(service)
            .ok_or_else(|| {
                ApiGrokError::ProtocolError(format!(
                    "Service '{}' not found in proto file. Available services: {}",
                    service,
                    descriptor_pool
                        .services()
                        .map(|s| s.name().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;

        // Find method descriptor
        let method_descriptor = service_descriptor
            .methods()
            .find(|m| m.name() == method)
            .ok_or_else(|| {
                ApiGrokError::ProtocolError(format!(
                    "Method '{}' not found in service '{}'. Available methods: {}",
                    method,
                    service,
                    service_descriptor
                        .methods()
                        .map(|m| m.name().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;

        // Get request and response message types
        let request_type = method_descriptor.input().full_name().to_string();
        let response_type = method_descriptor.output().full_name().to_string();

        // Detect streaming type
        let is_client_streaming = method_descriptor.is_client_streaming();
        let is_server_streaming = method_descriptor.is_server_streaming();

        // Prepare request message(s)
        let (request_msgs, streaming_type) = if is_client_streaming {
            // Client streaming or bidirectional - need multiple messages
            let messages = if let Some(input_file) = &request.stream_input {
                // Read messages from file (one JSON per line)
                Self::load_stream_input(input_file, &descriptor_pool, &request_type)?
            } else {
                // Single message from body for now (could be extended)
                let json_body = if let Some(body) = &request.body {
                    String::from_utf8(body.clone())
                        .map_err(|e| ApiGrokError::ParseError(format!("Invalid UTF-8 in body: {}", e)))?
                } else {
                    "{}".to_string()
                };
                vec![Self::json_to_dynamic_message(&json_body, &descriptor_pool, &request_type)?]
            };

            let stream_type = if is_server_streaming {
                "bidirectional"
            } else {
                "client_streaming"
            };
            (messages, stream_type)
        } else {
            // Unary or server streaming - single request message
            let json_body = if let Some(body) = &request.body {
                String::from_utf8(body.clone())
                    .map_err(|e| ApiGrokError::ParseError(format!("Invalid UTF-8 in body: {}", e)))?
            } else {
                "{}".to_string()
            };
            let msg = Self::json_to_dynamic_message(&json_body, &descriptor_pool, &request_type)?;
            let stream_type = if is_server_streaming {
                "server_streaming"
            } else {
                "unary"
            };
            (vec![msg], stream_type)
        };

        // Invoke method based on streaming type
        let (response_json, message_count) = match (is_client_streaming, is_server_streaming) {
            (false, false) => {
                // Unary
                let response_msg = Self::invoke_method(
                    channel,
                    service,
                    method,
                    request_msgs.into_iter().next().unwrap(),
                    &descriptor_pool,
                    &response_type,
                )
                .await?;

                let json_value = Self::dynamic_message_to_json(&response_msg);
                let json_str = serde_json::to_string_pretty(&json_value)
                    .map_err(|e| ApiGrokError::ParseError(format!("Failed to serialize response: {}", e)))?;
                (json_str, 1)
            }
            (false, true) => {
                // Server streaming
                let response_msgs = Self::invoke_server_streaming(
                    channel,
                    service,
                    method,
                    request_msgs.into_iter().next().unwrap(),
                    &descriptor_pool,
                    &response_type,
                )
                .await?;

                let count = response_msgs.len();
                let json_values: Vec<_> = response_msgs.iter()
                    .map(Self::dynamic_message_to_json)
                    .collect();
                let json_str = serde_json::to_string_pretty(&json_values)
                    .map_err(|e| ApiGrokError::ParseError(format!("Failed to serialize response: {}", e)))?;
                (json_str, count)
            }
            (true, false) => {
                // Client streaming
                let response_msg = Self::invoke_client_streaming(
                    channel,
                    service,
                    method,
                    request_msgs,
                    &descriptor_pool,
                    &response_type,
                )
                .await?;

                let json_value = Self::dynamic_message_to_json(&response_msg);
                let json_str = serde_json::to_string_pretty(&json_value)
                    .map_err(|e| ApiGrokError::ParseError(format!("Failed to serialize response: {}", e)))?;
                (json_str, 1)
            }
            (true, true) => {
                // Bidirectional streaming
                let response_msgs = Self::invoke_bidirectional_streaming(
                    channel,
                    service,
                    method,
                    request_msgs,
                    &descriptor_pool,
                    &response_type,
                )
                .await?;

                let count = response_msgs.len();
                let json_values: Vec<_> = response_msgs.iter()
                    .map(Self::dynamic_message_to_json)
                    .collect();
                let json_str = serde_json::to_string_pretty(&json_values)
                    .map_err(|e| ApiGrokError::ParseError(format!("Failed to serialize response: {}", e)))?;
                (json_str, count)
            }
        };

        let response_json = response_json;

        let elapsed = start.elapsed().as_millis();

        let summary = format!(
            "✓ gRPC Call Successful\n\
            \n\
            Endpoint: {}\n\
            Service: {}\n\
            Method: {}\n\
            Mode: {}\n\
            Streaming: {}\n\
            {} Time: {}ms\n\
            \n\
            Response:\n\
            {}",
            endpoint,
            service,
            method,
            if request.insecure { "insecure (http)" } else { "secure (https)" },
            streaming_type,
            if message_count > 1 {
                format!("Messages: {}\n", message_count)
            } else {
                String::new()
            },
            elapsed,
            response_json
        );

        Ok(Response::new(
            200,
            "OK".to_string(),
            HashMap::new(),
            summary.into_bytes(),
            "gRPC".to_string(),
            elapsed,
        ))
    }
}
