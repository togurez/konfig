use sqlx::PgPool;

use crate::{
    error::AppError,
    models::{
        revision::AuditAction,
        setting::{CreateSettingRequest, ListSettingsQuery, Setting, UpdateSettingRequest},
    },
};

use super::revisions;

pub(crate) const SETTING_COLUMNS: &str =
    "id, key, setting_type, value, description, is_active, created_at, updated_at, created_by, updated_by";

pub async fn insert_setting(
    pool: &PgPool,
    req: &CreateSettingRequest,
    user_id: &str,
) -> Result<Setting, AppError> {
    let mut tx = pool.begin().await.map_err(AppError::DatabaseError)?;

    let setting = sqlx::query_as::<_, Setting>(&format!(
        "INSERT INTO settings (key, setting_type, value, description, is_active, created_by, updated_by)
         VALUES ($1, $2, $3, $4, $5, $6, $6)
         RETURNING {SETTING_COLUMNS}"
    ))
    .bind(&req.key)
    .bind(&req.setting_type)
    .bind(&req.value)
    .bind(&req.description)
    .bind(req.is_active.unwrap_or(true))
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("settings_key_key") {
                return AppError::Conflict(format!("Setting '{}' already exists", req.key));
            }
        }
        AppError::DatabaseError(e)
    })?;

    revisions::insert_revision(
        &mut tx,
        &setting.key,
        AuditAction::Created,
        None,
        Some(&setting.value),
        user_id,
    )
    .await?;

    tx.commit().await.map_err(AppError::DatabaseError)?;
    Ok(setting)
}

