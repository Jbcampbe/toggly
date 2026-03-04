use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use toggly_core::errors::TogglyError;
use toggly_core::models::*;

use crate::middleware::admin_auth;
use crate::state::AppState;

/// Map our domain errors to HTTP status codes + JSON body.
fn error_response(err: TogglyError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, msg) = match &err {
        TogglyError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
        TogglyError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
        TogglyError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
        TogglyError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Projects
        .route("/projects", post(create_project).get(list_projects))
        .route(
            "/projects/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
        // Environments
        .route(
            "/projects/{project_id}/environments",
            post(create_environment).get(list_environments),
        )
        .route(
            "/projects/{project_id}/environments/{env_id}",
            delete(delete_environment),
        )
        // Flags
        .route(
            "/projects/{project_id}/flags",
            post(create_flag).get(list_flags),
        )
        .route(
            "/projects/{project_id}/flags/{key}",
            get(get_flag).put(update_flag).delete(delete_flag),
        )
        // Flag state
        .route(
            "/projects/{project_id}/flags/{key}/environments/{env_id}",
            put(update_flag_state),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), admin_auth))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>), (StatusCode, Json<serde_json::Value>)> {
    let project = state.db.create_project(&req).map_err(error_response)?;
    Ok((StatusCode::CREATED, Json(project)))
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Project>>, (StatusCode, Json<serde_json::Value>)> {
    let projects = state.db.list_projects().map_err(error_response)?;
    Ok(Json(projects))
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Project>, (StatusCode, Json<serde_json::Value>)> {
    let project = state.db.get_project(id).map_err(error_response)?;
    Ok(Json(project))
}

async fn update_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, (StatusCode, Json<serde_json::Value>)> {
    let project = state.db.update_project(id, &req).map_err(error_response)?;
    Ok(Json(project))
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    state.db.delete_project(id).map_err(error_response)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Environments
// ---------------------------------------------------------------------------

async fn create_environment(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateEnvironmentRequest>,
) -> Result<(StatusCode, Json<Environment>), (StatusCode, Json<serde_json::Value>)> {
    let env = state
        .db
        .create_environment(project_id, &req)
        .map_err(error_response)?;
    Ok((StatusCode::CREATED, Json(env)))
}

async fn list_environments(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<Environment>>, (StatusCode, Json<serde_json::Value>)> {
    let envs = state
        .db
        .list_environments(project_id)
        .map_err(error_response)?;
    Ok(Json(envs))
}

async fn delete_environment(
    State(state): State<Arc<AppState>>,
    Path((project_id, env_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    state
        .db
        .delete_environment(project_id, env_id)
        .map_err(error_response)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

async fn create_flag(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateFlagRequest>,
) -> Result<(StatusCode, Json<Flag>), (StatusCode, Json<serde_json::Value>)> {
    let flag = state
        .db
        .create_flag(project_id, &req)
        .map_err(error_response)?;
    Ok((StatusCode::CREATED, Json(flag)))
}

async fn list_flags(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<Flag>>, (StatusCode, Json<serde_json::Value>)> {
    let flags = state.db.list_flags(project_id).map_err(error_response)?;
    Ok(Json(flags))
}

async fn get_flag(
    State(state): State<Arc<AppState>>,
    Path((project_id, key)): Path<(Uuid, String)>,
) -> Result<Json<FlagWithStates>, (StatusCode, Json<serde_json::Value>)> {
    let flag = state.db.get_flag(project_id, &key).map_err(error_response)?;
    Ok(Json(flag))
}

async fn update_flag(
    State(state): State<Arc<AppState>>,
    Path((project_id, key)): Path<(Uuid, String)>,
    Json(req): Json<UpdateFlagRequest>,
) -> Result<Json<Flag>, (StatusCode, Json<serde_json::Value>)> {
    let flag = state
        .db
        .update_flag(project_id, &key, &req)
        .map_err(error_response)?;
    Ok(Json(flag))
}

async fn delete_flag(
    State(state): State<Arc<AppState>>,
    Path((project_id, key)): Path<(Uuid, String)>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    state
        .db
        .delete_flag(project_id, &key)
        .map_err(error_response)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Flag State
// ---------------------------------------------------------------------------

async fn update_flag_state(
    State(state): State<Arc<AppState>>,
    Path((project_id, key, env_id)): Path<(Uuid, String, Uuid)>,
    Json(req): Json<UpdateFlagStateRequest>,
) -> Result<Json<FlagEnvironmentState>, (StatusCode, Json<serde_json::Value>)> {
    let flag_state = state
        .db
        .update_flag_state(project_id, &key, env_id, &req)
        .map_err(error_response)?;
    Ok(Json(flag_state))
}
