/// Generates Rust code from protobuf definitions at build time.
///
/// Compiles the fleet, agent, and events protocol buffer files using Tonic and Prost code generation.
/// This function is invoked automatically by Cargo during the build process.
///
/// # Examples
///
/// This function runs automatically when building the SDK:
///
/// ```sh
/// cargo build
/// ```
/// The generated Rust code from `proto/fleet.proto`, `proto/agent.proto`, and `proto/events.proto`
/// is made available to the crate.
///
/// # Errors
///
/// Propagates any compilation errors from the protobuf compiler.
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
