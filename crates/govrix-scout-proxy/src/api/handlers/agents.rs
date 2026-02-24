//! Agent registry handlers.
//!
//! Route map:
//!   GET  /api/v1/agents            — list_agents
//!   GET  /api/v1/agents/:id        — get_agent
//!   PUT  /api/v1/agents/:id        — update_agent
//!   POST /api/v1/agents/:id/retire — retire_agent
//!   GET  /api/v1/agents/:id/events — get_agent_events

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use govrix_scout_store::AgentFilter;

use crate::api::state::AppState;

// ── Request types ─────────────────────────────────────────────────────────────

/// Query parameters for GET /api/v1/agents
#[derive(Debug, Deserialize)]
pub struct ListAgentsParams {
    /// Filter by lifecycle status (active, idle, error, blocked)
    pub status: Option<String>,
    /// Filter by agent_type (langchain, mcp_client, etc.)
    pub agent_type: Option<String>,
    /// Substring search on agent name (case-insensitive)
    pub name: Option<String>,
    /// Maximum results (default 50, max 200)
    pub limit: Option<i64>,
    /// Offset for pagination (default 0)
    pub offset: Option<i64>,
}

/// Query parameters for GET /api/v1/agents/:id/events
#[derive(Debug, Deserialize)]
pub struct AgentEventsParams {
    /// Maximum results (default 100, max 500)
    pub limit: Option<i64>,
}

/// Body for PUT /api/v1/agents/:id
#[derive(Debug, Deserialize)]
pub struct UpdateAgentBody {
    /// Optional display name
    pub name: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Arbitrary JSON labels
    pub labels: Option<serde_json::Value>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List agents with optional filters.
///
/// GET /api/v1/agents
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListAgentsParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0).max(0);

    let filter = AgentFilter {
        status: params.status,
        agent_type: params.agent_type,
        name_contains: params.name,
        limit,
        offset,
    };

    match govrix_scout_store::agents::list_agents(&state.pool, &filter).await {
        Ok(agents) => {
            let total = agents.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": agents,
                    "total": total,
                    "limit": limit,
                    "offset": offset,
                })),
            )
        }
        Err(e) => {
            tracing::error!("list_agents store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to query agents", "detail": e.to_string() })),
            )
        }
    }
}

/// Get a single agent by ID.
///
/// GET /api/v1/agents/:id
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match govrix_scout_store::agents::get_agent(&state.pool, &id).await {
        Ok(Some(agent)) => (StatusCode::OK, Json(json!({ "data": agent }))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "agent not found", "id": id })),
        ),
        Err(e) => {
            tracing::error!("get_agent store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch agent", "detail": e.to_string() })),
            )
        }
    }
}

/// Update agent metadata (name, description, labels).
///
/// PUT /api/v1/agents/:id
pub async fn update_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateAgentBody>,
) -> impl IntoResponse {
    // First check agent exists
    match govrix_scout_store::agents::get_agent(&state.pool, &id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "agent not found", "id": id })),
            );
        }
        Err(e) => {
            tracing::error!("update_agent lookup error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch agent", "detail": e.to_string() })),
            );
        }
        Ok(Some(_)) => {}
    }

    // Apply partial updates via the store
    let result = govrix_scout_store::agents::update_agent_metadata(
        &state.pool,
        &id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.labels.as_ref(),
    )
    .await
    .map_err(|e| e.to_string());

    match result {
        Ok(_) => {
            // Return updated agent
            match govrix_scout_store::agents::get_agent(&state.pool, &id).await {
                Ok(Some(agent)) => (StatusCode::OK, Json(json!({ "data": agent }))),
                _ => (
                    StatusCode::OK,
                    Json(json!({ "data": { "id": id }, "updated": true })),
                ),
            }
        }
        Err(e) => {
            tracing::error!("update_agent store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to update agent", "detail": e.to_string() })),
            )
        }
    }
}

/// Retire an agent (set status to 'blocked').
///
/// POST /api/v1/agents/:id/retire
pub async fn retire_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Verify agent exists first
    match govrix_scout_store::agents::get_agent(&state.pool, &id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "agent not found", "id": id })),
            );
        }
        Err(e) => {
            tracing::error!("retire_agent lookup error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch agent", "detail": e.to_string() })),
            );
        }
        Ok(Some(_)) => {}
    }

    match govrix_scout_store::agents::retire_agent(&state.pool, &id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "data": { "id": id, "status": "blocked" },
                "message": "agent retired successfully",
            })),
        ),
        Err(e) => {
            tracing::error!("retire_agent store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to retire agent", "detail": e.to_string() })),
            )
        }
    }
}

/// Get recent events for a specific agent.
///
/// GET /api/v1/agents/:id/events
pub async fn get_agent_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<AgentEventsParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(100).clamp(1, 500);

    match govrix_scout_store::events::get_events_for_agent(&state.pool, &id, limit, None).await {
        Ok(events) => {
            let total = events.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": events,
                    "total": total,
                    "agent_id": id,
                    "limit": limit,
                })),
            )
        }
        Err(e) => {
            tracing::error!("get_agent_events store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch agent events", "detail": e.to_string() })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_agents_limit_clamp() {
        let params = ListAgentsParams {
            status: None,
            agent_type: None,
            name: None,
            limit: Some(9999),
            offset: None,
        };
        let limit = params.limit.unwrap_or(50).clamp(1, 200);
        assert_eq!(limit, 200);
    }

    #[test]
    fn list_agents_default_limit() {
        let params = ListAgentsParams {
            status: None,
            agent_type: None,
            name: None,
            limit: None,
            offset: None,
        };
        let limit = params.limit.unwrap_or(50).clamp(1, 200);
        let offset = params.offset.unwrap_or(0).max(0);
        assert_eq!(limit, 50);
        assert_eq!(offset, 0);
    }

    #[test]
    fn update_agent_body_optional_fields() {
        let body = UpdateAgentBody {
            name: Some("My Agent".to_string()),
            description: None,
            labels: None,
        };
        assert_eq!(body.name.as_deref(), Some("My Agent"));
        assert!(body.description.is_none());
    }
}
