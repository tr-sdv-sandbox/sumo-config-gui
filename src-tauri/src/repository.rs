use crate::adapters;
use crate::model::{CloneOptions, CommandResponse, ValidationResult, VehicleConfig};
use crate::schema::{validation_errors, ConfigError, ConfigResult, SchemaRegistry};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct VehicleRepository {
    root: PathBuf,
    registry: SchemaRegistry,
}

impl VehicleRepository {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            registry: SchemaRegistry::new(adapters::default_adapters()),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list(&self) -> ConfigResult<Vec<VehicleConfig>> {
        let mut configs = Vec::new();
        if !self.root.exists() {
            return Ok(configs);
        }

        for entry in WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !is_ignored_dir(entry.path()))
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.is_file() || self.registry.adapter_for_path(path).is_none() {
                continue;
            }
            if let Ok(config) = self.registry.load(path) {
                configs.push(config);
            }
        }
        configs.sort_by(|a, b| {
            a.id.cmp(&b.id)
                .then_with(|| a.source_path.cmp(&b.source_path))
        });
        Ok(configs)
    }

    pub fn get(&self, key: &str) -> ConfigResult<VehicleConfig> {
        self.list()?
            .into_iter()
            .find(|config| config.key == key || config.source_path == key)
            .ok_or_else(|| ConfigError::NotFound(key.into()))
    }

    pub fn save(&self, config: &VehicleConfig) -> ConfigResult<CommandResponse<VehicleConfig>> {
        let validation = validate(config);
        if !validation.valid {
            return Err(ConfigError::Validation(validation.errors.join(", ")));
        }
        let adapter = self.registry.adapter_for_config(config)?;
        let saved = adapter.save(config)?;
        Ok(CommandResponse {
            value: saved,
            validation,
        })
    }

    pub fn clone_from(
        &self,
        source_key: &str,
        options: &CloneOptions,
    ) -> ConfigResult<CommandResponse<VehicleConfig>> {
        let source = self.get(source_key)?;
        let adapter = self.registry.adapter_for_config(&source)?;
        let cloned = adapter.clone_from(
            &source,
            &options.new_id,
            options.channel.as_deref(),
            options.profile.as_deref(),
        )?;
        let validation = validate(&cloned);
        Ok(CommandResponse {
            value: cloned,
            validation,
        })
    }

    pub fn disable(&self, key: &str) -> ConfigResult<CommandResponse<VehicleConfig>> {
        let source = self.get(key)?;
        let adapter = self.registry.adapter_for_config(&source)?;
        let disabled = adapter.disable(&source)?;
        let validation = validate(&disabled);
        Ok(CommandResponse {
            value: disabled,
            validation,
        })
    }
}

pub fn validate(config: &VehicleConfig) -> ValidationResult {
    let (errors, warnings) = validation_errors(config);
    ValidationResult::with_errors(errors, warnings)
}

pub fn default_root() -> PathBuf {
    if let Ok(env) = std::env::var("SUMO_CONFIG_GUI_ROOT") {
        return PathBuf::from(env);
    }

    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for ancestor in current.ancestors() {
        let candidate = ancestor.join("examples/managed-cvc-tower");
        if candidate.exists() {
            return candidate;
        }
    }
    current
}

pub fn resolve_root(root: Option<String>) -> PathBuf {
    root.filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_root)
}

fn is_ignored_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    matches!(name, ".git" | "node_modules" | "target" | "dist")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn validates_duplicate_component_and_part_ids() {
        let config = VehicleConfig {
            id: "truck-1".into(),
            deployment: crate::model::Deployment {
                channel: "bleeding".into(),
                profile: "test".into(),
            },
            components: vec![
                crate::model::ComponentConfig {
                    path: "hpc1".into(),
                    parts: vec![
                        crate::model::PartConfig {
                            id: "kernel".into(),
                            kind: "file".into(),
                            source: None,
                        },
                        crate::model::PartConfig {
                            id: "kernel".into(),
                            kind: "file".into(),
                            source: None,
                        },
                    ],
                    ..Default::default()
                },
                crate::model::ComponentConfig {
                    path: "hpc1".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let result = validate(&config);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("duplicate component path")));
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("duplicate part id")));
    }

    #[test]
    fn repository_lists_current_json_and_profile_yaml() {
        let temp = tempdir().unwrap();
        let channel = temp.path().join("channels/bleeding");
        fs::create_dir_all(&channel).unwrap();
        fs::write(
            channel.join("vehicle.json"),
            r#"{
              "vehicle": { "tag": "managed-cvc", "kind": "truck" },
              "components": [ { "path": "vm1", "kind": "bank", "parts": [ { "id": "kernel", "kind": "file", "source": "vm1/kernel" } ] } ]
            }"#,
        )
        .unwrap();
        fs::write(
            temp.path().join("truck-002.yaml"),
            r#"vehicle:
  id: truck-002
  kind: truck
deployment:
  channel: bleeding
  profile: test
components:
  hpc1:
    kind: high-performance-ecu
    parts:
      kernel: hpc1/kernel
"#,
        )
        .unwrap();

        let repo = VehicleRepository::new(temp.path());
        let list = repo.list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|c| c.schema == "current-vehicle-json"));
        assert!(list.iter().any(|c| c.schema == "profile-yaml"));
    }

    #[test]
    fn clone_and_disable_current_json() {
        let temp = tempdir().unwrap();
        let channel = temp.path().join("channels/bleeding");
        fs::create_dir_all(&channel).unwrap();
        fs::write(
            channel.join("vehicle.json"),
            r#"{
              "vehicle": { "tag": "truck-001" },
              "deployment": { "channel": "bleeding", "profile": "test" },
              "components": [ { "path": "hpc1", "parts": [ { "id": "kernel" } ] } ]
            }"#,
        )
        .unwrap();

        let repo = VehicleRepository::new(temp.path());
        let source = repo.list().unwrap().pop().unwrap();
        let cloned = repo
            .clone_from(
                &source.key,
                &CloneOptions {
                    new_id: "truck-002".into(),
                    channel: Some("bleeding/truck-002".into()),
                    profile: Some("test".into()),
                },
            )
            .unwrap()
            .value;
        assert_eq!(cloned.id, "truck-002");
        assert!(Path::new(&cloned.source_path).exists());

        let disabled = repo.disable(&cloned.key).unwrap().value;
        assert!(disabled.disabled);
        assert!(fs::read_to_string(disabled.source_path)
            .unwrap()
            .contains("\"disabled\": true"));
    }
}
