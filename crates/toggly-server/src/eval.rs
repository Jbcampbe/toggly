use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::get,
    Extension, Json, Router,
};

use toggly_core::errors::TogglyError;
use toggly_core::models::*;

use crate::middleware::sdk_key_auth;
use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/flags", get(eval_all_flags))
        .route("/flags/{key}", get(eval_single_flag))
        .route_layer(middleware::from_fn_with_state(state.clone(), sdk_key_auth))
        .with_state(state)
}

async fn eval_all_flags(
    State(state): State<Arc<AppState>>,
    Extension(env): Extension<Environment>,
) -> Result<Json<EvalAllFlags>, StatusCode> {
    let flags = state
        .db
        .eval_all_flags(env.id, env.project_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut map = HashMap::new();
    for f in flags {
        map.insert(
            f.key,
            EvalFlagEntry {
                enabled: f.enabled,
                value: f.value,
            },
        );
    }

    Ok(Json(EvalAllFlags { flags: map }))
}

async fn eval_single_flag(
    State(state): State<Arc<AppState>>,
    Extension(env): Extension<Environment>,
    Path(key): Path<String>,
) -> Result<Json<EvalFlag>, StatusCode> {
    let flag = state
        .db
        .eval_single_flag(env.id, env.project_id, &key)
        .map_err(|e| match e {
            TogglyError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(Json(flag))
}
