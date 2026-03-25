use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub enum OutputFormat {
    Json,
    Human,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "human" => Ok(OutputFormat::Human),
            _ => Err(format!("Invalid format: {}. Use 'json' or 'human'", s)),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Output<T> {
    status: String,
    data: T,
    metadata: Metadata,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    timestamp: String,
    tool: String,
    version: String,
}

impl<T> Output<T> {
    pub fn success(tool: &str, data: T) -> Self {
        Self {
            status: "success".to_string(),
            data,
            metadata: Metadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                tool: tool.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}

impl Output<serde_json::Value> {
    pub fn error(tool: &str, code: &str, message: &str) -> Self {
        Self {
            status: "error".to_string(),
            data: serde_json::json!({
                "code": code,
                "message": message,
            }),
            metadata: Metadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                tool: tool.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}
