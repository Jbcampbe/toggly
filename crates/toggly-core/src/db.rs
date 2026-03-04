use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::errors::{Result, TogglyError};
use crate::models::*;

/// Thread-safe wrapper around a SQLite connection.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS environments (
                id          TEXT PRIMARY KEY,
                project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                name        TEXT NOT NULL,
                sdk_key     TEXT NOT NULL UNIQUE,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS flags (
                id          TEXT PRIMARY KEY,
                project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                key         TEXT NOT NULL,
                name        TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                flag_type   TEXT NOT NULL DEFAULT 'boolean',
                tags        TEXT NOT NULL DEFAULT '[]',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                UNIQUE(project_id, key)
            );

            CREATE TABLE IF NOT EXISTS flag_environment_states (
                id              TEXT PRIMARY KEY,
                flag_id         TEXT NOT NULL REFERENCES flags(id) ON DELETE CASCADE,
                environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
                enabled         INTEGER NOT NULL DEFAULT 0,
                default_value   TEXT NOT NULL DEFAULT 'true',
                off_value       TEXT NOT NULL DEFAULT 'false',
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL,
                UNIQUE(flag_id, environment_id)
            );
            ",
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Projects
    // -----------------------------------------------------------------------

    pub fn create_project(&self, req: &CreateProjectRequest) -> Result<Project> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let project = Project {
            id: Uuid::new_v4(),
            name: req.name.clone(),
            description: req.description.clone(),
            created_at: now,
            updated_at: now,
        };
        conn.execute(
            "INSERT INTO projects (id, name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                project.id.to_string(),
                project.name,
                project.description,
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(project)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, description, created_at, updated_at FROM projects ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get::<_, String>(3)?.parse().unwrap(),
                updated_at: row.get::<_, String>(4)?.parse().unwrap(),
            })
        })?;
        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        Ok(projects)
    }

    pub fn get_project(&self, id: Uuid) -> Result<Project> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, description, created_at, updated_at FROM projects WHERE id = ?1",
            params![id.to_string()],
            |row| {
                Ok(Project {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get::<_, String>(3)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(4)?.parse().unwrap(),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                TogglyError::NotFound(format!("Project {id} not found"))
            }
            other => TogglyError::Database(other),
        })
    }

    pub fn update_project(&self, id: Uuid, req: &UpdateProjectRequest) -> Result<Project> {
        let existing = self.get_project(id)?;
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let name = req.name.as_deref().unwrap_or(&existing.name);
        let description = req.description.as_deref().unwrap_or(&existing.description);
        conn.execute(
            "UPDATE projects SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
            params![name, description, now.to_rfc3339(), id.to_string()],
        )?;
        Ok(Project {
            id,
            name: name.to_string(),
            description: description.to_string(),
            created_at: existing.created_at,
            updated_at: now,
        })
    }

    pub fn delete_project(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM projects WHERE id = ?1", params![id.to_string()])?;
        if affected == 0 {
            return Err(TogglyError::NotFound(format!("Project {id} not found")));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Environments
    // -----------------------------------------------------------------------

    pub fn create_environment(
        &self,
        project_id: Uuid,
        req: &CreateEnvironmentRequest,
    ) -> Result<Environment> {
        // Verify project exists
        self.get_project(project_id)?;

        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let env = Environment {
            id: Uuid::new_v4(),
            project_id,
            name: req.name.clone(),
            sdk_key: format!("sdk-{}", Uuid::new_v4()),
            created_at: now,
            updated_at: now,
        };
        conn.execute(
            "INSERT INTO environments (id, project_id, name, sdk_key, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                env.id.to_string(),
                env.project_id.to_string(),
                env.name,
                env.sdk_key,
                env.created_at.to_rfc3339(),
                env.updated_at.to_rfc3339(),
            ],
        )?;

        // Create flag_environment_states for all existing flags in this project
        let flag_ids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM flags WHERE project_id = ?1")?;
            let rows = stmt.query_map(params![project_id.to_string()], |row| {
                row.get::<_, String>(0)
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        for flag_id in &flag_ids {
            let state_id = Uuid::new_v4();
            conn.execute(
                "INSERT INTO flag_environment_states (id, flag_id, environment_id, enabled, default_value, off_value, created_at, updated_at) VALUES (?1, ?2, ?3, 0, 'false', 'false', ?4, ?5)",
                params![
                    state_id.to_string(),
                    flag_id,
                    env.id.to_string(),
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                ],
            )?;
        }

        Ok(env)
    }

    pub fn list_environments(&self, project_id: Uuid) -> Result<Vec<Environment>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, sdk_key, created_at, updated_at FROM environments WHERE project_id = ?1 ORDER BY name",
        )?;
        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            Ok(Environment {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                name: row.get(2)?,
                sdk_key: row.get(3)?,
                created_at: row.get::<_, String>(4)?.parse().unwrap(),
                updated_at: row.get::<_, String>(5)?.parse().unwrap(),
            })
        })?;
        let mut envs = Vec::new();
        for row in rows {
            envs.push(row?);
        }
        Ok(envs)
    }

    pub fn delete_environment(&self, project_id: Uuid, env_id: Uuid) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM environments WHERE id = ?1 AND project_id = ?2",
            params![env_id.to_string(), project_id.to_string()],
        )?;
        if affected == 0 {
            return Err(TogglyError::NotFound(format!(
                "Environment {env_id} not found in project {project_id}"
            )));
        }
        Ok(())
    }

    pub fn get_environment_by_sdk_key(&self, sdk_key: &str) -> Result<Environment> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, project_id, name, sdk_key, created_at, updated_at FROM environments WHERE sdk_key = ?1",
            params![sdk_key],
            |row| {
                Ok(Environment {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    name: row.get(2)?,
                    sdk_key: row.get(3)?,
                    created_at: row.get::<_, String>(4)?.parse().unwrap(),
                    updated_at: row.get::<_, String>(5)?.parse().unwrap(),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                TogglyError::NotFound("Invalid SDK key".into())
            }
            other => TogglyError::Database(other),
        })
    }

    // -----------------------------------------------------------------------
    // Flags
    // -----------------------------------------------------------------------

    pub fn create_flag(&self, project_id: Uuid, req: &CreateFlagRequest) -> Result<Flag> {
        // Verify project exists
        self.get_project(project_id)?;

        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let tags_json = serde_json::to_string(&req.tags).unwrap();
        let flag = Flag {
            id: Uuid::new_v4(),
            project_id,
            key: req.key.clone(),
            name: req.name.clone(),
            description: req.description.clone(),
            flag_type: req.flag_type,
            tags: req.tags.clone(),
            created_at: now,
            updated_at: now,
        };
        conn.execute(
            "INSERT INTO flags (id, project_id, key, name, description, flag_type, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                flag.id.to_string(),
                flag.project_id.to_string(),
                flag.key,
                flag.name,
                flag.description,
                flag.flag_type.as_str(),
                tags_json,
                flag.created_at.to_rfc3339(),
                flag.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| {
            if let rusqlite::Error::SqliteFailure(err, _) = &e {
                if err.code == rusqlite::ErrorCode::ConstraintViolation {
                    return TogglyError::Conflict(format!(
                        "Flag with key '{}' already exists in this project",
                        req.key
                    ));
                }
            }
            TogglyError::Database(e)
        })?;

        // Create flag_environment_states for all existing environments
        let env_ids: Vec<String> = {
            let mut stmt =
                conn.prepare("SELECT id FROM environments WHERE project_id = ?1")?;
            let rows = stmt.query_map(params![project_id.to_string()], |row| {
                row.get::<_, String>(0)
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let default_value = match req.flag_type {
            FlagType::Boolean => "false",
            FlagType::String => "\"\"",
            FlagType::Number => "0",
            FlagType::Json => "null",
        };

        for env_id in &env_ids {
            let state_id = Uuid::new_v4();
            conn.execute(
                "INSERT INTO flag_environment_states (id, flag_id, environment_id, enabled, default_value, off_value, created_at, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7)",
                params![
                    state_id.to_string(),
                    flag.id.to_string(),
                    env_id,
                    default_value,
                    default_value,
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                ],
            )?;
        }

        Ok(flag)
    }

    pub fn list_flags(&self, project_id: Uuid) -> Result<Vec<Flag>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, key, name, description, flag_type, tags, created_at, updated_at FROM flags WHERE project_id = ?1 ORDER BY key",
        )?;
        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            let tags_str: String = row.get(6)?;
            Ok(Flag {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                key: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                flag_type: FlagType::from_str(&row.get::<_, String>(5)?).unwrap_or(FlagType::Boolean),
                tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                created_at: row.get::<_, String>(7)?.parse().unwrap(),
                updated_at: row.get::<_, String>(8)?.parse().unwrap(),
            })
        })?;
        let mut flags = Vec::new();
        for row in rows {
            flags.push(row?);
        }
        Ok(flags)
    }

    pub fn get_flag(&self, project_id: Uuid, key: &str) -> Result<FlagWithStates> {
        let flag = {
            let conn = self.conn.lock().unwrap();
            conn.query_row(
                "SELECT id, project_id, key, name, description, flag_type, tags, created_at, updated_at FROM flags WHERE project_id = ?1 AND key = ?2",
                params![project_id.to_string(), key],
                |row| {
                    let tags_str: String = row.get(6)?;
                    Ok(Flag {
                        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                        project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                        key: row.get(2)?,
                        name: row.get(3)?,
                        description: row.get(4)?,
                        flag_type: FlagType::from_str(&row.get::<_, String>(5)?).unwrap_or(FlagType::Boolean),
                        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                        created_at: row.get::<_, String>(7)?.parse().unwrap(),
                        updated_at: row.get::<_, String>(8)?.parse().unwrap(),
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    TogglyError::NotFound(format!("Flag '{key}' not found in project {project_id}"))
                }
                other => TogglyError::Database(other),
            })?
        };

        let environments = self.get_flag_states(flag.id)?;

        Ok(FlagWithStates { flag, environments })
    }

    fn get_flag_states(&self, flag_id: Uuid) -> Result<Vec<FlagEnvironmentState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT fes.id, fes.flag_id, fes.environment_id, e.name, fes.enabled, fes.default_value, fes.off_value, fes.created_at, fes.updated_at
             FROM flag_environment_states fes
             JOIN environments e ON e.id = fes.environment_id
             WHERE fes.flag_id = ?1
             ORDER BY e.name",
        )?;
        let rows = stmt.query_map(params![flag_id.to_string()], |row| {
            let default_val_str: String = row.get(5)?;
            let off_val_str: String = row.get(6)?;
            Ok(FlagEnvironmentState {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                flag_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                environment_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                environment_name: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                default_value: serde_json::from_str(&default_val_str).unwrap_or(serde_json::Value::Null),
                off_value: serde_json::from_str(&off_val_str).unwrap_or(serde_json::Value::Null),
                created_at: row.get::<_, String>(7)?.parse().unwrap(),
                updated_at: row.get::<_, String>(8)?.parse().unwrap(),
            })
        })?;
        let mut states = Vec::new();
        for row in rows {
            states.push(row?);
        }
        Ok(states)
    }

    pub fn update_flag(&self, project_id: Uuid, key: &str, req: &UpdateFlagRequest) -> Result<Flag> {
        let existing = self.get_flag(project_id, key)?;
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let name = req.name.as_deref().unwrap_or(&existing.flag.name);
        let description = req
            .description
            .as_deref()
            .unwrap_or(&existing.flag.description);
        let tags = req.tags.as_ref().unwrap_or(&existing.flag.tags);
        let tags_json = serde_json::to_string(tags).unwrap();
        conn.execute(
            "UPDATE flags SET name = ?1, description = ?2, tags = ?3, updated_at = ?4 WHERE project_id = ?5 AND key = ?6",
            params![
                name,
                description,
                tags_json,
                now.to_rfc3339(),
                project_id.to_string(),
                key,
            ],
        )?;
        Ok(Flag {
            id: existing.flag.id,
            project_id,
            key: key.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            flag_type: existing.flag.flag_type,
            tags: tags.clone(),
            created_at: existing.flag.created_at,
            updated_at: now,
        })
    }

    pub fn delete_flag(&self, project_id: Uuid, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM flags WHERE project_id = ?1 AND key = ?2",
            params![project_id.to_string(), key],
        )?;
        if affected == 0 {
            return Err(TogglyError::NotFound(format!(
                "Flag '{key}' not found in project {project_id}"
            )));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Flag Environment State
    // -----------------------------------------------------------------------

    pub fn update_flag_state(
        &self,
        project_id: Uuid,
        flag_key: &str,
        env_id: Uuid,
        req: &UpdateFlagStateRequest,
    ) -> Result<FlagEnvironmentState> {
        let flag_with_states = self.get_flag(project_id, flag_key)?;
        let existing = flag_with_states
            .environments
            .iter()
            .find(|s| s.environment_id == env_id)
            .ok_or_else(|| {
                TogglyError::NotFound(format!(
                    "Flag state for flag '{}' in environment {} not found",
                    flag_key, env_id
                ))
            })?;

        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let enabled = req.enabled.unwrap_or(existing.enabled);
        let default_value = req
            .default_value
            .as_ref()
            .unwrap_or(&existing.default_value);
        let off_value = req.off_value.as_ref().unwrap_or(&existing.off_value);

        conn.execute(
            "UPDATE flag_environment_states SET enabled = ?1, default_value = ?2, off_value = ?3, updated_at = ?4 WHERE id = ?5",
            params![
                enabled as i32,
                serde_json::to_string(default_value).unwrap(),
                serde_json::to_string(off_value).unwrap(),
                now.to_rfc3339(),
                existing.id.to_string(),
            ],
        )?;

        Ok(FlagEnvironmentState {
            id: existing.id,
            flag_id: existing.flag_id,
            environment_id: env_id,
            environment_name: existing.environment_name.clone(),
            enabled,
            default_value: default_value.clone(),
            off_value: off_value.clone(),
            created_at: existing.created_at,
            updated_at: now,
        })
    }

    // -----------------------------------------------------------------------
    // Evaluation queries
    // -----------------------------------------------------------------------

    pub fn eval_all_flags(
        &self,
        environment_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<EvalFlag>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT f.key, fes.enabled, fes.default_value, fes.off_value
             FROM flags f
             JOIN flag_environment_states fes ON fes.flag_id = f.id
             WHERE f.project_id = ?1 AND fes.environment_id = ?2
             ORDER BY f.key",
        )?;
        let rows = stmt.query_map(
            params![project_id.to_string(), environment_id.to_string()],
            |row| {
                let enabled = row.get::<_, i32>(1)? != 0;
                let default_val: String = row.get(2)?;
                let off_val: String = row.get(3)?;
                let value = if enabled {
                    serde_json::from_str(&default_val).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::from_str(&off_val).unwrap_or(serde_json::Value::Null)
                };
                Ok(EvalFlag {
                    key: row.get(0)?,
                    enabled,
                    value,
                })
            },
        )?;
        let mut flags = Vec::new();
        for row in rows {
            flags.push(row?);
        }
        Ok(flags)
    }

    pub fn eval_single_flag(
        &self,
        environment_id: Uuid,
        project_id: Uuid,
        flag_key: &str,
    ) -> Result<EvalFlag> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT f.key, fes.enabled, fes.default_value, fes.off_value
             FROM flags f
             JOIN flag_environment_states fes ON fes.flag_id = f.id
             WHERE f.project_id = ?1 AND fes.environment_id = ?2 AND f.key = ?3",
            params![
                project_id.to_string(),
                environment_id.to_string(),
                flag_key
            ],
            |row| {
                let enabled = row.get::<_, i32>(1)? != 0;
                let default_val: String = row.get(2)?;
                let off_val: String = row.get(3)?;
                let value = if enabled {
                    serde_json::from_str(&default_val).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::from_str(&off_val).unwrap_or(serde_json::Value::Null)
                };
                Ok(EvalFlag {
                    key: row.get(0)?,
                    enabled,
                    value,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                TogglyError::NotFound(format!("Flag '{flag_key}' not found"))
            }
            other => TogglyError::Database(other),
        })
    }
}
