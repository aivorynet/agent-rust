//! AIVory Rust Agent Test Application
//!
//! Generates various panic types to test exception capture and stack trace extraction.
//! Note: Rust doesn't have runtime reflection, so local variable capture is limited.
//!
//! Usage:
//!   cd monitor-agents/agent-rust
//!   AIVORY_API_KEY=test-key-123 AIVORY_BACKEND_URL=ws://localhost:19999/api/monitor/agent/v1 AIVORY_DEBUG=true cargo run --example test_app

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

fn main() {
    println!("===========================================");
    println!("AIVory Rust Agent Test Application");
    println!("===========================================");

    // Initialize the agent
    aivory_monitor::init(aivory_monitor::Config {
        api_key: std::env::var("AIVORY_API_KEY").unwrap_or_else(|_| "test-key-123".to_string()),
        backend_url: std::env::var("AIVORY_BACKEND_URL")
            .unwrap_or_else(|_| "ws://localhost:19999/api/monitor/agent/v1".to_string()),
        environment: std::env::var("AIVORY_ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string()),
        debug: std::env::var("AIVORY_DEBUG")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false),
        ..Default::default()
    });

    // Set user context
    aivory_monitor::set_user(
        Some("test-user-001"),
        Some("tester@example.com"),
        Some("tester"),
    );

    // Wait for agent to connect
    println!("Waiting for agent to connect...");
    thread::sleep(Duration::from_secs(3));
    println!("Starting exception tests...\n");

    // Test 1: Manual error capture
    println!("--- Test 1: Manual Error Capture ---");
    let err = std::io::Error::new(std::io::ErrorKind::Other, "Test IO error");
    aivory_monitor::capture_error(&err);
    println!("Captured error: {}\n", err);
    thread::sleep(Duration::from_secs(3));

    // Test 2: Error with context
    println!("--- Test 2: Error with Context ---");
    let err2 = std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid input value");
    let mut context = HashMap::new();
    context.insert("iteration".to_string(), serde_json::json!(2));
    context.insert("test_var".to_string(), serde_json::json!("test-value-2"));
    aivory_monitor::capture_error_with_context(&err2, context);
    println!("Captured error with context: {}\n", err2);
    thread::sleep(Duration::from_secs(3));

    // Test 3: Panic (will be caught by panic hook)
    println!("--- Test 3: Panic ---");
    println!("Triggering panic...");

    // Spawn a thread that will panic so we don't exit the main program
    let handle = thread::spawn(|| {
        // Create some local variables (Rust can't capture these at runtime)
        let test_var = "test-value-3";
        let count = 30;
        let items = vec!["apple", "banana", "cherry"];

        // Use variables to prevent unused warnings
        let _ = (test_var, count, items);

        panic!("Test panic from spawned thread!");
    });

    // Wait for thread to panic
    let _ = handle.join();
    println!("Recovered from panic in spawned thread\n");
    thread::sleep(Duration::from_secs(3));

    println!("===========================================");
    println!("Test complete. Check database for exceptions.");
    println!("Note: Rust cannot capture local variables at runtime.");
    println!("===========================================");

    // Keep running briefly to allow final messages to send
    thread::sleep(Duration::from_secs(2));

    // Shutdown cleanly
    aivory_monitor::shutdown();
}
