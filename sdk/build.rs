fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure().compile_protos(
        &[
            "proto/fleet.proto",
            "proto/agent.proto",
            "proto/events.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
