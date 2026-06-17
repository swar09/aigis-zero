use crate::types::{QueryResponse, QueryStatus};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use thrift::protocol::{
    TBinaryInputProtocol, TBinaryOutputProtocol, TFieldIdentifier, TInputProtocol,
    TMessageIdentifier, TMessageType, TOutputProtocol, TType,
};
use thrift::transport::TBufferChannel;

pub struct OsqueryClient {
    socket_path: PathBuf,
}

impl OsqueryClient {
    pub async fn connect(socket_path: &Path) -> Result<Self> {
        tracing::debug!("Connecting to osquery at {}", socket_path.display());
        Ok(Self {
            socket_path: socket_path.to_path_buf(),
        })
    }

    pub async fn query(&mut self, sql: &str) -> Result<QueryResponse> {
        tracing::debug!("Executing query: {}", sql);

        // 1. Serialize request locally
        let mut t = TBufferChannel::with_capacity(0, 1024);
        {
            let mut out_prot = TBinaryOutputProtocol::new(&mut t, true);

            out_prot.write_message_begin(&TMessageIdentifier::new(
                "query",
                TMessageType::Call,
                1,
            ))?;

            out_prot.write_struct_begin(&thrift::protocol::TStructIdentifier::new("query_args"))?;

            // Argument 1: sql (string)
            out_prot.write_field_begin(&TFieldIdentifier::new("sql", TType::String, 1))?;
            out_prot.write_string(sql)?;
            out_prot.write_field_end()?;

            out_prot.write_field_stop()?;
            out_prot.write_struct_end()?;
            out_prot.write_message_end()?;
            out_prot.flush()?;
        }

        let request_bytes = t.write_bytes();

        // 2. Connect to socket and write/read asynchronously
        let mut stream = UnixStream::connect(&self.socket_path).await?;

        // Write FRAMED header (4 bytes, big endian length)
        let len = request_bytes.len() as u32;
        let mut frame = Vec::with_capacity(4 + request_bytes.len());
        frame.extend_from_slice(&len.to_be_bytes());
        frame.extend_from_slice(&request_bytes);
        println!("Request bytes (hex): {:02x?}", frame);
        stream.write_all(&frame).await?;
        stream.flush().await?;

        // Read FRAMED header (4 bytes)
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read exactly the payload length
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await?;

        Self::parse_query_response(&buf)
    }

    fn parse_query_response(buf: &[u8]) -> Result<QueryResponse> {
        let mut t = TBufferChannel::with_capacity(buf.len(), 0);
        t.set_readable_bytes(buf);
        let mut in_prot = TBinaryInputProtocol::new(&mut t, true);

        let msg_ident = in_prot.read_message_begin()?;
        if msg_ident.message_type == TMessageType::Exception {
            let _ = thrift::Error::read_application_error_from_in_protocol(&mut in_prot)?;
            return Err(anyhow!("Thrift exception returned"));
        }

        let mut status = QueryStatus {
            code: -1,
            message: String::new(),
        };
        let mut rows = Vec::new();

        in_prot.read_struct_begin()?;
        loop {
            let field = in_prot.read_field_begin()?;
            if field.field_type == TType::Stop {
                break;
            }
            if field.id == Some(0) && field.field_type == TType::Struct {
                // ExtensionResponse
                in_prot.read_struct_begin()?;
                loop {
                    let res_field = in_prot.read_field_begin()?;
                    if res_field.field_type == TType::Stop {
                        break;
                    }
                    match res_field.id {
                        Some(1) => {
                            // ExtensionStatus
                            in_prot.read_struct_begin()?;
                            loop {
                                let st_field = in_prot.read_field_begin()?;
                                if st_field.field_type == TType::Stop {
                                    break;
                                }
                                match st_field.id {
                                    Some(1) => status.code = in_prot.read_i32()?,
                                    Some(2) => status.message = in_prot.read_string()?,
                                    _ => in_prot.skip(st_field.field_type)?,
                                }
                                in_prot.read_field_end()?;
                            }
                            in_prot.read_struct_end()?;
                        }
                        Some(2) => {
                            // list<map<string, string>> response
                            let list_ident = in_prot.read_list_begin()?;
                            for _ in 0..list_ident.size {
                                let map_ident = in_prot.read_map_begin()?;
                                let mut row = HashMap::new();
                                for _ in 0..map_ident.size {
                                    let k = in_prot.read_string()?;
                                    let v = in_prot.read_string()?;
                                    row.insert(k, v);
                                }
                                in_prot.read_map_end()?;
                                rows.push(row);
                            }
                            in_prot.read_list_end()?;
                        }
                        _ => in_prot.skip(res_field.field_type)?,
                    }
                    in_prot.read_field_end()?;
                }
                in_prot.read_struct_end()?;
            } else {
                in_prot.skip(field.field_type)?;
            }
            in_prot.read_field_end()?;
        }
        in_prot.read_struct_end()?;
        in_prot.read_message_end()?;

        Ok(QueryResponse { status, rows })
    }

    pub async fn get_query_columns(&mut self, _sql: &str) -> Result<QueryResponse> {
        Ok(QueryResponse {
            status: QueryStatus {
                code: 0,
                message: "OK".to_string(),
            },
            rows: vec![],
        })
    }

    pub async fn ping(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn live_query(&mut self, sql: &str) -> Result<QueryResponse> {
        self.query(sql).await
    }
}
