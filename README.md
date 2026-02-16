# AIVory Monitor Rust Agent

Panic hook and error capture agent for Rust applications.

## Requirements

- Rust 1.70+ (2021 edition)
- Tokio async runtime
- Cargo for building

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
aivory-monitor = "1.0"
tokio = { version = "1.35", features = ["rt-multi-thread", "macros"] }
```

Or install from crates.io:

```bash
cargo add aivory-monitor
cargo add tokio --features rt-multi-thread,macros
```

## Usage

### Basic Initialization

```rust
use aivory_monitor::Config;

#[tokio::main]
async fn main() {
    // Initialize with API key from environment
    aivory_monitor::init(Config::default());

    // Or configure explicitly
    aivory_monitor::init(Config::new("your-api-key")
        .environment("production")
        .sampling_rate(1.0)
        .debug(false)
    );

    // Your application code
    run_application().await;
}
```

### Manual Error Capture

Capture errors explicitly:

```rust
use std::fs::File;

fn risky_operation() -> Result<(), Box<dyn std::error::Error>> {
    match File::open("config.toml") {
        Ok(file) => Ok(()),
        Err(e) => {
            // Capture the error
            aivory_monitor::capture_error(&e);
            Err(Box::new(e))
        }
    }
}
```

### Error Capture with Context

Add custom context to errors:

```rust
use std::collections::HashMap;
use serde_json::json;

fn process_user_request(user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    match dangerous_operation() {
        Ok(result) => Ok(result),
        Err(e) => {
            let mut context = HashMap::new();
            context.insert("user_id".to_string(), json!(user_id));
            context.insert("request_id".to_string(), json!("req-12345"));

            aivory_monitor::capture_error_with_context(&e, context);
            Err(e)
        }
    }
}
```

### Setting Global Context

Set context that applies to all captured errors:

```rust
use std::collections::HashMap;
use serde_json::json;

fn main() {
    aivory_monitor::init(Config::default());

    // Set global context
    let mut context = HashMap::new();
    context.insert("service".to_string(), json!("api-server"));
    context.insert("version".to_string(), json!("1.2.3"));
    aivory_monitor::set_context(context);

    // Set user information
    aivory_monitor::set_user(
        Some("user-123"),
        Some("user@example.com"),
        Some("john_doe")
    );
}
```

### Automatic Panic Hook

The agent automatically installs a panic hook on initialization to capture panics:

```rust
fn main() {
    aivory_monitor::init(Config::default());

    // This panic will be automatically captured and sent to AIVory
    panic!("Something went wrong!");
}
```

### Graceful Shutdown

```rust
fn main() {
    aivory_monitor::init(Config::default());

    // Run application...

    // Shutdown agent before exit
    aivory_monitor::shutdown();
}
```

## Configuration

Configure via environment variables or `Config` struct:

| Environment Variable | Config Field | Default | Description |
|---------------------|--------------|---------|-------------|
| `AIVORY_API_KEY` | `api_key` | - | API key (required) |
| `AIVORY_BACKEND_URL` | `backend_url` | `wss://api.aivory.net/ws/agent` | Backend WebSocket URL |
| `AIVORY_ENVIRONMENT` | `environment` | `production` | Environment name |
| `AIVORY_SAMPLING_RATE` | `sampling_rate` | `1.0` | Error sampling rate (0.0-1.0) |
| `AIVORY_MAX_DEPTH` | `max_capture_depth` | `10` | Max variable depth |
| `AIVORY_MAX_STRING_LENGTH` | `max_string_length` | `1000` | Max string capture length |
| `AIVORY_MAX_COLLECTION_SIZE` | `max_collection_size` | `100` | Max collection size |
| `AIVORY_DEBUG` | `debug` | `false` | Enable debug logging |

### Environment Variable Configuration

```bash
export AIVORY_API_KEY=your_api_key
export AIVORY_ENVIRONMENT=staging
export AIVORY_SAMPLING_RATE=0.5
cargo run
```

### Programmatic Configuration

```rust
let config = Config::new("your-api-key")
    .environment("production")
    .backend_url("wss://custom.backend.url/ws/agent")
    .sampling_rate(0.8)
    .debug(true);

aivory_monitor::init(config);
```

## Building from Source

```bash
cd monitor-agents/agent-rust
cargo build --release
```

The compiled library will be at `target/release/libaivory_monitor.rlib`.

## How It Works

1. **Panic Hook Installation**: Automatically installs a panic hook when `init()` is called
2. **Backtrace Capture**: Uses the `backtrace` crate to capture full stack traces
3. **Async WebSocket**: Uses Tokio + tokio-tungstenite for async WebSocket communication
4. **Error Serialization**: Converts errors and panics to JSON using serde
5. **Context Tracking**: Thread-safe context storage using parking_lot RwLock
6. **Sampling**: Configurable sampling rate to reduce overhead in high-volume scenarios

## Async Runtime

The agent requires a Tokio runtime. It will create its own runtime internally if not already running in one, but for best performance, initialize the agent within your application's Tokio runtime:

```rust
#[tokio::main]
async fn main() {
    aivory_monitor::init(Config::default());
    // Your async code here
}
```

For non-async applications, the agent will spawn a background thread with its own Tokio runtime:

```rust
fn main() {
    aivory_monitor::init(Config::default());
    // Your sync code here
}
```

## Framework Integration

### Actix Web

```rust
use actix_web::{web, App, HttpServer, middleware};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    aivory_monitor::init(Config::default());

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(web::resource("/").to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Rocket

```rust
#[macro_use] extern crate rocket;

#[launch]
fn rocket() -> _ {
    aivory_monitor::init(Config::default());

    rocket::build()
        .mount("/", routes![index])
}
```

### Axum

```rust
use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    aivory_monitor::init(Config::default());

    let app = Router::new()
        .route("/", get(handler));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## Troubleshooting

**Agent not capturing panics:**
- Ensure `init()` is called at the start of your application
- Verify API key is set correctly
- Check that another panic hook isn't overriding ours

**WebSocket connection failures:**
- Verify `AIVORY_BACKEND_URL` is correct
- Check network connectivity to backend
- Enable debug logging: `AIVORY_DEBUG=true`

**No errors being captured:**
- Verify errors implement `std::error::Error` trait
- Check sampling rate isn't set too low
- Ensure `capture_error()` is being called

**Compilation errors:**
- Verify Rust version is 1.70+
- Ensure tokio features include `rt-multi-thread` and `macros`
- Run `cargo clean` and rebuild

**Performance concerns:**
- Use sampling rate < 1.0 for high-volume applications
- Reduce `max_capture_depth` and `max_string_length`
- Consider async initialization to avoid blocking startup
