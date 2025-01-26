// logging.rs
use log4rs::{
    append::rolling_file::{
        RollingFileAppender,
        policy::compound::{
            CompoundPolicy,
            trigger::size::SizeTrigger,
            roll::fixed_window::FixedWindowRoller,
        },
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    Handle,
};
use serde::Deserialize;
use log::LevelFilter;
use std::path::PathBuf;
use anyhow::Context;

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default = "default_max_size")]
    pub max_size: u64,        // in bytes
    #[serde(default = "default_max_files")]
    pub max_files: u32,
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
}

// Default value functions for serde
fn default_enabled() -> bool { true }
fn default_level() -> String { "info".to_string() }
fn default_max_size() -> u64 { 10 * 1024 * 1024 }  // 10MB
fn default_max_files() -> u32 { 7 }
fn default_log_dir() -> PathBuf { PathBuf::from("logs") }

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            level: default_level(),
            max_size: default_max_size(),
            max_files: default_max_files(),
            log_dir: default_log_dir(),
        }
    }
}

impl LogConfig {
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            // Optional: Also allow environment variables to override config file
            .add_source(config::Environment::with_prefix("LOG"))
            .build()?;

        settings.try_deserialize()
    }

    pub fn get_level_filter(&self) -> LevelFilter {
        match self.level.to_lowercase().as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }
}

pub fn initialize_logging(config_path: &str) -> anyhow::Result<Handle> {
    let config = LogConfig::from_file(config_path).unwrap_or_default();

    if !config.enabled {
        return Ok(log4rs::init_config(
            Config::builder()
                .build(Root::builder().build(LevelFilter::Off))
                .unwrap()
        )?);
    }

    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&config.log_dir)
        .context("Failed to create log directory")?;

    let log_file_path = config.log_dir.join("application.log");
    let pattern = "{d(%Y-%m-%d %H:%M:%S)} {l} {t} - {m}{n}";

    // Set up log rotation
    let window_size = config.max_files;
    let fixed_window_roller = FixedWindowRoller::builder()
        .build(
            config.log_dir.join("application.{}.log").to_str().unwrap(),
            window_size,
        )
        .context("Failed to build fixed window roller")?;

    let size_trigger = SizeTrigger::new(config.max_size);
    let compound_policy = CompoundPolicy::new(
        Box::new(size_trigger),
        Box::new(fixed_window_roller),
    );

    let rolling_appender = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build(log_file_path, Box::new(compound_policy))
        .context("Failed to build rolling file appender")?;

    let config = Config::builder()
        .appender(Appender::builder().build("rolling", Box::new(rolling_appender)))
        .build(Root::builder().appender("rolling").build(config.get_level_filter()))
        .context("Failed to build logging config")?;

    Ok(log4rs::init_config(config)
        .context("Failed to initialize logging config")?)
}

pub fn cleanup_old_logs(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = LogConfig::from_file(config_path).unwrap_or_default();
    let log_dir = config.log_dir;

    if !log_dir.exists() {
        return Ok(());
    }

    let cleanup_threshold = chrono::Local::now() - chrono::Duration::days(config.max_files as i64);

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            if let Ok(modified_time) = metadata.modified() {
                if let Ok(duration) = modified_time.duration_since(chrono::Utc::now().into()) {
                    let elapsed_seconds = duration.as_secs() as i64;
                    if elapsed_seconds as i64 > (cleanup_threshold - chrono::Local::now()).num_days() {
                        std::fs::remove_file(entry.path())?;
                    }
                }
            }
        }
    }

    Ok(())
}