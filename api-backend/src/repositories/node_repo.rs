use async_trait::async_trait;
use chrono::Utc;
use diesel::{dsl::count_star, prelude::*};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    db::{
        DbPool,
        schema::{node_health, nodes},
    },
    error::AppError,
    models::node::{
        IsolateNodeResponse, NodeDetailDto, NodeEntity, NodeFilterParams, NodeHealthEntity,
        NodeSummaryDto,
    },
};

#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn find_all(
        &self,
        params: NodeFilterParams,
    ) -> Result<(Vec<NodeSummaryDto>, i64), AppError>;
    async fn find_by_id(&self, target_id: Uuid) -> Result<Option<NodeDetailDto>, AppError>;
    async fn update_operator_status(
        &self,
        target_id: Uuid,
        new_status: &str,
    ) -> Result<IsolateNodeResponse, AppError>;
}

#[derive(Clone)]
pub struct DieselNodeRepository {
    pool: DbPool,
}

impl DieselNodeRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NodeRepository for DieselNodeRepository {
    async fn find_all(
        &self,
        params: NodeFilterParams,
    ) -> Result<(Vec<NodeSummaryDto>, i64), AppError> {
        let mut conn = self.pool.get().await?;

        let mut query = nodes::table.into_boxed();
        let mut count_query = nodes::table.into_boxed();

        if let Some(ref agent_st) = params.agent_status {
            query = query.filter(nodes::agent_status.eq(agent_st));
            count_query = count_query.filter(nodes::agent_status.eq(agent_st));
        }
        if let Some(ref op_st) = params.operator_status {
            query = query.filter(nodes::operator_status.eq(op_st));
            count_query = count_query.filter(nodes::operator_status.eq(op_st));
        }
        if let Some(ref search_term) = params.search {
            let pattern = format!("%{search_term}%");
            query = query.filter(
                nodes::hostname
                    .ilike(pattern.clone())
                    .or(nodes::machine_id.ilike(pattern.clone())),
            );
            count_query = count_query.filter(
                nodes::hostname
                    .ilike(pattern.clone())
                    .or(nodes::machine_id.ilike(pattern)),
            );
        }

        let total: i64 = count_query.select(count_star()).first(&mut conn).await?;

        let limit = params.limit.unwrap_or(50).clamp(1, 100);
        let offset = params.offset.unwrap_or(0).max(0);

        let entities = query
            .order(nodes::last_enrolled_at.desc())
            .limit(limit)
            .offset(offset)
            .load::<NodeEntity>(&mut conn)
            .await?;

        let summaries = entities
            .into_iter()
            .map(|n| NodeSummaryDto {
                node_id: n.node_id,
                machine_id: n.machine_id,
                hostname: n.hostname,
                os_version: n.os_version,
                agent_version: n.agent_version,
                agent_status: n.agent_status,
                operator_status: n.operator_status,
                first_seen_at: n.first_seen_at,
                last_enrolled_at: n.last_enrolled_at,
            })
            .collect();

        Ok((summaries, total))
    }

    async fn find_by_id(&self, target_id: Uuid) -> Result<Option<NodeDetailDto>, AppError> {
        let mut conn = self.pool.get().await?;

        let node_opt = nodes::table
            .filter(nodes::node_id.eq(target_id))
            .first::<NodeEntity>(&mut conn)
            .await
            .optional()?;

        let node = match node_opt {
            Some(n) => n,
            None => return Ok(None),
        };

        let health_records = node_health::table
            .filter(node_health::node_id.eq(target_id))
            .order(node_health::recorded_at.desc())
            .limit(20)
            .load::<NodeHealthEntity>(&mut conn)
            .await?;

        Ok(Some(NodeDetailDto {
            node,
            recent_health: health_records,
        }))
    }

    async fn update_operator_status(
        &self,
        target_id: Uuid,
        new_status: &str,
    ) -> Result<IsolateNodeResponse, AppError> {
        let mut conn = self.pool.get().await?;

        let updated_node = diesel::update(nodes::table.filter(nodes::node_id.eq(target_id)))
            .set(nodes::operator_status.eq(new_status))
            .get_result::<NodeEntity>(&mut conn)
            .await
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    AppError::NotFound(format!("Node with id '{target_id}' not found"))
                }
                other => AppError::DatabaseError(other.to_string()),
            })?;

        Ok(IsolateNodeResponse {
            node_id: updated_node.node_id,
            operator_status: updated_node.operator_status,
            command_dispatched: true,
            updated_at: Utc::now(),
        })
    }
}
