use crate::model::{ComponentConfig, Deployment, PartConfig, TargetTypeConfig, VehicleConfig};
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
    target_type: ProfileTargetType,
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    id: String,
    #[serde(default)]
    kind: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfileTargetType {
    #[serde(default)]
    name: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    description: Option<String>,
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
    #[serde(default)]
    workloads: YamlValue,
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
        target_type: Option<&str>,
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
        if let Some(target_type) = target_type {
            clone.target_type.name = target_type.to_string();
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
    let vehicle_kind = if file.vehicle.kind.is_empty() {
        "vehicle".into()
    } else {
        file.vehicle.kind
    };
    let target_type_name = file.target_type.name;
    let target_type_kind = if file.target_type.kind.is_empty() {
        vehicle_kind.clone()
    } else {
        file.target_type.kind
    };

    VehicleConfig {
        key: path_key(path),
        id: String::new(),
        kind: vehicle_kind,
        target_type: TargetTypeConfig {
            name: target_type_name,
            kind: target_type_kind,
            description: file.target_type.description,
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
        config_snapshot: None,
        components: parse_components(file.components),
        disabled: file.disabled,
        schema: "profile-yaml".into(),
        source_path: path.to_string_lossy().to_string(),
    }
}

fn from_model(config: &VehicleConfig) -> ProfileYamlFile {
    let mut components = Mapping::new();
    let mut top_level = Vec::new();
    let mut workloads_by_parent: BTreeMap<String, Vec<&ComponentConfig>> = BTreeMap::new();

    for component in &config.components {
        if let Some(parent_path) = component_parent_path(component) {
            workloads_by_parent
                .entry(parent_path)
                .or_default()
                .push(component);
        } else {
            top_level.push(component);
        }
    }

    for component in top_level {
        let mut workloads = Mapping::new();
        if let Some(children) = workloads_by_parent.remove(&component.path) {
            for child in children {
                workloads.insert(
                    YamlValue::String(workload_name(&component.path, &child.path)),
                    component_to_yaml_value(child, Mapping::new()),
                );
            }
        }
        components.insert(
            YamlValue::String(component.path.clone()),
            component_to_yaml_value(component, workloads),
        );
    }

    // Keep orphaned workload-shaped components editable instead of dropping them.
    for children in workloads_by_parent.into_values() {
        for child in children {
            components.insert(
                YamlValue::String(child.path.clone()),
                component_to_yaml_value(child, Mapping::new()),
            );
        }
    }

    ProfileYamlFile {
        vehicle: ProfileVehicle {
            id: String::new(),
            kind: config.kind.clone(),
        },
        target_type: ProfileTargetType {
            name: config.target_type.name.clone(),
            kind: config.target_type.kind.clone(),
            description: config.target_type.description.clone(),
        },
        target: config.target.clone(),
        labels: config.labels.clone(),
        deployment: config.deployment.clone(),
        components: YamlValue::Mapping(components),
        disabled: config.disabled,
    }
}

fn component_to_yaml_value(component: &ComponentConfig, workloads: Mapping) -> YamlValue {
    let mut map = Mapping::new();

    if !component.kind.is_empty() {
        map.insert(
            YamlValue::String("kind".into()),
            YamlValue::String(component.kind.clone()),
        );
    }
    if let Some(version) = &component.version {
        map.insert(
            YamlValue::String("version".into()),
            YamlValue::String(version.clone()),
        );
    }
    if let Some(update_mode) = &component.update_mode {
        map.insert(
            YamlValue::String("update_mode".into()),
            YamlValue::String(update_mode.clone()),
        );
    }

    let mut target = component.target.clone();
    target.remove("parent");
    if !target.is_empty() {
        map.insert(
            YamlValue::String("target".into()),
            serde_yaml::to_value(target).unwrap_or(YamlValue::Mapping(Mapping::new())),
        );
    }

    let parts = parts_to_yaml_mapping(&component.parts);
    if !parts.is_empty() {
        map.insert(YamlValue::String("parts".into()), YamlValue::Mapping(parts));
    }
    if !workloads.is_empty() {
        map.insert(
            YamlValue::String("workloads".into()),
            YamlValue::Mapping(workloads),
        );
    }

    YamlValue::Mapping(map)
}

fn parts_to_yaml_mapping(parts: &[PartConfig]) -> Mapping {
    let mut out = Mapping::new();
    for part in parts {
        let source = part.source.clone().unwrap_or_default();
        if part.kind.is_empty() || part.kind == "file" {
            out.insert(
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
            out.insert(
                YamlValue::String(part.id.clone()),
                YamlValue::Mapping(part_map),
            );
        }
    }
    out
}

fn component_parent_path(component: &ComponentConfig) -> Option<String> {
    component
        .parent_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            component
                .target
                .get("parent")
                .and_then(JsonValue::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned)
        })
}

fn workload_name(parent_path: &str, child_path: &str) -> String {
    child_path
        .strip_prefix(parent_path)
        .and_then(|suffix| suffix.strip_prefix('/'))
        .filter(|suffix| !suffix.is_empty())
        .unwrap_or(child_path)
        .to_string()
}

fn parse_components(value: YamlValue) -> Vec<ComponentConfig> {
    let mut out = Vec::new();
    match value {
        YamlValue::Mapping(map) => {
            for (key, value) in map {
                let path = key.as_str().unwrap_or_default().to_string();
                if let Ok(mut component) = serde_yaml::from_value::<ProfileComponent>(value) {
                    component.path = Some(path);
                    push_component_and_workloads(component, &mut out);
                }
            }
        }
        YamlValue::Sequence(items) => {
            for value in items {
                if let Ok(component) = serde_yaml::from_value::<ProfileComponent>(value) {
                    push_component_and_workloads(component, &mut out);
                }
            }
        }
        _ => {}
    }
    out
}

fn push_component_and_workloads(component: ProfileComponent, out: &mut Vec<ComponentConfig>) {
    let parent_path = component.path.clone().unwrap_or_else(|| "component".into());
    let workloads = component.workloads.clone();
    out.push(component_to_model(component, None));

    if let YamlValue::Mapping(map) = workloads {
        for (key, value) in map {
            let workload_name = key.as_str().unwrap_or_default();
            if workload_name.is_empty() {
                continue;
            }
            if let Ok(mut workload) = serde_yaml::from_value::<ProfileComponent>(value) {
                workload.path = Some(format!("{parent_path}/{workload_name}"));
                out.push(component_to_model(workload, Some(&parent_path)));
            }
        }
    }
}

fn component_to_model(component: ProfileComponent, parent_path: Option<&str>) -> ComponentConfig {
    ComponentConfig {
        path: component.path.unwrap_or_else(|| "component".into()),
        parent_path: parent_path.map(ToOwned::to_owned),
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_preserves_workloads_as_nested_yaml_children() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("truck.yaml");
        fs::write(
            &path,
            r#"vehicle:
  id: truck-002
  kind: truck
target_type:
  name: s32g3-qnx71-hpc
  kind: truck
deployment:
  channel: bleeding
  profile: integration-test
components:
  hpc1:
    kind: high-performance-ecu
    update_mode: banked
    parts:
      host_kernel: hpc1/host/boot.ifs
    workloads:
      vm1:
        kind: vm
        update_mode: banked
        parts:
          rootfs: hpc1/vm1/rootfs.img
"#,
        )
        .unwrap();

        let adapter = ProfileYamlAdapter;
        let config = adapter.load(&path).unwrap();
        let workload = config
            .components
            .iter()
            .find(|component| component.path == "hpc1/vm1")
            .unwrap();
        assert_eq!(workload.parent_path.as_deref(), Some("hpc1"));

        adapter.save(&config).unwrap();
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("workloads:"));
        assert!(saved.contains("vm1:"));
        assert!(!saved.contains("  id: truck-002"));
        assert!(!saved.contains("hpc1/vm1:"));

        let reloaded = adapter.load(&path).unwrap();
        let reloaded_workload = reloaded
            .components
            .iter()
            .find(|component| component.path == "hpc1/vm1")
            .unwrap();
        assert_eq!(reloaded_workload.parent_path.as_deref(), Some("hpc1"));
    }
}
