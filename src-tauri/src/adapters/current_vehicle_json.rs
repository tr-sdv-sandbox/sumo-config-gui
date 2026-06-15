use crate::model::{ComponentConfig, Deployment, PartConfig, VehicleConfig};
use crate::schema::{path_key, sanitize_segment, ConfigResult, SchemaAdapter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CurrentVehicleJsonAdapter;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CurrentVehicleFile {
    #[serde(default, rename = "_comment", skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(default)]
    vehicle: CurrentVehicle,
    #[serde(default)]
    deployment: Option<Deployment>,
    #[serde(default)]
    target: BTreeMap<String, Value>,
    #[serde(default)]
    labels: BTreeMap<String, Value>,
    #[serde(default)]
    components: Vec<CurrentComponent>,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CurrentVehicle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CurrentComponent {
    path: String,
    #[serde(default)]
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    update_mode: Option<String>,
    #[serde(default)]
    target: BTreeMap<String, Value>,
    #[serde(default)]
    parts: Vec<CurrentPart>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CurrentPart {
    id: String,
    #[serde(default)]
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source: Option<String>,
}

impl SchemaAdapter for CurrentVehicleJsonAdapter {
    fn name(&self) -> &'static str {
        "current-vehicle-json"
    }

    fn detect(&self, path: &Path) -> bool {
        path.file_name().and_then(|n| n.to_str()) == Some("vehicle.json")
    }

    fn load(&self, path: &Path) -> ConfigResult<VehicleConfig> {
        let raw = fs::read_to_string(path)?;
        let file: CurrentVehicleFile = serde_json::from_str(&raw)?;
        Ok(to_model(file, path))
    }

    fn save(&self, config: &VehicleConfig) -> ConfigResult<VehicleConfig> {
        let path = PathBuf::from(&config.source_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = from_model(config);
        fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&file)?))?;
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
        clone.deployment.channel = channel
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| default_clone_channel(source, new_id));
        clone.deployment.profile = profile.unwrap_or(&source.deployment.profile).to_string();
        let target = clone_path_for_channel(&source.source_path, &clone.deployment.channel, new_id);
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

fn to_model(file: CurrentVehicleFile, path: &Path) -> VehicleConfig {
    let channel = file
        .deployment
        .as_ref()
        .map(|d| d.channel.clone())
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| infer_channel(path));
    let profile = file
        .deployment
        .as_ref()
        .map(|d| d.profile.clone())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "default".into());
    let id = file
        .vehicle
        .id
        .clone()
        .or_else(|| file.vehicle.tag.clone())
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    VehicleConfig {
        key: path_key(path),
        id,
        kind: file.vehicle.kind.unwrap_or_else(|| "vehicle".into()),
        deployment: Deployment { channel, profile },
        target: file.target,
        labels: file.labels,
        components: file
            .components
            .into_iter()
            .map(component_to_model)
            .collect(),
        disabled: file.disabled,
        schema: "current-vehicle-json".into(),
        source_path: path.to_string_lossy().to_string(),
    }
}

fn component_to_model(component: CurrentComponent) -> ComponentConfig {
    ComponentConfig {
        path: component.path,
        kind: component.kind,
        version: component.version,
        update_mode: component.update_mode,
        target: component.target,
        parts: component
            .parts
            .into_iter()
            .map(|part| PartConfig {
                id: part.id,
                kind: part.kind,
                source: part.source,
            })
            .collect(),
    }
}

fn from_model(config: &VehicleConfig) -> CurrentVehicleFile {
    CurrentVehicleFile {
        comment: Some("Managed by SUMO Config GUI".into()),
        vehicle: CurrentVehicle {
            id: Some(config.id.clone()),
            tag: Some(config.id.clone()),
            kind: Some(config.kind.clone()),
            version: None,
        },
        deployment: Some(config.deployment.clone()),
        target: config.target.clone(),
        labels: config.labels.clone(),
        components: config
            .components
            .iter()
            .map(|component| CurrentComponent {
                path: component.path.clone(),
                kind: component.kind.clone(),
                version: component.version.clone(),
                update_mode: component.update_mode.clone(),
                target: component.target.clone(),
                parts: component
                    .parts
                    .iter()
                    .map(|part| CurrentPart {
                        id: part.id.clone(),
                        kind: part.kind.clone(),
                        source: part.source.clone(),
                    })
                    .collect(),
            })
            .collect(),
        disabled: config.disabled,
    }
}

fn infer_channel(path: &Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("default")
        .to_string()
}

fn default_clone_channel(source: &VehicleConfig, new_id: &str) -> String {
    format!("{}-{}", source.deployment.channel, sanitize_segment(new_id))
}

fn clone_path_for_channel(source_path: &str, channel: &str, new_id: &str) -> PathBuf {
    let source = Path::new(source_path);
    let sanitized_channel = sanitize_segment(channel);
    if source.file_name().and_then(|n| n.to_str()) == Some("vehicle.json") {
        if let Some(channel_dir) = source.parent() {
            if let Some(channels_dir) = channel_dir.parent() {
                return channels_dir.join(sanitized_channel).join("vehicle.json");
            }
        }
    }
    source
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{}-vehicle.json", sanitize_segment(new_id)))
}
