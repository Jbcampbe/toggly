use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub sdk_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
}

// ---------------------------------------------------------------------------
// Flag
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FlagType {
    Boolean,
    String,
    Number,
    Json,
}

impl FlagType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagType::Boolean => "boolean",
            FlagType::String => "string",
            FlagType::Number => "number",
            FlagType::Json => "json",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "boolean" => Some(FlagType::Boolean),
            "string" => Some(FlagType::String),
            "number" => Some(FlagType::Number),
            "json" => Some(FlagType::Json),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flag {
    pub id: Uuid,
    pub project_id: Uuid,
    pub key: String,
    pub name: String,
    pub description: String,
    pub flag_type: FlagType,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagWithStates {
    #[serde(flatten)]
    pub flag: Flag,
    pub environments: Vec<FlagEnvironmentState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFlagRequest {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_flag_type")]
    pub flag_type: FlagType,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_flag_type() -> FlagType {
    FlagType::Boolean
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFlagRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Flag Environment State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagEnvironmentState {
    pub id: Uuid,
    pub flag_id: Uuid,
    pub environment_id: Uuid,
    pub environment_name: String,
    pub enabled: bool,
    pub default_value: serde_json::Value,
    pub off_value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFlagStateRequest {
    pub enabled: Option<bool>,
    pub default_value: Option<serde_json::Value>,
    pub off_value: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Evaluation response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct EvalFlag {
    pub key: String,
    pub enabled: bool,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvalAllFlags {
    pub flags: std::collections::HashMap<String, EvalFlagEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvalFlagEntry {
    pub enabled: bool,
    pub value: serde_json::Value,
}
