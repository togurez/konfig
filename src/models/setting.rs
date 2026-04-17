use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Setting {
    pub id: Uuid,
    pub key: String,
    pub setting_type: SettingType,
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SettingType {
    FeatureFlag,
    Limit,
    Appearance,
    Integration,
    Custom,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSettingRequest {
    #[validate(length(min = 1, max = 255))]
    pub key: String,
    pub setting_type: SettingType,
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSettingRequest {
    pub setting_type: Option<SettingType>,
    pub value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListSettingsQuery {
    #[serde(rename = "type")]
    pub setting_type: Option<SettingType>,
    pub active: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}
