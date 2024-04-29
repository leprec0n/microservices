use crate::template::Snackbar;

use askama::Template;
use axum::response::Html;
use bb8_postgres::PostgresConnectionManager;
use bb8_redis::{bb8::Pool, RedisConnectionManager};
use reqwest::StatusCode;
use tokio_postgres::NoTls;
use tracing::debug;

use super::{PostgresConn, RedisConn};

pub async fn extract_redis_conn<'a>(
    pool: &'a Pool<RedisConnectionManager>,
    snackbar: &mut Snackbar<'_>,
) -> Result<RedisConn<'a>, (StatusCode, Html<String>)> {
    match pool.get().await {
        Ok(v) => Ok(v),
        Err(e) => {
            debug!("Cannot get Redis connection from pool: {:?}", e);
            snackbar.message = "Could not process request";
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            ));
        }
    }
}

pub async fn extract_postgres_conn<'a>(
    pool: &'a Pool<PostgresConnectionManager<NoTls>>,
    snackbar: &mut Snackbar<'_>,
) -> Result<PostgresConn<'a>, (StatusCode, Html<String>)> {
    match pool.get().await {
        Ok(v) => Ok(v),
        Err(e) => {
            debug!("Cannot get Postgres connection from pool: {:?}", e);
            snackbar.message = "Could not process request";
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            ));
        }
    }
}