pub async fn get_setting_by_key(pool: &PgPool, key: &str) -> Result<Option<Setting>, AppError> {
    sqlx::query_as::<_, Setting>(&format!(
        "SELECT {SETTING_COLUMNS} FROM settings WHERE key = $1"
    ))
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

    let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(format!(
        "SELECT {SETTING_COLUMNS} FROM settings WHERE 1=1"
    ));

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

pub async fn count_settings(pool: &PgPool, filters: &ListSettingsQuery) -> Result<i64, AppError> {
    let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT COUNT(*) FROM settings WHERE 1=1");

    if let Some(setting_type) = &filters.setting_type {
        qb.push(" AND setting_type = ").push_bind(setting_type);
    }
    if let Some(active) = filters.active {
        qb.push(" AND is_active = ").push_bind(active);
    }

    qb.build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(AppError::DatabaseError)
}

pub async fn update_setting(
    pool: &PgPool,
    key: &str,
    req: &UpdateSettingRequest,
    user_id: &str,
) -> Result<Option<Setting>, AppError> {
    let mut tx = pool.begin().await.map_err(AppError::DatabaseError)?;

    let current = sqlx::query_as::<_, Setting>(&format!(
        "SELECT {SETTING_COLUMNS} FROM settings WHERE key = $1 FOR UPDATE"
    ))
    .bind(key)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    let Some(current) = current else {
        return Ok(None);
    };

    let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("UPDATE settings SET updated_at = NOW(), updated_by = ");
    qb.push_bind(user_id);

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

    qb.push(" WHERE key = ")
        .push_bind(key)
        .push(format!(" RETURNING {SETTING_COLUMNS}"));

    let updated = qb
        .build_query_as::<Setting>()
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

    let Some(updated) = updated else {
        return Ok(None);
    };

    let action = match req.is_active {
        Some(true) if !current.is_active => AuditAction::Activated,
        Some(false) if current.is_active => AuditAction::Deactivated,
        _ => AuditAction::Updated,
    };

    revisions::insert_revision(
        &mut tx,
        key,
        action,
        Some(&current.value),
        Some(&updated.value),
        user_id,
    )
    .await?;

    tx.commit().await.map_err(AppError::DatabaseError)?;
    Ok(Some(updated))
}

pub async fn delete_setting(pool: &PgPool, key: &str, user_id: &str) -> Result<bool, AppError> {
    let mut tx = pool.begin().await.map_err(AppError::DatabaseError)?;

    let current = sqlx::query_as::<_, Setting>(&format!(
        "SELECT {SETTING_COLUMNS} FROM settings WHERE key = $1"
    ))
    .bind(key)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::DatabaseError)?;

    let result = sqlx::query("DELETE FROM settings WHERE key = $1")
        .bind(key)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

    if result.rows_affected() > 0 {
        if let Some(current) = current {
            revisions::insert_revision(
                &mut tx,
                key,
                AuditAction::Deleted,
                Some(&current.value),
                None,
                user_id,
            )
            .await?;
        }
        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::setting::{CreateSettingRequest, SettingType, UpdateSettingRequest};
    use serde_json::json;

    const TEST_USER: &str = "auth0|test-user";

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_and_fetch(pool: PgPool) {
        let req = CreateSettingRequest {
            key: "feature.dark_mode".to_string(),
            setting_type: SettingType::FeatureFlag,
            value: json!({ "enabled": true }),
            description: Some("Dark mode toggle".to_string()),
            is_active: Some(true),
        };

        let setting = insert_setting(&pool, &req, TEST_USER).await.unwrap();
        assert_eq!(setting.key, "feature.dark_mode");
        assert!(setting.is_active);
        assert_eq!(setting.created_by.as_deref(), Some(TEST_USER));
        assert_eq!(setting.updated_by.as_deref(), Some(TEST_USER));

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
        insert_setting(&pool, &req, TEST_USER).await.unwrap();

        let result = insert_setting(&pool, &req, TEST_USER).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_missing_key_returns_none(pool: PgPool) {
        let result = get_setting_by_key(&pool, "nonexistent.key").await.unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_full_lifecycle(pool: PgPool) {
        let req = CreateSettingRequest {
            key: "ui.theme".to_string(),
            setting_type: SettingType::Appearance,
            value: json!("dark"),
            description: None,
            is_active: Some(true),
        };
        let created = insert_setting(&pool, &req, TEST_USER).await.unwrap();

        let updater = "auth0|another-user";
        let update = UpdateSettingRequest {
            setting_type: None,
            value: Some(json!("light")),
            description: Some("Theme setting".to_string()),
            is_active: Some(false),
        };
        let updated = update_setting(&pool, "ui.theme", &update, updater)
            .await
            .unwrap()
            .expect("setting should exist");
        assert_eq!(updated.value, json!("light"));
        assert!(!updated.is_active);
        assert_eq!(updated.created_by.as_deref(), Some(TEST_USER));
        assert_eq!(updated.updated_by.as_deref(), Some(updater));
        assert!(updated.updated_at >= created.updated_at);

        let deleted = delete_setting(&pool, "ui.theme", TEST_USER).await.unwrap();
        assert!(deleted);

        let gone = get_setting_by_key(&pool, "ui.theme").await.unwrap();
        assert!(gone.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_count_with_filters(pool: PgPool) {
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
                TEST_USER,
            )
            .await
            .unwrap();
        }

        let total = count_settings(
            &pool,
            &ListSettingsQuery { setting_type: None, active: None, page: None, per_page: None },
        )
        .await
        .unwrap();
        assert_eq!(total, 3);

        let active_total = count_settings(
            &pool,
            &ListSettingsQuery { setting_type: None, active: Some(true), page: None, per_page: None },
        )
        .await
        .unwrap();
        assert_eq!(active_total, 2);

        let flag_total = count_settings(
            &pool,
            &ListSettingsQuery {
                setting_type: Some(SettingType::FeatureFlag),
                active: None,
                page: None,
                per_page: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(flag_total, 2);
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
                TEST_USER,
            )
            .await
            .unwrap();
        }

        let all = list_settings(
            &pool,
            &ListSettingsQuery { setting_type: None, active: None, page: None, per_page: None },
        )
        .await
        .unwrap();
        assert_eq!(all.len(), 3);

        let active_only = list_settings(
            &pool,
            &ListSettingsQuery { setting_type: None, active: Some(true), page: None, per_page: None },
        )
        .await
        .unwrap();
        assert_eq!(active_only.len(), 2);

        let flags = list_settings(
            &pool,
            &ListSettingsQuery {
                setting_type: Some(SettingType::FeatureFlag),
                active: None,
                page: None,
                per_page: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(flags.len(), 2);
    }
}
