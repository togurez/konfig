use serde::{Deserialize, Serialize};

use crate::models::setting::{Setting, SettingType};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    #[serde(rename = "type")]
    pub setting_type: Option<SettingType>,
    pub active: Option<bool>,
    pub updated_within: Option<String>,
    pub key: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct SearchPage {
    pub data: Vec<Setting>,
    pub total: i64,
    pub hidden: i64,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BulkFilter {
    pub q: Option<String>,
    #[serde(rename = "type")]
    pub setting_type: Option<SettingType>,
    pub active: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkAction {
    Activate,
    Deactivate,
    Delete,
}

#[derive(Debug, Deserialize)]
pub struct BulkRequest {
    pub filter: BulkFilter,
    pub action: BulkAction,
}

#[derive(Debug, Serialize)]
pub struct BulkResponse {
    pub action: String,
    pub matched: i64,
    pub affected: i64,
}
