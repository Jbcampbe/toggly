mod app;
mod client;
mod ui;

fn main() -> anyhow::Result<()> {
    let server_url =
        std::env::var("TOGGLY_SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let api_key =
        std::env::var("TOGGLY_ADMIN_API_KEY").expect("TOGGLY_ADMIN_API_KEY must be set");

    let client = client::ApiClient::new(&server_url, &api_key);
    app::run(client)?;
    Ok(())
}
