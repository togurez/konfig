use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Created,
    Updated,
    Activated,
    Deactivated,
    Deleted,
    BulkActivated,
    BulkDeactivated,
    BulkDeleted,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::Created => "created",
            AuditAction::Updated => "updated",
            AuditAction::Activated => "activated",
            AuditAction::Deactivated => "deactivated",
            AuditAction::Deleted => "deleted",
            AuditAction::BulkActivated => "bulk_activated",
            AuditAction::BulkDeactivated => "bulk_deactivated",
            AuditAction::BulkDeleted => "bulk_deleted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Revision {
    pub id: Uuid,
    pub setting_key: String,
    pub action: String,
    pub previous_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub changed_by: String,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub setting_key: Option<String>,
    pub action: Option<String>,
    pub changed_by: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct AuditPage {
    pub data: Vec<Revision>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}
