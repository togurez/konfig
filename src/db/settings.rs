use sqlx::PgPool;

use crate::{
    error::AppError,
    models::setting::{CreateSettingRequest, ListSettingsQuery, Setting, UpdateSettingRequest},
};

pub async fn insert_setting(pool: &PgPool, req: &CreateSettingRequest) -> Result<Setting, AppError> {
    sqlx::query_as::<_, Setting>(
        r#"
        INSERT INTO settings (key, setting_type, value, description, is_active)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, key, setting_type, value, description, is_active, created_at, updated_at
        "#,
    )
    .bind(&req.key)
    .bind(&req.setting_type)
    .bind(&req.value)
    .bind(&req.description)
    .bind(req.is_active.unwrap_or(true))
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("settings_key_key") {
                return AppError::Conflict(format!("Setting '{}' already exists", req.key));
            }
        }
        AppError::DatabaseError(e)
    })
}

pub async fn get_setting_by_key(pool: &PgPool, key: &str) -> Result<Option<Setting>, AppError> {
    sqlx::query_as::<_, Setting>(
        "SELECT id, key, setting_type, value, description, is_active, created_at, updated_at
         FROM settings WHERE key = $1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(AppError::DatabaseError)
}

pub async fn list_settings(
    pool: &PgPool,
    filters: &ListSettingsQuery,
) -> Result<Vec<Setting>, AppError> {
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
        "SELECT id, key, setting_type, value, description, is_active, created_at, updated_at
         FROM settings WHERE 1=1",
    );

    if let Some(setting_type) = &filters.setting_type {
        qb.push(" AND setting_type = ").push_bind(setting_type);
    }
    if let Some(active) = filters.active {
        qb.push(" AND is_active = ").push_bind(active);
    }

    qb.push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(per_page as i64)
        .push(" OFFSET ")
        .push_bind(offset);

    qb.build_query_as::<Setting>()
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)
}

pub async fn update_setting(
    pool: &PgPool,
    key: &str,
    req: &UpdateSettingRequest,
) -> Result<Option<Setting>, AppError> {
    let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("UPDATE settings SET updated_at = NOW()");

    if let Some(setting_type) = &req.setting_type {
        qb.push(", setting_type = ").push_bind(setting_type);
    }
    if let Some(value) = &req.value {
        qb.push(", value = ").push_bind(value);
    }
    if req.description.is_some() {
        qb.push(", description = ").push_bind(&req.description);
    }
    if let Some(is_active) = req.is_active {
        qb.push(", is_active = ").push_bind(is_active);
    }

    qb.push(" WHERE key = ").push_bind(key).push(
        " RETURNING id, key, setting_type, value, description, is_active, created_at, updated_at",
    );

    qb.build_query_as::<Setting>()
        .fetch_optional(pool)
        .await
        .map_err(AppError::DatabaseError)
}

pub async fn delete_setting(pool: &PgPool, key: &str) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM settings WHERE key = $1")
        .bind(key)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::setting::{CreateSettingRequest, SettingType, UpdateSettingRequest};
    use serde_json::json;

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_and_fetch(pool: PgPool) {
        let req = CreateSettingRequest {
            key: "feature.dark_mode".to_string(),
            setting_type: SettingType::FeatureFlag,
            value: json!({ "enabled": true }),
            description: Some("Dark mode toggle".to_string()),
            is_active: Some(true),
        };

        let setting = insert_setting(&pool, &req).await.unwrap();
        assert_eq!(setting.key, "feature.dark_mode");
        assert!(setting.is_active);

        let fetched = get_setting_by_key(&pool, "feature.dark_mode")
            .await
            .unwrap()
            .expect("setting should exist");
        assert_eq!(fetched.id, setting.id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_duplicate_key_returns_conflict(pool: PgPool) {
        let req = CreateSettingRequest {
            key: "app.limit".to_string(),
            setting_type: SettingType::Limit,
            value: json!(100),
            description: None,
            is_active: None,
        };
        insert_setting(&pool, &req).await.unwrap();

        let result = insert_setting(&pool, &req).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_missing_key_returns_none(pool: PgPool) {
        let result = get_setting_by_key(&pool, "nonexistent.key")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_full_lifecycle(pool: PgPool) {
        // Create
        let req = CreateSettingRequest {
            key: "ui.theme".to_string(),
            setting_type: SettingType::Appearance,
            value: json!("dark"),
            description: None,
            is_active: Some(true),
        };
        let created = insert_setting(&pool, &req).await.unwrap();

        // Update
        let update = UpdateSettingRequest {
            setting_type: None,
            value: Some(json!("light")),
            description: Some("Theme setting".to_string()),
            is_active: Some(false),
        };
        let updated = update_setting(&pool, "ui.theme", &update)
            .await
            .unwrap()
            .expect("setting should exist");
        assert_eq!(updated.value, json!("light"));
        assert!(!updated.is_active);
        assert!(updated.updated_at > created.updated_at || updated.updated_at == created.updated_at);

        // Delete
        let deleted = delete_setting(&pool, "ui.theme").await.unwrap();
        assert!(deleted);

        // Confirm gone
        let gone = get_setting_by_key(&pool, "ui.theme").await.unwrap();
        assert!(gone.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_list_with_filters(pool: PgPool) {
        for (key, st, active) in [
            ("a.flag", SettingType::FeatureFlag, true),
            ("b.flag", SettingType::FeatureFlag, false),
            ("c.limit", SettingType::Limit, true),
        ] {
            insert_setting(
                &pool,
                &CreateSettingRequest {
                    key: key.to_string(),
                    setting_type: st,
                    value: json!(1),
                    description: None,
                    is_active: Some(active),
                },
            )
            .await
            .unwrap();
        }

        let all = list_settings(&pool, &ListSettingsQuery {
            setting_type: None,
            active: None,
            page: None,
            per_page: None,
        })
        .await
        .unwrap();
        assert_eq!(all.len(), 3);

        let active_only = list_settings(&pool, &ListSettingsQuery {
            setting_type: None,
            active: Some(true),
            page: None,
            per_page: None,
        })
        .await
        .unwrap();
        assert_eq!(active_only.len(), 2);

        let flags = list_settings(&pool, &ListSettingsQuery {
            setting_type: Some(SettingType::FeatureFlag),
            active: None,
            page: None,
            per_page: None,
        })
        .await
        .unwrap();
        assert_eq!(flags.len(), 2);
    }
}
