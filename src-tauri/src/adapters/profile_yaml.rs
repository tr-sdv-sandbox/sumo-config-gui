use crate::model::{ComponentConfig, Deployment, PartConfig, VehicleConfig};
use crate::schema::{
    path_key, sanitize_segment, sibling_file_for_clone, ConfigResult, SchemaAdapter,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ProfileYamlAdapter;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfileYamlFile {
    #[serde(default)]
    vehicle: ProfileVehicle,
    #[serde(default)]
    target: BTreeMap<String, JsonValue>,
    #[serde(default)]
    labels: BTreeMap<String, JsonValue>,
    #[serde(default)]
    deployment: Deployment,
    #[serde(default)]
    components: YamlValue,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfileVehicle {
    #[serde(default)]
    id: String,
    #[serde(default)]
    kind: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfileComponent {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    update_mode: Option<String>,
    #[serde(default)]
    target: BTreeMap<String, JsonValue>,
    #[serde(default)]
    parts: YamlValue,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfilePart {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

impl SchemaAdapter for ProfileYamlAdapter {
    fn name(&self) -> &'static str {
        "profile-yaml"
    }

    fn detect(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yaml") | Some("yml")
        )
    }

    fn load(&self, path: &Path) -> ConfigResult<VehicleConfig> {
        let raw = fs::read_to_string(path)?;
        let file: ProfileYamlFile = serde_yaml::from_str(&raw)?;
        Ok(to_model(file, path))
    }

    fn save(&self, config: &VehicleConfig) -> ConfigResult<VehicleConfig> {
        let path = PathBuf::from(&config.source_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = from_model(config);
        fs::write(&path, serde_yaml::to_string(&file)?)?;
        self.load(&path)
    }

    fn clone_from(
        &self,
        source: &VehicleConfig,
        new_id: &str,
        channel: Option<&str>,
        profile: Option<&str>,
    ) -> ConfigResult<VehicleConfig> {
        let mut clone = source.clone();
        clone.id = new_id.to_string();
        clone.disabled = false;
        if let Some(channel) = channel {
            clone.deployment.channel = channel.to_string();
        }
        if let Some(profile) = profile {
            clone.deployment.profile = profile.to_string();
        }
        let target = sibling_file_for_clone(&source.source_path, new_id, "yaml");
        clone.source_path = target.to_string_lossy().to_string();
        clone.key = path_key(&target);
        self.save(&clone)
    }

    fn disable(&self, source: &VehicleConfig) -> ConfigResult<VehicleConfig> {
        let mut disabled = source.clone();
        disabled.disabled = true;
        self.save(&disabled)
    }
}

fn to_model(file: ProfileYamlFile, path: &Path) -> VehicleConfig {
    VehicleConfig {
        key: path_key(path),
        id: file.vehicle.id,
        kind: if file.vehicle.kind.is_empty() {
            "vehicle".into()
        } else {
            file.vehicle.kind
        },
        deployment: Deployment {
            channel: if file.deployment.channel.is_empty() {
                "default".into()
            } else {
                file.deployment.channel
            },
            profile: if file.deployment.profile.is_empty() {
                "default".into()
            } else {
                file.deployment.profile
            },
        },
        target: file.target,
        labels: file.labels,
        components: parse_components(file.components),
        disabled: file.disabled,
        schema: "profile-yaml".into(),
        source_path: path.to_string_lossy().to_string(),
    }
}

fn from_model(config: &VehicleConfig) -> ProfileYamlFile {
    let mut components = Mapping::new();
    for component in &config.components {
        let mut parts = Mapping::new();
        for part in &component.parts {
            let source = part.source.clone().unwrap_or_default();
            if part.kind.is_empty() || part.kind == "file" {
                parts.insert(
                    YamlValue::String(part.id.clone()),
                    YamlValue::String(source),
                );
            } else {
                let mut part_map = Mapping::new();
                part_map.insert(
                    YamlValue::String("kind".into()),
                    YamlValue::String(part.kind.clone()),
                );
                part_map.insert(
                    YamlValue::String("source".into()),
                    YamlValue::String(source),
                );
                parts.insert(
                    YamlValue::String(part.id.clone()),
                    YamlValue::Mapping(part_map),
                );
            }
        }

        let mut component_value = serde_yaml::to_value(ProfileComponent {
            path: None,
            kind: component.kind.clone(),
            version: component.version.clone(),
            update_mode: component.update_mode.clone(),
            target: component.target.clone(),
            parts: YamlValue::Mapping(parts),
        })
        .unwrap_or(YamlValue::Mapping(Mapping::new()));

        if let YamlValue::Mapping(map) = &mut component_value {
            map.remove(YamlValue::String("path".into()));
        }
        components.insert(YamlValue::String(component.path.clone()), component_value);
    }

    ProfileYamlFile {
        vehicle: ProfileVehicle {
            id: config.id.clone(),
            kind: config.kind.clone(),
        },
        target: config.target.clone(),
        labels: config.labels.clone(),
        deployment: config.deployment.clone(),
        components: YamlValue::Mapping(components),
        disabled: config.disabled,
    }
}

fn parse_components(value: YamlValue) -> Vec<ComponentConfig> {
    match value {
        YamlValue::Mapping(map) => map
            .into_iter()
            .filter_map(|(key, value)| {
                let path = key.as_str().unwrap_or_default().to_string();
                let mut component: ProfileComponent = serde_yaml::from_value(value).ok()?;
                component.path = Some(path);
                Some(component_to_model(component))
            })
            .collect(),
        YamlValue::Sequence(items) => items
            .into_iter()
            .filter_map(|value| serde_yaml::from_value::<ProfileComponent>(value).ok())
            .map(component_to_model)
            .collect(),
        _ => Vec::new(),
    }
}

fn component_to_model(component: ProfileComponent) -> ComponentConfig {
    ComponentConfig {
        path: component.path.unwrap_or_else(|| "component".into()),
        kind: component.kind,
        version: component.version,
        update_mode: component.update_mode,
        target: component.target,
        parts: parse_parts(component.parts),
    }
}

fn parse_parts(value: YamlValue) -> Vec<PartConfig> {
    match value {
        YamlValue::Mapping(map) => map
            .into_iter()
            .map(|(key, value)| {
                let id = key.as_str().unwrap_or_default().to_string();
                match value {
                    YamlValue::String(source) => PartConfig {
                        id,
                        kind: "file".into(),
                        source: Some(source),
                    },
                    YamlValue::Mapping(_) => {
                        let part: ProfilePart = serde_yaml::from_value(value).unwrap_or_default();
                        PartConfig {
                            id,
                            kind: part.kind.unwrap_or_else(|| "file".into()),
                            source: part.source,
                        }
                    }
                    other => PartConfig {
                        id,
                        kind: "file".into(),
                        source: Some(value_to_string(other)),
                    },
                }
            })
            .collect(),
        YamlValue::Sequence(items) => items
            .into_iter()
            .filter_map(|value| serde_yaml::from_value::<ProfilePart>(value).ok())
            .map(|part| PartConfig {
                id: part.id.unwrap_or_default(),
                kind: part.kind.unwrap_or_else(|| "file".into()),
                source: part.source,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn value_to_string(value: YamlValue) -> String {
    match value {
        YamlValue::Null => String::new(),
        YamlValue::Bool(v) => v.to_string(),
        YamlValue::Number(v) => v.to_string(),
        YamlValue::String(v) => v,
        other => serde_yaml::to_string(&other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

#[allow(dead_code)]
fn default_clone_name(source: &VehicleConfig, new_id: &str) -> String {
    format!(
        "{}-{}",
        sanitize_segment(&source.deployment.profile),
        sanitize_segment(new_id)
    )
}
