use sqlx::{PgPool, Postgres, Transaction};

use crate::errors::AppError;

pub type PgTx<'a> = Transaction<'a, Postgres>;

pub async fn begin_tx(pool: &PgPool) -> Result<PgTx<'_>, AppError> {
    pool.begin().await.map_err(AppError::from)
}

pub async fn commit_tx(tx: PgTx<'_>) -> Result<(), AppError> {
    tx.commit().await.map_err(AppError::from)
}

