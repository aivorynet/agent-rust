//! Error and panic capture functionality.

use crate::config::Config;
use backtrace::Backtrace;
use chrono::Utc;
use serde::Serialize;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::error::Error;
use uuid::Uuid;

/// Captured exception data.
#[derive(Clone, Serialize)]
pub struct ExceptionCapture {
    pub id: String,
    pub exception_type: String,
    pub message: String,
    pub fingerprint: String,
    pub stack_trace: Vec<StackFrame>,
    pub local_variables: HashMap<String, Variable>,
    pub context: HashMap<String, serde_json::Value>,
    pub captured_at: String,
    pub agent_id: String,
    pub environment: String,
    pub runtime_info: crate::config::RuntimeInfo,
}

/// A single stack frame.
#[derive(Clone, Serialize)]
pub struct StackFrame {
    pub method_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_number: Option<u32>,
    pub is_native: bool,
    pub source_available: bool,
}

/// A captured variable.
#[derive(Clone, Serialize)]
pub struct Variable {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: String,
    pub value: String,
    pub is_null: bool,
    pub is_truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<HashMap<String, Variable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_elements: Option<Vec<Variable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_length: Option<usize>,
}

/// Trait for types that can be captured as errors.
pub trait CaptureError {
    fn capture(&self, config: &Config) -> ExceptionCapture;
}

impl<E: Error> CaptureError for E {
    fn capture(&self, config: &Config) -> ExceptionCapture {
        capture_error(self, config)
    }
}

/// Captures an error with stack trace.
pub fn capture_error<E: Error + ?Sized>(error: &E, config: &Config) -> ExceptionCapture {
    let stack_trace = capture_stack_trace();
    let exception_type = std::any::type_name::<E>()
        .split("::")
        .last()
        .unwrap_or("Error")
        .to_string();
    let message = error.to_string();
    let fingerprint = calculate_fingerprint(&exception_type, &stack_trace);

    ExceptionCapture {
        id: Uuid::new_v4().to_string(),
        exception_type,
        message,
        fingerprint,
        stack_trace,
        local_variables: HashMap::new(),
        context: HashMap::new(),
        captured_at: Utc::now().to_rfc3339(),
        agent_id: config.agent_id.clone(),
        environment: config.environment.clone(),
        runtime_info: config.runtime_info(),
    }
}

/// Captures a panic with stack trace.
pub fn capture_panic(message: &str, location: Option<String>, config: &Config) -> ExceptionCapture {
    let stack_trace = capture_stack_trace();
    let fingerprint = calculate_fingerprint("panic", &stack_trace);

    let mut context = HashMap::new();
    context.insert("panic".to_string(), serde_json::json!(true));
    if let Some(loc) = location {
        context.insert("location".to_string(), serde_json::json!(loc));
    }

    ExceptionCapture {
        id: Uuid::new_v4().to_string(),
        exception_type: "panic".to_string(),
        message: message.to_string(),
        fingerprint,
        stack_trace,
        local_variables: HashMap::new(),
        context,
        captured_at: Utc::now().to_rfc3339(),
        agent_id: config.agent_id.clone(),
        environment: config.environment.clone(),
        runtime_info: config.runtime_info(),
    }
}

fn capture_stack_trace() -> Vec<StackFrame> {
    let bt = Backtrace::new();
    let mut frames = Vec::new();

    for frame in bt.frames() {
        for symbol in frame.symbols() {
            let method_name = symbol
                .name()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());

            // Skip internal frames
            if method_name.starts_with("std::")
                || method_name.starts_with("core::")
                || method_name.starts_with("backtrace::")
                || method_name.starts_with("aivory_monitor::capture")
            {
                continue;
            }

            let file_path = symbol.filename().map(|p| p.to_string_lossy().to_string());
            let file_name = file_path.as_ref().and_then(|p| {
                p.split(['/', '\\']).last().map(|s| s.to_string())
            });

            let is_native = file_path
                .as_ref()
                .map(|p| p.contains(".rustup") || p.contains("registry"))
                .unwrap_or(true);

            frames.push(StackFrame {
                method_name: extract_function_name(&method_name),
                file_name,
                file_path: file_path.clone(),
                line_number: symbol.lineno(),
                column_number: symbol.colno(),
                is_native,
                source_available: !is_native && file_path.is_some(),
            });

            if frames.len() >= 50 {
                break;
            }
        }

        if frames.len() >= 50 {
            break;
        }
    }

    frames
}

fn extract_function_name(full_name: &str) -> String {
    // Extract just the function name from a fully qualified name
    // e.g., "my_crate::module::function" -> "function"
    full_name
        .split("::")
        .last()
        .unwrap_or(full_name)
        .to_string()
}

fn calculate_fingerprint(exception_type: &str, stack_trace: &[StackFrame]) -> String {
    let mut parts = vec![exception_type.to_string()];

    let mut added = 0;
    for frame in stack_trace {
        if added >= 5 {
            break;
        }
        if frame.is_native {
            continue;
        }

        parts.push(format!(
            "{}:{}",
            frame.method_name,
            frame.line_number.unwrap_or(0)
        ));
        added += 1;
    }

    let mut hasher = Sha256::new();
    hasher.update(parts.join(":"));
    let result = hasher.finalize();
    hex::encode(&result[..8])
}
