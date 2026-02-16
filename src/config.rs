//! Agent configuration.

use std::env;
use uuid::Uuid;

/// Agent configuration.
#[derive(Clone)]
pub struct Config {
    /// AIVory API key.
    pub api_key: String,
    /// Backend WebSocket URL.
    pub backend_url: String,
    /// Environment name.
    pub environment: String,
    /// Sampling rate (0.0 - 1.0).
    pub sampling_rate: f64,
    /// Maximum capture depth for variables.
    pub max_capture_depth: usize,
    /// Maximum string length to capture.
    pub max_string_length: usize,
    /// Maximum collection size to capture.
    pub max_collection_size: usize,
    /// Enable debug logging.
    pub debug: bool,
    /// Hostname.
    pub hostname: String,
    /// Agent ID.
    pub agent_id: String,
}

impl Default for Config {
    fn default() -> Self {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Config {
            api_key: env::var("AIVORY_API_KEY").unwrap_or_default(),
            backend_url: env::var("AIVORY_BACKEND_URL")
                .unwrap_or_else(|_| "wss://api.aivory.net/ws/agent".to_string()),
            environment: env::var("AIVORY_ENVIRONMENT")
                .unwrap_or_else(|_| "production".to_string()),
            sampling_rate: env::var("AIVORY_SAMPLING_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0),
            max_capture_depth: env::var("AIVORY_MAX_DEPTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            max_string_length: env::var("AIVORY_MAX_STRING_LENGTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000),
            max_collection_size: env::var("AIVORY_MAX_COLLECTION_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            debug: env::var("AIVORY_DEBUG")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(false),
            hostname,
            agent_id: format!("agent-{}-{}",
                hex::encode(&chrono::Utc::now().timestamp().to_be_bytes()[4..]),
                &Uuid::new_v4().to_string()[..8]),
        }
    }
}

impl Config {
    /// Creates a new configuration with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Config {
            api_key: api_key.into(),
            ..Default::default()
        }
    }

    /// Sets the environment.
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    /// Sets the backend URL.
    pub fn backend_url(mut self, url: impl Into<String>) -> Self {
        self.backend_url = url.into();
        self
    }

    /// Sets the sampling rate.
    pub fn sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = rate;
        self
    }

    /// Enables debug logging.
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Determines if the current event should be sampled.
    pub fn should_sample(&self) -> bool {
        if self.sampling_rate >= 1.0 {
            return true;
        }
        if self.sampling_rate <= 0.0 {
            return false;
        }
        rand::random::<f64>() < self.sampling_rate
    }

    /// Gets runtime information.
    pub fn runtime_info(&self) -> RuntimeInfo {
        RuntimeInfo {
            runtime: "rust".to_string(),
            runtime_version: env!("CARGO_PKG_VERSION").to_string(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        }
    }
}

/// Runtime information.
#[derive(Clone, serde::Serialize)]
pub struct RuntimeInfo {
    pub runtime: String,
    pub runtime_version: String,
    pub platform: String,
    pub arch: String,
}

mod rand {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};

    thread_local! {
        static SEED: Cell<u64> = Cell::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        );
    }

    pub fn random<T: FromU64>() -> T {
        SEED.with(|seed| {
            let mut s = seed.get();
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            seed.set(s);
            T::from_u64(s)
        })
    }

    pub trait FromU64 {
        fn from_u64(value: u64) -> Self;
    }

    impl FromU64 for f64 {
        fn from_u64(value: u64) -> Self {
            (value as f64) / (u64::MAX as f64)
        }
    }
}
