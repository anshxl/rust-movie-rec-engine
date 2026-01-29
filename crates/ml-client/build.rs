fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the proto file for the ML client
    tonic_build::compile_protos("../../proto/recommendations.proto")?;
    Ok(())
}
