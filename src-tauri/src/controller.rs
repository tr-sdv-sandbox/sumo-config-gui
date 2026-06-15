use crate::model::{
    CloneOptions, CommandResponse, TowerLinkage, ValidationResult, VehicleConfig, VehicleSummary,
};
use crate::repository::{resolve_root, validate, VehicleRepository};
use crate::tower;

#[tauri::command]
pub async fn list_vehicles(
    root: Option<String>,
    tower1_url: Option<String>,
    tower2_url: Option<String>,
) -> Result<Vec<VehicleSummary>, String> {
    let repo = VehicleRepository::new(resolve_root(root));
    let configs = repo.list().map_err(|e| e.to_string())?;
    let mut summaries = Vec::with_capacity(configs.len());
    for config in configs {
        let mut summary = VehicleSummary::from_config(&config);
        summary.linkage =
            Some(tower::check_tower_linkage(&config, tower1_url.clone(), tower2_url.clone()).await);
        summaries.push(summary);
    }
    Ok(summaries)
}

#[tauri::command]
pub async fn get_vehicle(root: Option<String>, key: String) -> Result<VehicleConfig, String> {
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
    tower1_url: Option<String>,
    tower2_url: Option<String>,
) -> Result<TowerLinkage, String> {
    Ok(tower::check_tower_linkage(&vehicle, tower1_url, tower2_url).await)
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

        let summaries = list_vehicles(Some(temp.path().to_string_lossy().to_string()), None, None)
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "truck-001");
        assert_eq!(
            summaries[0].linkage.as_ref().unwrap().tower1_device.state,
            "skipped"
        );
    }
}
