//! `Govrix Scout status` — show proxy health and version.

use comfy_table::{Cell, Table};

use crate::client::ApiClient;

/// Run the `status` command.
///
/// Fetches `GET /health` and prints a summary table (or raw JSON when `json` is
/// true).
pub async fn run(client: &ApiClient, json: bool) -> anyhow::Result<()> {
    let resp: serde_json::Value = client.get("/health").await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["Field", "Value"]);

    let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
    let version = resp.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");

    table.add_row(vec![Cell::new("Status"), Cell::new(status)]);
    table.add_row(vec![Cell::new("Version"), Cell::new(version)]);

    println!("{table}");
    Ok(())
}
