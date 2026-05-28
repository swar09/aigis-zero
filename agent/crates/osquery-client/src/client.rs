use crate::types::{QueryResponse, QueryStatus};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct OsqueryClient {
    socket_path: PathBuf,
}

impl OsqueryClient {
    pub async fn connect(socket_path: &Path) -> Result<Self> {
        Ok(Self {
            socket_path: socket_path.to_path_buf(),
        })
    }

    pub async fn query(&mut self, sql: &str) -> Result<QueryResponse> {
        // Stub for now. Will require Thrift serialization over UnixStream.
        tracing::debug!("Executing query: {}", sql);
        Ok(QueryResponse {
            status: QueryStatus {
                code: 0,
                message: "OK".to_string(),
            },
            rows: vec![],
        })
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
