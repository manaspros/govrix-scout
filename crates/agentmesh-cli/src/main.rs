//! AgentMesh CLI — `agentmesh-ctl` command-line tool.
//!
//! Provides commands for interacting with a running AgentMesh proxy:
//! - `status`       — check proxy health and statistics
//! - `agents list`  — list registered agents
//! - `agents get`   — get details for a specific agent
//! - `events list`  — list recent events
//! - `events tail`  — stream live events (SSE)
//! - `costs`        — show cost summary
//! - `config get`   — show current config
//! - `config set`   — update a config value

mod http;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// AgentMesh control plane CLI
#[derive(Debug, Parser)]
#[command(
    name = "agentmesh-ctl",
    version = env!("CARGO_PKG_VERSION"),
    about = "AgentMesh control plane CLI — interact with a running proxy",
    long_about = None,
)]
struct Cli {
    /// AgentMesh API base URL
    #[arg(
        long,
        env = "AGENTMESH_API_URL",
        default_value = "http://localhost:4001"
    )]
    api_url: String,

    /// Output format
    #[arg(long, default_value = "table", value_parser = ["table", "json", "yaml"])]
    output: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Check proxy health and show statistics
    Status,

    /// Manage registered agents
    #[command(subcommand)]
    Agents(AgentsCommands),

    /// Query intercepted events
    #[command(subcommand)]
    Events(EventsCommands),

    /// Show cost analysis
    Costs {
        /// Number of days to include in the summary
        #[arg(long, default_value = "7")]
        days: u32,
    },

    /// Manage proxy configuration
    #[command(subcommand)]
    Config(ConfigCommands),
}

#[derive(Debug, Subcommand)]
enum AgentsCommands {
    /// List all registered agents
    List {
        #[arg(long, default_value = "50")]
        limit: u32,
    },
    /// Get details for a specific agent
    Get {
        /// Agent ID
        id: String,
    },
    /// Retire an agent (stop accepting new sessions)
    Retire {
        /// Agent ID
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum EventsCommands {
    /// List recent events
    List {
        #[arg(long, default_value = "50")]
        limit: u32,
        /// Filter by agent ID
        #[arg(long)]
        agent: Option<String>,
    },
    /// Get a specific event by ID
    Get {
        /// Event ID (UUID)
        id: String,
    },
    /// Stream live events as they arrive
    Tail {
        /// Filter by agent ID
        #[arg(long)]
        agent: Option<String>,
    },
    /// Show all events for a compliance session
    Session {
        /// Session ID (UUID)
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Get,
    /// Update a configuration value
    Set {
        /// Config key (e.g. "proxy.fail_open")
        key: String,
        /// New value
        value: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let client = http::ApiClient::new(&cli.api_url);

    let result: Result<serde_json::Value> = match cli.command {
        Commands::Status => client.get("/health").await,
        Commands::Agents(AgentsCommands::List { limit }) => {
            client.get(&format!("/api/v1/agents?limit={limit}")).await
        }
        Commands::Agents(AgentsCommands::Get { id }) => {
            client.get(&format!("/api/v1/agents/{id}")).await
        }
        Commands::Agents(AgentsCommands::Retire { id }) => {
            client.post(&format!("/api/v1/agents/{id}/retire")).await
        }
        Commands::Events(EventsCommands::List { limit, agent }) => {
            let agent_filter = agent.map(|a| format!("&agent_id={a}")).unwrap_or_default();
            client
                .get(&format!("/api/v1/events?limit={limit}{agent_filter}"))
                .await
        }
        Commands::Events(EventsCommands::Get { id }) => {
            client.get(&format!("/api/v1/events/{id}")).await
        }
        Commands::Events(EventsCommands::Tail { agent: _ }) => {
            anyhow::bail!("events tail (SSE streaming) is not yet supported in the CLI");
        }
        Commands::Events(EventsCommands::Session { id }) => {
            client.get(&format!("/api/v1/events/sessions/{id}")).await
        }
        Commands::Costs { days } => {
            client
                .get(&format!("/api/v1/costs/summary?days={days}"))
                .await
        }
        Commands::Config(ConfigCommands::Get) => client.get("/api/v1/config").await,
        Commands::Config(ConfigCommands::Set { key, value }) => {
            let body = serde_json::json!({"key": key, "value": value});
            client.put_json("/api/v1/config", &body).await
        }
    };

    match result {
        Ok(value) => output::print_output(&cli.output, &value)?,
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    }

    Ok(())
}
