//! `Govrix Scout events list` — list recent proxy events.

use comfy_table::{Cell, Table};

use crate::client::ApiClient;

/// Run the `events list` command.
///
/// Fetches `GET /api/v1/events?limit=N` and renders a table. Prints raw JSON
/// when `json` is true.
pub async fn run_list(client: &ApiClient, limit: u32, json: bool) -> anyhow::Result<()> {
    let path = format!("/api/v1/events?limit={limit}");
    let resp: serde_json::Value = client.get(&path).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
        return Ok(());
    }

    let empty = vec![];
    let events = resp
        .get("data")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    if events.is_empty() {
        println!("No events found.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["ID", "Agent ID", "Direction", "Provider", "Model", "Tokens"]);

    for event in events {
        let id = event.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let agent_id = event.get("agent_id").and_then(|v| v.as_str()).unwrap_or("-");
        let direction = event.get("direction").and_then(|v| v.as_str()).unwrap_or("-");
        let provider = event.get("provider").and_then(|v| v.as_str()).unwrap_or("-");
        let model = event.get("model").and_then(|v| v.as_str()).unwrap_or("-");
        let tokens = event
            .get("total_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        table.add_row(vec![
            Cell::new(id),
            Cell::new(agent_id),
            Cell::new(direction),
            Cell::new(provider),
            Cell::new(model),
            Cell::new(tokens),
        ]);
    }

    println!("{table}");
    Ok(())
}
