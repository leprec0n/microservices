mod db;

use std::{env, str::FromStr};
use tracing::Level;
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub use db::*;

pub fn get_env_var(s: &str) -> String {
    env::var_os(s).unwrap().into_string().unwrap()
}

/// Configure tracing with tracing_subscriber.
pub fn configure_tracing(log_level: &str) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout.with_max_level(Level::from_str(log_level).unwrap())),
        )
        .init();
}
