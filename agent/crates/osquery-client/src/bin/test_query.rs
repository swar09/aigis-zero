use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use thrift::protocol::{
    TBinaryOutputProtocol, TFieldIdentifier, TMessageIdentifier, TMessageType, TOutputProtocol,
    TType,
};
use thrift::transport::TBufferChannel;

/// Tests registerExtension RPC communication with an osquery socket in framed and unframed modes.
///
/// Establishes connections to `/var/osquery/osquery.em`, builds a Thrift `registerExtension`
/// request with extension info and an empty registry, and sends it in two transmission modes:
/// - **Framed**: Prefixes the payload with a 4-byte big-endian length field.
/// - **Unframed**: Sends the raw payload without a length prefix.
///
/// For each mode, attempts to read a response within a 2-second timeout and logs the outcome.
///
/// # Examples
///
/// ```no_run
/// // Run to test both framed and unframed RPC modes
/// // cargo run
/// ```
///
/// # Returns
///
/// `Ok(())` if both test attempts complete, or an error if socket connection, write operations,
/// or Thrift protocol serialization fail.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = Path::new("/var/osquery/osquery.em");
    println!("Connecting to {:?}", socket);

    let mut t_reg = TBufferChannel::with_capacity(0, 1024);
    {
        let mut out_prot = TBinaryOutputProtocol::new(&mut t_reg, true);

        out_prot.write_message_begin(&TMessageIdentifier::new(
            "registerExtension",
            TMessageType::Call,
            1,
        ))?;

        out_prot.write_struct_begin(&thrift::protocol::TStructIdentifier::new(
            "registerExtension_args",
        ))?;

        out_prot.write_field_begin(&TFieldIdentifier::new("info", TType::Struct, 1))?;
        out_prot.write_struct_begin(&thrift::protocol::TStructIdentifier::new(
            "InternalExtensionInfo",
        ))?;

        out_prot.write_field_begin(&TFieldIdentifier::new("name", TType::String, 1))?;
        out_prot.write_string("aigis_zero")?;
        out_prot.write_field_end()?;

        out_prot.write_field_begin(&TFieldIdentifier::new("version", TType::String, 2))?;
        out_prot.write_string("0.1.0")?;
        out_prot.write_field_end()?;

        out_prot.write_field_begin(&TFieldIdentifier::new("sdk_version", TType::String, 3))?;
        out_prot.write_string("0.0.0")?;
        out_prot.write_field_end()?;

        out_prot.write_field_begin(&TFieldIdentifier::new("min_sdk_version", TType::String, 4))?;
        out_prot.write_string("0.0.0")?;
        out_prot.write_field_end()?;

        out_prot.write_field_stop()?;
        out_prot.write_struct_end()?;
        out_prot.write_field_end()?;

        out_prot.write_field_begin(&TFieldIdentifier::new("registry", TType::Map, 2))?;
        out_prot.write_map_begin(&thrift::protocol::TMapIdentifier::new(
            TType::String,
            TType::Map,
            0,
        ))?;
        out_prot.write_map_end()?;
        out_prot.write_field_end()?;

        out_prot.write_field_stop()?;
        out_prot.write_struct_end()?;
        out_prot.write_message_end()?;
        out_prot.flush()?;
    }

    let reg_bytes = t_reg.write_bytes();

    println!("--- Testing FAMED registerExtension ---");
    let mut stream = UnixStream::connect(socket).await?;
    let len = reg_bytes.len() as u32;
    let mut frame = Vec::with_capacity(4 + reg_bytes.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(&reg_bytes);

    stream.write_all(&frame).await?;
    stream.flush().await?;

    let mut len_buf = [0u8; 4];
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read_exact(&mut len_buf),
    )
    .await
    {
        Ok(Ok(_)) => {
            let resp_len = u32::from_be_bytes(len_buf) as usize;
            println!("FRAMED success! Response length: {}", resp_len);
        }
        Ok(Err(e)) => println!("FRAMED read error: {}", e),
        Err(_) => println!("FRAMED timed out!"),
    }

    println!("--- Testing UNFRAMED registerExtension ---");
    let mut stream2 = UnixStream::connect(socket).await?;
    stream2.write_all(&reg_bytes).await?;
    stream2.flush().await?;

    let mut resp_buf = [0u8; 1024];
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream2.read(&mut resp_buf),
    )
    .await
    {
        Ok(Ok(n)) => {
            println!("UNFRAMED success! Read {} bytes", n);
            println!("Response (hex): {:02x?}", &resp_buf[..n]);
        }
        Ok(Err(e)) => println!("UNFRAMED read error: {}", e),
        Err(_) => println!("UNFRAMED timed out!"),
    }

    Ok(())
}
