use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use toggly_core::models::*;

pub struct ApiClient {
    base_url: String,
    api_key: String,
    http: Client,
}

impl ApiClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            http: Client::new(),
        }
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("API error {status}: {body}");
        }
        Ok(resp.json()?)
    }

    fn post<T: DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> anyhow::Result<T> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(body)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("API error {status}: {body}");
        }
        Ok(resp.json()?)
    }

    fn put<T: DeserializeOwned>(&self, path: &str, body: &impl serde::Serialize) -> anyhow::Result<T> {
        let resp = self
            .http
            .put(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(body)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("API error {status}: {body}");
        }
        Ok(resp.json()?)
    }

    fn delete(&self, path: &str) -> anyhow::Result<()> {
        let resp = self
            .http
            .delete(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("API error {status}: {body}");
        }
        Ok(())
    }

    // Projects

    pub fn list_projects(&self) -> anyhow::Result<Vec<Project>> {
        self.get("/api/v1/projects")
    }

    pub fn create_project(&self, req: &CreateProjectRequest) -> anyhow::Result<Project> {
        self.post("/api/v1/projects", req)
    }

    pub fn delete_project(&self, id: Uuid) -> anyhow::Result<()> {
        self.delete(&format!("/api/v1/projects/{id}"))
    }

    // Environments

    pub fn list_environments(&self, project_id: Uuid) -> anyhow::Result<Vec<Environment>> {
        self.get(&format!("/api/v1/projects/{project_id}/environments"))
    }

    pub fn create_environment(
        &self,
        project_id: Uuid,
        req: &CreateEnvironmentRequest,
    ) -> anyhow::Result<Environment> {
        self.post(
            &format!("/api/v1/projects/{project_id}/environments"),
            req,
        )
    }

    pub fn delete_environment(&self, project_id: Uuid, env_id: Uuid) -> anyhow::Result<()> {
        self.delete(&format!(
            "/api/v1/projects/{project_id}/environments/{env_id}"
        ))
    }

    // Flags

    pub fn list_flags(&self, project_id: Uuid) -> anyhow::Result<Vec<Flag>> {
        self.get(&format!("/api/v1/projects/{project_id}/flags"))
    }

    pub fn get_flag(&self, project_id: Uuid, key: &str) -> anyhow::Result<FlagWithStates> {
        self.get(&format!("/api/v1/projects/{project_id}/flags/{key}"))
    }

    pub fn create_flag(&self, project_id: Uuid, req: &CreateFlagRequest) -> anyhow::Result<Flag> {
        self.post(&format!("/api/v1/projects/{project_id}/flags"), req)
    }

    pub fn delete_flag(&self, project_id: Uuid, key: &str) -> anyhow::Result<()> {
        self.delete(&format!("/api/v1/projects/{project_id}/flags/{key}"))
    }

    // Flag state

    pub fn update_flag_state(
        &self,
        project_id: Uuid,
        flag_key: &str,
        env_id: Uuid,
        req: &UpdateFlagStateRequest,
    ) -> anyhow::Result<FlagEnvironmentState> {
        self.put(
            &format!("/api/v1/projects/{project_id}/flags/{flag_key}/environments/{env_id}"),
            req,
        )
    }
}
