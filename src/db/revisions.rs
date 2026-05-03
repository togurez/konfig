use sqlx::PgPool;

use crate::{
    error::AppError,
    models::revision::{AuditAction, AuditPage, AuditQuery, Revision},
};

pub(crate) async fn insert_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    setting_key: &str,
    action: AuditAction,
    previous_value: Option<&serde_json::Value>,
    new_value: Option<&serde_json::Value>,
    changed_by: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO setting_revisions (setting_key, action, previous_value, new_value, changed_by)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(setting_key)
    .bind(action.as_str())
    .bind(previous_value)
    .bind(new_value)
    .bind(changed_by)
    .execute(&mut **tx)
    .await
    .map_err(AppError::DatabaseError)?;
    Ok(())
}

pub async fn list_history(
    pool: &PgPool,
    setting_key: &str,
    page: u32,
    per_page: u32,
) -> Result<(Vec<Revision>, i64), AppError> {
    let offset = ((page - 1) * per_page) as i64;

    let (rows, total) = tokio::try_join!(
        sqlx::query_as::<_, Revision>(
            "SELECT id, setting_key, action, previous_value, new_value, changed_by, changed_at
             FROM setting_revisions
             WHERE setting_key = $1
             ORDER BY changed_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(setting_key)
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(pool),
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM setting_revisions WHERE setting_key = $1",
        )
        .bind(setting_key)
        .fetch_one(pool),
    )
    .map_err(AppError::DatabaseError)?;

    Ok((rows, total))
}

pub async fn list_audit(pool: &PgPool, query: &AuditQuery) -> Result<AuditPage, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    let mut count_qb: sqlx::QueryBuilder<sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT COUNT(*) FROM setting_revisions WHERE 1=1");
    push_audit_filters(&mut count_qb, query);

    let mut data_qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
        "SELECT id, setting_key, action, previous_value, new_value, changed_by, changed_at
         FROM setting_revisions WHERE 1=1",
    );
    push_audit_filters(&mut data_qb, query);
    data_qb
        .push(" ORDER BY changed_at DESC LIMIT ")
        .push_bind(per_page as i64)
        .push(" OFFSET ")
        .push_bind(offset);

    let total = count_qb
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .map_err(AppError::DatabaseError)?;

    let data = data_qb
        .build_query_as::<Revision>()
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(AuditPage { data, total, page, per_page })
}

fn push_audit_filters(qb: &mut sqlx::QueryBuilder<sqlx::Postgres>, query: &AuditQuery) {
    if let Some(key) = &query.setting_key {
        qb.push(" AND setting_key = ").push_bind(key.clone());
    }
    if let Some(action) = &query.action {
        qb.push(" AND action = ").push_bind(action.clone());
    }
    if let Some(changed_by) = &query.changed_by {
        qb.push(" AND changed_by = ").push_bind(changed_by.clone());
    }
    if let Some(from) = query.from {
        qb.push(" AND changed_at >= ").push_bind(from);
    }
    if let Some(to) = query.to {
        qb.push(" AND changed_at <= ").push_bind(to);
    }
}
