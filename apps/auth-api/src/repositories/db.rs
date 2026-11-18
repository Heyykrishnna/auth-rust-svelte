use sqlx::{PgPool, Postgres, Transaction};
use std::future::Future;

use crate::errors::AppError;
use crate::models::EnvironmentId;

pub type PgTx<'a> = Transaction<'a, Postgres>;

pub async fn begin_tx(pool: &PgPool) -> Result<PgTx<'_>, AppError> {
    pool.begin().await.map_err(AppError::from)
}

pub async fn commit_tx(tx: PgTx<'_>) -> Result<(), AppError> {
    tx.commit().await.map_err(AppError::from)
}

pub async fn set_local_environment_context(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
) -> Result<(), AppError> {
    sqlx::query("SELECT set_config('app.current_environment_id', $1, true)")
        .bind(environment_id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn begin_tenant_tx(
    pool: &PgPool,
    environment_id: EnvironmentId,
) -> Result<PgTx<'_>, AppError> {
    let mut tx = begin_tx(pool).await?;
    set_local_environment_context(&mut tx, environment_id).await?;
    Ok(tx)
}

pub async fn begin_rls_enforced_tenant_tx(
    pool: &PgPool,
    environment_id: EnvironmentId,
) -> Result<PgTx<'_>, AppError> {
    let mut tx = begin_tx(pool).await?;
    sqlx::query("SET LOCAL ROLE app_tenant_rls")
        .execute(&mut *tx)
        .await?;
    set_local_environment_context(&mut tx, environment_id).await?;
    Ok(tx)
}

pub async fn with_tenant_tx<'a, T, F, Fut>(
    pool: &'a PgPool,
    environment_id: EnvironmentId,
    operation: F,
) -> Result<T, AppError>
where
    F: FnOnce(PgTx<'a>) -> Fut,
    Fut: Future<Output = Result<(PgTx<'a>, T), AppError>>,
{
    let tx = begin_tenant_tx(pool, environment_id).await?;
    let (tx, output) = operation(tx).await?;
    commit_tx(tx).await?;
    Ok(output)
}
