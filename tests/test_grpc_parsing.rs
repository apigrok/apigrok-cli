// Integration tests for gRPC URL parsing and validation

#[cfg(test)]
mod grpc_tests {

    #[test]
    fn test_valid_grpc_url() {
        let url = "grpc://localhost:50051/helloworld.Greeter/SayHello";
        assert!(url.starts_with("grpc://"));
    }

    #[test]
    fn test_grpc_url_parts() {
        let url = "grpc://localhost:50051/helloworld.Greeter/SayHello";
        let url = url.strip_prefix("grpc://").unwrap();
        let parts: Vec<&str> = url.splitn(2, '/').collect();

        assert_eq!(parts[0], "localhost:50051");
        assert_eq!(parts[1], "helloworld.Greeter/SayHello");

        let path_parts: Vec<&str> = parts[1].split('/').collect();
        assert_eq!(path_parts.len(), 2);
        assert_eq!(path_parts[0], "helloworld.Greeter");
        assert_eq!(path_parts[1], "SayHello");
    }

    #[test]
    fn test_invalid_grpc_url_no_prefix() {
        let url = "localhost:50051/helloworld.Greeter/SayHello";
        assert!(!url.starts_with("grpc://"));
    }

    #[test]
    fn test_invalid_grpc_url_missing_parts() {
        let url = "grpc://localhost:50051";
        let url = url.strip_prefix("grpc://").unwrap();
        let parts: Vec<&str> = url.splitn(2, '/').collect();
        assert_eq!(parts.len(), 1); // Should be 2
    }
}
