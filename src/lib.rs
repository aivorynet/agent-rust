//! AIVory Monitor Rust Agent
//!
//! Remote debugging with AI-powered fix generation.
//!
//! # Usage
//!
//! ```rust,no_run
//! use aivory_monitor::Agent;
//!
//! fn main() {
//!     // Initialize the agent
//!     aivory_monitor::init(aivory_monitor::Config {
//!         api_key: "your-api-key".to_string(),
//!         environment: "production".to_string(),
//!         ..Default::default()
//!     });
//!
//!     // Or use the panic hook (automatically installed)
//! }
//! ```

pub mod config;
pub mod capture;
pub mod transport;

pub use config::Config;
pub use capture::{CaptureError, ExceptionCapture};

use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::panic;
use std::sync::Arc;

static AGENT: OnceCell<Arc<Agent>> = OnceCell::new();

/// The main AIVory Monitor agent.
pub struct Agent {
    config: Config,
    connection: transport::Connection,
    custom_context: RwLock<HashMap<String, serde_json::Value>>,
    user: RwLock<HashMap<String, String>>,
}

impl Agent {
    /// Creates a new agent with the given configuration.
    pub fn new(config: Config) -> Self {
        Agent {
            config,
            connection: transport::Connection::new(),
            custom_context: RwLock::new(HashMap::new()),
            user: RwLock::new(HashMap::new()),
        }
    }

    /// Starts the agent and connects to the backend.
    pub async fn start(&self) {
        if self.config.api_key.is_empty() {
            eprintln!("[AIVory Monitor] API key is required");
            return;
        }

        // Install panic hook
        self.install_panic_hook();

        // Connect to backend
        self.connection.connect(&self.config).await;

        println!("[AIVory Monitor] Agent v1.0.0 initialized");
        println!("[AIVory Monitor] Environment: {}", self.config.environment);
    }

    /// Stops the agent.
    pub async fn stop(&self) {
        self.connection.disconnect().await;
        println!("[AIVory Monitor] Agent stopped");
    }

    /// Captures an error.
    pub fn capture_error<E: std::error::Error>(&self, error: &E, context: Option<HashMap<String, serde_json::Value>>) {
        if !self.config.should_sample() {
            return;
        }

        let mut exc = capture::capture_error(error, &self.config);

        // Add custom context
        {
            let custom = self.custom_context.read();
            for (k, v) in custom.iter() {
                exc.context.insert(k.clone(), v.clone());
            }
        }

        // Add user
        {
            let user = self.user.read();
            if !user.is_empty() {
                exc.context.insert("user".to_string(), serde_json::json!(user.clone()));
            }
        }

        // Add additional context
        if let Some(ctx) = context {
            for (k, v) in ctx {
                exc.context.insert(k, v);
            }
        }

        // Send to backend
        self.connection.send_exception(exc);
    }

    /// Sets custom context.
    pub fn set_context(&self, context: HashMap<String, serde_json::Value>) {
        let mut custom = self.custom_context.write();
        *custom = context;
    }

    /// Sets user information.
    pub fn set_user(&self, id: Option<&str>, email: Option<&str>, username: Option<&str>) {
        let mut user = self.user.write();
        user.clear();
        if let Some(id) = id {
            user.insert("id".to_string(), id.to_string());
        }
        if let Some(email) = email {
            user.insert("email".to_string(), email.to_string());
        }
        if let Some(username) = username {
            user.insert("username".to_string(), username.to_string());
        }
    }

    fn install_panic_hook(&self) {
        let config = self.config.clone();

        panic::set_hook(Box::new(move |panic_info| {
            if let Some(agent) = AGENT.get() {
                // Create an error from panic info
                let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                    s.clone()
                } else {
                    "panic occurred".to_string()
                };

                let location = panic_info.location().map(|loc| {
                    format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
                });

                let exc = capture::capture_panic(&message, location, &config);
                agent.connection.send_exception(exc);
            }

            // Print default panic message
            eprintln!("{}", panic_info);
        }));
    }
}

/// Initializes the global agent.
pub fn init(config: Config) {
    let agent = Arc::new(Agent::new(config));

    if AGENT.set(agent.clone()).is_err() {
        eprintln!("[AIVory Monitor] Agent already initialized");
        return;
    }

    // Start in background
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            agent.start().await;
        });
    });
}

/// Captures an error using the global agent.
pub fn capture_error<E: std::error::Error>(error: &E) {
    if let Some(agent) = AGENT.get() {
        agent.capture_error(error, None);
    }
}

/// Captures an error with context using the global agent.
pub fn capture_error_with_context<E: std::error::Error>(
    error: &E,
    context: HashMap<String, serde_json::Value>,
) {
    if let Some(agent) = AGENT.get() {
        agent.capture_error(error, Some(context));
    }
}

/// Sets custom context using the global agent.
pub fn set_context(context: HashMap<String, serde_json::Value>) {
    if let Some(agent) = AGENT.get() {
        agent.set_context(context);
    }
}

/// Sets user information using the global agent.
pub fn set_user(id: Option<&str>, email: Option<&str>, username: Option<&str>) {
    if let Some(agent) = AGENT.get() {
        agent.set_user(id, email, username);
    }
}

/// Shuts down the global agent.
pub fn shutdown() {
    if let Some(agent) = AGENT.get() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            agent.stop().await;
        });
    }
}
