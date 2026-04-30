use chrono::Utc;
use sqlx::PgPool;

use crate::{
    error::AppError,
    models::{
        revision::AuditAction,
        search::{BulkAction, BulkFilter, BulkRequest, BulkResponse, SearchPage, SearchQuery},
        setting::Setting,
    },
};

use super::{revisions, settings::SETTING_COLUMNS};

pub async fn search_settings(
    pool: &PgPool,
    query: &SearchQuery,
    user_id: &str,
) -> Result<SearchPage, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let workspace_total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM settings WHERE created_by = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .map_err(AppError::DatabaseError)?;

    let mut count_qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT COUNT(*) FROM settings WHERE created_by = ");
    count_qb.push_bind(user_id);
    push_search_filters(&mut count_qb, query);

    let total = count_qb
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(AppError::DatabaseError)?;

    let mut data_qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new(format!("SELECT {SETTING_COLUMNS} FROM settings WHERE created_by = "));
    data_qb.push_bind(user_id);
    push_search_filters(&mut data_qb, query);
    push_search_order(&mut data_qb, query);
    data_qb
        .push(" LIMIT ")
        .push_bind(per_page as i64)
        .push(" OFFSET ")
        .push_bind(offset);

    let data = data_qb
        .build_query_as::<Setting>()
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(SearchPage {
        data,
        total,
        hidden: workspace_total - total,
        page,
        per_page,
    })
}

pub async fn bulk_action(
    pool: &PgPool,
    req: &BulkRequest,
    user_id: &str,
) -> Result<BulkResponse, AppError> {
    let mut count_qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT COUNT(*) FROM settings WHERE created_by = ");
    count_qb.push_bind(user_id);
    push_bulk_filters(&mut count_qb, &req.filter);

    let matched: i64 = count_qb
        .build_query_scalar()
        .fetch_one(pool)
        .await
        .map_err(AppError::DatabaseError)?;

    let mut tx = pool.begin().await.map_err(AppError::DatabaseError)?;

    let (action_str, audit_action, rows_affected) = match req.action {
        BulkAction::Activate => {
            let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "UPDATE settings SET is_active = true, updated_at = NOW(), updated_by = ",
            );
            qb.push_bind(user_id)
                .push(" WHERE is_active = false AND created_by = ")
                .push_bind(user_id);
            push_bulk_filters(&mut qb, &req.filter);
            let r = qb
                .build()
                .execute(&mut *tx)
                .await
                .map_err(AppError::DatabaseError)?;
            ("activate", AuditAction::BulkActivated, r.rows_affected() as i64)
        }
        BulkAction::Deactivate => {
            let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "UPDATE settings SET is_active = false, updated_at = NOW(), updated_by = ",
            );
            qb.push_bind(user_id)
                .push(" WHERE is_active = true AND created_by = ")
                .push_bind(user_id);
            push_bulk_filters(&mut qb, &req.filter);
            let r = qb
                .build()
                .execute(&mut *tx)
                .await
                .map_err(AppError::DatabaseError)?;
            ("deactivate", AuditAction::BulkDeactivated, r.rows_affected() as i64)
        }
        BulkAction::Delete => {
            let mut qb: sqlx::QueryBuilder<sqlx::Postgres> =
                sqlx::QueryBuilder::new("DELETE FROM settings WHERE created_by = ");
            qb.push_bind(user_id);
            push_bulk_filters(&mut qb, &req.filter);
            let r = qb
                .build()
                .execute(&mut *tx)
                .await
                .map_err(AppError::DatabaseError)?;
            ("delete", AuditAction::BulkDeleted, r.rows_affected() as i64)
        }
    };

    let filter_json = serde_json::to_value(&req.filter).ok();
    revisions::insert_revision(&mut tx, "*", audit_action, None, filter_json.as_ref(), user_id)
        .await?;

    tx.commit().await.map_err(AppError::DatabaseError)?;

    Ok(BulkResponse {
        action: action_str.to_string(),
        matched,
        affected: rows_affected,
    })
}

fn push_search_filters(qb: &mut sqlx::QueryBuilder<sqlx::Postgres>, query: &SearchQuery) {
    if let Some(q) = &query.q {
        let like = format!("%{}%", q);
        qb.push(" AND (key ILIKE ")
            .push_bind(like.clone())
            .push(" OR description ILIKE ")
            .push_bind(like.clone())
            .push(" OR value::text ILIKE ")
            .push_bind(like)
            .push(")");
    }
    if let Some(st) = &query.setting_type {
        qb.push(" AND setting_type = ").push_bind(st.clone());
    }
    if let Some(active) = query.active {
        qb.push(" AND is_active = ").push_bind(active);
    }
    if let Some(within) = &query.updated_within {
        if let Some(since) = parse_updated_within(within) {
            qb.push(" AND updated_at > ").push_bind(since);
        }
    }
    if let Some(key_glob) = &query.key {
        qb.push(" AND key ILIKE ").push_bind(glob_to_like(key_glob));
    }
}

fn push_search_order(qb: &mut sqlx::QueryBuilder<sqlx::Postgres>, query: &SearchQuery) {
    if let Some(q) = &query.q {
        qb.push(" ORDER BY CASE")
            .push(" WHEN key = ")
            .push_bind(q.clone())
            .push(" THEN 0 WHEN key ILIKE ")
            .push_bind(format!("{}%", q))
            .push(" THEN 1 WHEN key ILIKE ")
            .push_bind(format!("%{}%", q))
            .push(" THEN 2 WHEN description ILIKE ")
            .push_bind(format!("%{}%", q))
            .push(" THEN 3 ELSE 4 END, updated_at DESC");
    } else {
        qb.push(" ORDER BY updated_at DESC");
    }
}

fn push_bulk_filters(qb: &mut sqlx::QueryBuilder<sqlx::Postgres>, filter: &BulkFilter) {
    if let Some(q) = &filter.q {
        let like = format!("%{}%", q);
        qb.push(" AND (key ILIKE ")
            .push_bind(like.clone())
            .push(" OR description ILIKE ")
            .push_bind(like.clone())
            .push(" OR value::text ILIKE ")
            .push_bind(like)
            .push(")");
    }
    if let Some(st) = &filter.setting_type {
        qb.push(" AND setting_type = ").push_bind(st.clone());
    }
    if let Some(active) = filter.active {
        qb.push(" AND is_active = ").push_bind(active);
    }
}

fn parse_updated_within(s: &str) -> Option<chrono::DateTime<Utc>> {
    let duration = match s {
        "1h" => chrono::Duration::hours(1),
        "24h" => chrono::Duration::hours(24),
        "7d" => chrono::Duration::days(7),
        "30d" => chrono::Duration::days(30),
        _ => return None,
    };
    Some(Utc::now() - duration)
}

fn glob_to_like(pattern: &str) -> String {
    pattern.replace('*', "%").replace('?', "_")
}
