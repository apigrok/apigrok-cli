// Build script to compile proto files for test servers

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only compile protos if we're building test servers
    #[cfg(feature = "test-servers")]
    {
        tonic_build::configure()
            .build_server(true)
            .build_client(false)
            .file_descriptor_set_path("tests/servers/helloworld_descriptor.bin")
            .compile_protos(&["test/helloworld.proto"], &["test"])?;

        tonic_build::configure()
            .build_server(true)
            .build_client(false)
            .file_descriptor_set_path("tests/servers/streaming_descriptor.bin")
            .compile_protos(&["test/streaming.proto"], &["test"])?;
    }

    Ok(())
}
