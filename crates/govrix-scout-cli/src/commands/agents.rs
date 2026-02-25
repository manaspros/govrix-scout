//! `Govrix Scout agents list` — list registered agents.

use comfy_table::{Cell, Table};

use crate::client::ApiClient;

/// Run the `agents list` command.
///
/// Fetches `GET /api/v1/agents` and renders a table of agents. Prints raw JSON
/// when `json` is true.
pub async fn run_list(client: &ApiClient, json: bool) -> anyhow::Result<()> {
    let resp: serde_json::Value = client.get("/api/v1/agents").await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    let empty = vec![];
    let agents = resp
        .get("data")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    if agents.is_empty() {
        println!("No agents found.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["ID", "Name", "Status", "Requests"]);

    for agent in agents {
        let id = agent.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let name = agent.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        let status = agent.get("status").and_then(|v| v.as_str()).unwrap_or("-");
        let requests = agent
            .get("total_requests")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        table.add_row(vec![
            Cell::new(id),
            Cell::new(name),
            Cell::new(status),
            Cell::new(requests),
        ]);
    }

    println!("{table}");
    Ok(())
}
