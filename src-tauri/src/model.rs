use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Deployment {
    pub channel: String,
    pub profile: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TargetTypeConfig {
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PartConfig {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ComponentConfig {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_path: Option<String>,
    #[serde(default)]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_mode: Option<String>,
    #[serde(default)]
    pub target: BTreeMap<String, Value>,
    #[serde(default)]
    pub parts: Vec<PartConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VehicleConfig {
    /// Stable UI/repository key. The first version uses the source path.
    pub key: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub target_type: TargetTypeConfig,
    #[serde(default)]
    pub deployment: Deployment,
    #[serde(default)]
    pub target: BTreeMap<String, Value>,
    #[serde(default)]
    pub labels: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_snapshot: Option<Value>,
    #[serde(default)]
    pub components: Vec<ComponentConfig>,
    #[serde(default)]
    pub disabled: bool,
    pub schema: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_errors(errors: Vec<String>, warnings: Vec<String>) -> Self {
        Self {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkStatus {
    pub available: bool,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl LinkStatus {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            available: true,
            state: "available".into(),
            message: Some(message.into()),
        }
    }

    pub fn missing(message: impl Into<String>) -> Self {
        Self {
            available: false,
            state: "missing".into(),
            message: Some(message.into()),
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            available: false,
            state: "unavailable".into(),
            message: Some(message.into()),
        }
    }

    pub fn skipped(message: impl Into<String>) -> Self {
        Self {
            available: false,
            state: "skipped".into(),
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TowerLinkage {
    pub tower2_channel: LinkStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tower1Config {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_serial: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_not_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VehicleSummary {
    pub key: String,
    pub id: String,
    pub kind: String,
    pub target_type: String,
    pub channel: String,
    pub profile: String,
    pub schema: String,
    pub source_path: String,
    pub disabled: bool,
    pub component_count: usize,
    pub part_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linkage: Option<TowerLinkage>,
}

impl VehicleSummary {
    pub fn from_config(config: &VehicleConfig) -> Self {
        let part_count = config.components.iter().map(|c| c.parts.len()).sum();
        Self {
            key: config.key.clone(),
            id: config.id.clone(),
            kind: config.kind.clone(),
            target_type: config.target_type.name.clone(),
            channel: config.deployment.channel.clone(),
            profile: config.deployment.profile.clone(),
            schema: config.schema.clone(),
            source_path: config.source_path.clone(),
            disabled: config.disabled,
            component_count: config.components.len(),
            part_count,
            linkage: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloneOptions {
    pub new_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandResponse<T> {
    pub value: T,
    pub validation: ValidationResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_root: Option<String>,
    pub tower1_url: String,
    pub tower2_url: String,
}
