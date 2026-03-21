use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub license_tier: String,
    pub created_at: DateTime<Utc>,
    pub max_agents: u32,
    pub retention_days: u32,
    pub active: bool,
}

impl Tenant {
    pub fn new(name: String, slug: String) -> Self {
        Self {
            id: Uuid::now_v7(),
            name,
            slug,
            license_tier: "oss".to_string(),
            created_at: Utc::now(),
            max_agents: 100,
            retention_days: 30,
            active: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tenant_defaults() {
        let t = Tenant::new("Acme Corp".into(), "acme".into());
        assert_eq!(t.name, "Acme Corp");
        assert_eq!(t.slug, "acme");
        assert!(t.active);
        assert_eq!(t.max_agents, 100);
    }
}
