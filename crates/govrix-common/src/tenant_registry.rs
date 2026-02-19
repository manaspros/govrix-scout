//! In-memory tenant registry with per-tenant policy engine refs.

use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::tenant::Tenant;

pub struct TenantRegistry {
    tenants: RwLock<HashMap<Uuid, Tenant>>,
}

impl TenantRegistry {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        // Default tenant always exists
        let default = Tenant {
            id: Uuid::nil(),
            name: "default".to_string(),
            slug: "default".to_string(),
            license_tier: crate::license::LicenseTier::Community,
            created_at: chrono::Utc::now(),
            max_agents: 100,
            retention_days: 30,
            active: true,
        };
        map.insert(default.id, default);
        Self {
            tenants: RwLock::new(map),
        }
    }

    pub fn create(&self, name: String) -> Tenant {
        let slug = name.to_lowercase().replace(' ', "-");
        let tenant = Tenant {
            id: Uuid::now_v7(),
            name,
            slug,
            license_tier: crate::license::LicenseTier::Community,
            created_at: chrono::Utc::now(),
            max_agents: 100,
            retention_days: 30,
            active: true,
        };
        self.tenants
            .write()
            .unwrap()
            .insert(tenant.id, tenant.clone());
        tenant
    }

    pub fn list(&self) -> Vec<Tenant> {
        let mut v: Vec<_> = self.tenants.read().unwrap().values().cloned().collect();
        v.sort_by_key(|t| t.created_at);
        v
    }

    pub fn get(&self, id: Uuid) -> Option<Tenant> {
        self.tenants.read().unwrap().get(&id).cloned()
    }

    pub fn count(&self) -> usize {
        self.tenants.read().unwrap().len()
    }
}

impl Default for TenantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tenant_exists() {
        let r = TenantRegistry::new();
        assert_eq!(r.count(), 1);
        let tenants = r.list();
        assert_eq!(tenants[0].name, "default");
    }

    #[test]
    fn create_tenant_increments_count() {
        let r = TenantRegistry::new();
        r.create("acme".to_string());
        assert_eq!(r.count(), 2);
    }

    #[test]
    fn list_returns_all_tenants() {
        let r = TenantRegistry::new();
        r.create("tenant-a".to_string());
        r.create("tenant-b".to_string());
        let list = r.list();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn get_existing_tenant() {
        let r = TenantRegistry::new();
        let t = r.create("test".to_string());
        assert!(r.get(t.id).is_some());
    }

    #[test]
    fn get_missing_tenant_returns_none() {
        let r = TenantRegistry::new();
        assert!(r.get(Uuid::now_v7()).is_none());
    }
}
