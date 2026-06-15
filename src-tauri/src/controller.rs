use crate::model::{
    CloneOptions, CommandResponse, LaunchConfig, Tower1Config, TowerLinkage, ValidationResult,
    VehicleConfig, VehicleSummary,
};
use crate::repository::{resolve_root, validate, VehicleRepository};
use crate::tower;

#[tauri::command]
pub fn launch_config() -> LaunchConfig {
    LaunchConfig {
        config_root: std::env::var("SUMO_CONFIG_GUI_ROOT").ok(),
        tower1_url: std::env::var("SUMO_CONFIG_GUI_TOWER1_URL")
            .unwrap_or_else(|_| "http://localhost:8080".into()),
        tower2_url: std::env::var("SUMO_CONFIG_GUI_TOWER2_URL")
            .unwrap_or_else(|_| "http://localhost:8081".into()),
    }
}

#[tauri::command]
pub async fn list_tower1_configs(tower1_url: Option<String>) -> Result<Vec<Tower1Config>, String> {
    tower::list_tower1_configs(tower1_url).await
}

#[tauri::command]
pub async fn list_vehicles(
    root: Option<String>,
    tower2_url: Option<String>,
) -> Result<Vec<VehicleSummary>, String> {
    let has_local_root = root
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let mut configs = match tower::list_tower_targets(tower2_url.clone()).await {
        Ok(configs) => configs,
        Err(err) if has_local_root => {
            eprintln!("Tower 2 target release listing failed; using local test files only: {err}");
            Vec::new()
        }
        Err(err) => return Err(err),
    };

    if has_local_root {
        let repo = VehicleRepository::new(resolve_root(root));
        configs.extend(repo.list().map_err(|e| e.to_string())?);
    }

    let mut summaries = Vec::with_capacity(configs.len());
    for config in configs {
        let mut summary = VehicleSummary::from_config(&config);
        summary.linkage = Some(tower::check_tower_linkage(&config, tower2_url.clone()).await);
        summaries.push(summary);
    }
    Ok(summaries)
}

#[tauri::command]
pub async fn get_vehicle(
    root: Option<String>,
    tower2_url: Option<String>,
    key: String,
) -> Result<VehicleConfig, String> {
    if key.starts_with("tower2:") {
        return tower::list_tower_targets(tower2_url)
            .await?
            .into_iter()
            .find(|config| config.key == key)
            .ok_or_else(|| format!("target release not found: {key}"));
    }

    let repo = VehicleRepository::new(resolve_root(root));
    repo.get(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_vehicle(
    root: Option<String>,
    vehicle: VehicleConfig,
) -> Result<CommandResponse<VehicleConfig>, String> {
    let repo = VehicleRepository::new(resolve_root(root));
    repo.save(&vehicle).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clone_vehicle(
    root: Option<String>,
    source_key: String,
    options: CloneOptions,
) -> Result<CommandResponse<VehicleConfig>, String> {
    let repo = VehicleRepository::new(resolve_root(root));
    repo.clone_from(&source_key, &options)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disable_vehicle(
    root: Option<String>,
    key: String,
) -> Result<CommandResponse<VehicleConfig>, String> {
    let repo = VehicleRepository::new(resolve_root(root));
    repo.disable(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_vehicle(vehicle: VehicleConfig) -> Result<ValidationResult, String> {
    Ok(validate(&vehicle))
}

#[tauri::command]
pub async fn check_tower_linkage(
    vehicle: VehicleConfig,
    tower2_url: Option<String>,
) -> Result<TowerLinkage, String> {
    Ok(tower::check_tower_linkage(&vehicle, tower2_url).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn list_vehicles_command_returns_summaries() {
        let temp = tempdir().unwrap();
        let channel = temp.path().join("channels/bleeding");
        fs::create_dir_all(&channel).unwrap();
        fs::write(
            channel.join("vehicle.json"),
            r#"{
              "vehicle": { "tag": "truck-001", "kind": "truck" },
              "deployment": { "channel": "bleeding", "profile": "test" },
              "components": [ { "path": "hpc1", "parts": [ { "id": "kernel" } ] } ]
            }"#,
        )
        .unwrap();

        let summaries = list_vehicles(Some(temp.path().to_string_lossy().to_string()), None)
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "truck-001");
        assert_eq!(
            summaries[0].linkage.as_ref().unwrap().tower2_channel.state,
            "skipped"
        );
    }
}
