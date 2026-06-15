use crate::model::VehicleConfig;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("unsupported config schema for {0}")]
    Unsupported(String),
    #[error("vehicle config not found: {0}")]
    NotFound(String),
    #[error("validation failed: {0}")]
    Validation(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;

pub trait SchemaAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn detect(&self, path: &Path) -> bool;
    fn load(&self, path: &Path) -> ConfigResult<VehicleConfig>;
    fn save(&self, config: &VehicleConfig) -> ConfigResult<VehicleConfig>;
    fn clone_from(
        &self,
        source: &VehicleConfig,
        new_id: &str,
        channel: Option<&str>,
        profile: Option<&str>,
    ) -> ConfigResult<VehicleConfig>;
    fn disable(&self, source: &VehicleConfig) -> ConfigResult<VehicleConfig>;
}

pub struct SchemaRegistry {
    adapters: Vec<Box<dyn SchemaAdapter>>,
}

impl SchemaRegistry {
    pub fn new(adapters: Vec<Box<dyn SchemaAdapter>>) -> Self {
        Self { adapters }
    }

    pub fn list_adapters(&self) -> Vec<&'static str> {
        self.adapters.iter().map(|a| a.name()).collect()
    }

    pub fn adapter_for_path(&self, path: &Path) -> Option<&dyn SchemaAdapter> {
        self.adapters
            .iter()
            .find(|adapter| adapter.detect(path))
            .map(|adapter| adapter.as_ref())
    }

    pub fn adapter_by_name(&self, name: &str) -> Option<&dyn SchemaAdapter> {
        self.adapters
            .iter()
            .find(|adapter| adapter.name() == name)
            .map(|adapter| adapter.as_ref())
    }

    pub fn load(&self, path: &Path) -> ConfigResult<VehicleConfig> {
        let adapter = self
            .adapter_for_path(path)
            .ok_or_else(|| ConfigError::Unsupported(path.display().to_string()))?;
        adapter.load(path)
    }

    pub fn adapter_for_config(&self, config: &VehicleConfig) -> ConfigResult<&dyn SchemaAdapter> {
        self.adapter_by_name(&config.schema)
            .ok_or_else(|| ConfigError::Unsupported(config.schema.clone()))
    }
}

pub fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn sanitize_segment(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn sibling_file_for_clone(source_path: &str, new_id: &str, extension: &str) -> PathBuf {
    let source = Path::new(source_path);
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}.{}", sanitize_segment(new_id), extension))
}

pub fn validation_errors(config: &VehicleConfig) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if config.id.trim().is_empty() {
        errors.push("vehicle id is required".into());
    }
    if config.deployment.channel.trim().is_empty() {
        errors.push("deployment.channel is required".into());
    }
    if config.deployment.profile.trim().is_empty() {
        errors.push("deployment.profile is required".into());
    }
    if config.components.is_empty() {
        warnings.push("vehicle has no components".into());
    }

    let mut component_paths = BTreeSet::new();
    for component in &config.components {
        if component.path.trim().is_empty() {
            errors.push("component path is required".into());
        }
        if !component_paths.insert(component.path.clone()) {
            errors.push(format!("duplicate component path: {}", component.path));
        }

        let mut part_ids = BTreeSet::new();
        for part in &component.parts {
            if part.id.trim().is_empty() {
                errors.push(format!(
                    "component {} contains a part with no id",
                    component.path
                ));
            }
            if !part_ids.insert(part.id.clone()) {
                errors.push(format!(
                    "component {} contains duplicate part id: {}",
                    component.path, part.id
                ));
            }
        }
    }

    (errors, warnings)
}
