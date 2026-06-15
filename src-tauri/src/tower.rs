use crate::model::{
    ComponentConfig, Deployment, LinkStatus, PartConfig, TargetTypeConfig, Tower1Config,
    TowerLinkage, VehicleConfig,
};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct ChannelState {
    name: String,
    #[serde(default)]
    vehicle: Option<VehicleRef>,
    #[serde(default)]
    target_release: Option<VehicleRef>,
    #[serde(default)]
    config_snapshot: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct VehicleRef {
    tag: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct TowerTree {
    #[serde(default)]
    entities: BTreeMap<String, TowerEntity>,
}

#[derive(Debug, Deserialize)]
struct TowerEntity {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    update_mode: Option<Value>,
    #[serde(default)]
    parts: Vec<TowerPart>,
}

#[derive(Debug, Deserialize)]
struct TowerPart {
    id: String,
    #[serde(default)]
    kind: String,
    content: Value,
}

#[derive(Debug, Deserialize)]
struct Tower1Device {
    id: String,
    #[serde(default)]
    model: Option<String>,
    status: String,
    #[serde(default)]
    cert_serial: Option<String>,
    #[serde(default)]
    cert_not_after: Option<String>,
    #[serde(default)]
    cert_fingerprint: Option<String>,
}

pub async fn list_tower1_configs(tower1_url: Option<String>) -> Result<Vec<Tower1Config>, String> {
    let Some(base_url) = clean_url(tower1_url) else {
        return Ok(Vec::new());
    };

    let devices = http_client()
        .get(format!("{base_url}/devices"))
        .send()
        .await
        .map_err(|err| format!("Tower 1 unavailable: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Tower 1 device list unavailable: {err}"))?
        .json::<Vec<Tower1Device>>()
        .await
        .map_err(|err| format!("Tower 1 device list is not valid JSON: {err}"))?;

    Ok(devices
        .into_iter()
        .map(|device| Tower1Config {
            id: device.id,
            model: device.model,
            status: device.status,
            cert_serial: device.cert_serial,
            cert_not_after: device.cert_not_after,
            cert_fingerprint: device.cert_fingerprint,
        })
        .collect())
}

pub async fn list_tower_targets(tower2_url: Option<String>) -> Result<Vec<VehicleConfig>, String> {
    let Some(base_url) = clean_url(tower2_url) else {
        return Ok(Vec::new());
    };

    let client = http_client();
    let channels_url = format!("{base_url}/admin/channels");
    let channels = client
        .get(channels_url)
        .send()
        .await
        .map_err(|err| format!("Tower 2 unavailable: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Tower 2 channel list unavailable: {err}"))?
        .json::<Vec<ChannelState>>()
        .await
        .map_err(|err| format!("Tower 2 channel list is not valid JSON: {err}"))?;

    let mut configs = Vec::new();
    for channel in channels {
        let Some(vehicle) = channel.target_release.or(channel.vehicle) else {
            continue;
        };
        let encoded = urlencoding::encode(&channel.name);
        let tree_url = format!("{base_url}/channels/{encoded}/tree");
        let tree = client
            .get(tree_url)
            .send()
            .await
            .map_err(|err| format!("Tower 2 tree unavailable for {}: {err}", channel.name))?
            .error_for_status()
            .map_err(|err| format!("Tower 2 tree unavailable for {}: {err}", channel.name))?
            .json::<TowerTree>()
            .await
            .map_err(|err| format!("Tower 2 tree for {} is not valid JSON: {err}", channel.name))?;
        configs.push(tower_config_from_release(
            &base_url,
            &channel.name,
            vehicle,
            channel.config_snapshot,
            tree,
        ));
    }

    Ok(configs)
}

pub async fn check_tower_linkage(
    config: &VehicleConfig,
    tower2_url: Option<String>,
) -> TowerLinkage {
    let tower2_channel = check_tower2_channel(
        &config.deployment.channel,
        &config.target_type.name,
        &config.deployment.profile,
        tower2_url,
    )
    .await;
    TowerLinkage { tower2_channel }
}

async fn check_tower2_channel(
    channel: &str,
    target_type: &str,
    profile: &str,
    base_url: Option<String>,
) -> LinkStatus {
    let Some(base_url) = clean_url(base_url) else {
        return LinkStatus::skipped("Tower 2 URL not configured");
    };
    if channel.trim().is_empty() {
        return LinkStatus::missing("deployment.channel is empty");
    }

    let client = http_client();
    let candidates = channel_candidates(channel, target_type, profile);
    for candidate in &candidates {
        let encoded = urlencoding::encode(candidate);
        let url = format!("{}/channels/{}/tree", base_url, encoded);
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let has_tree = resp
                    .json::<Value>()
                    .await
                    .map(|v| !v.is_null())
                    .unwrap_or(true);
                if has_tree {
                    return LinkStatus::ok(format!(
                        "Tower 2 has target release tree for {candidate}"
                    ));
                }
            }
            Ok(resp) if resp.status() == StatusCode::NOT_FOUND => continue,
            Ok(resp) => {
                return LinkStatus::unavailable(format!("Tower 2 returned HTTP {}", resp.status()))
            }
            Err(err) => return LinkStatus::unavailable(format!("Tower 2 unavailable: {err}")),
        }
    }

    LinkStatus::missing(format!(
        "Tower 2 target release pointer not found ({})",
        candidates.join(", ")
    ))
}

fn tower_config_from_release(
    base_url: &str,
    channel_name: &str,
    vehicle: VehicleRef,
    config_snapshot: Option<Value>,
    tree: TowerTree,
) -> VehicleConfig {
    let (channel, target_type, profile) = split_channel_name(channel_name, &vehicle.tag);
    let mut target = BTreeMap::new();
    target.insert("release_version".into(), Value::String(vehicle.version));
    target.insert(
        "channel_name".into(),
        Value::String(channel_name.to_string()),
    );

    VehicleConfig {
        key: format!("tower2:{channel_name}"),
        id: String::new(),
        kind: "target-release".into(),
        target_type: TargetTypeConfig {
            name: target_type,
            kind: "target".into(),
            description: Some("Read from Tower 2 target release".into()),
        },
        deployment: Deployment { channel, profile },
        target,
        labels: BTreeMap::new(),
        config_snapshot,
        components: tree
            .entities
            .into_iter()
            .map(|(path, entity)| tower_component(path, entity))
            .collect(),
        disabled: false,
        schema: "tower2-release".into(),
        source_path: format!(
            "{base_url}/channels/{}/tree",
            urlencoding::encode(channel_name)
        ),
    }
}

fn tower_component(path: String, entity: TowerEntity) -> ComponentConfig {
    ComponentConfig {
        parent_path: path.rsplit_once('/').map(|(parent, _)| parent.to_string()),
        path,
        kind: entity.kind,
        version: entity.version,
        update_mode: entity.update_mode.map(value_to_string),
        target: BTreeMap::new(),
        parts: entity
            .parts
            .into_iter()
            .map(|part| PartConfig {
                id: part.id,
                kind: part.kind,
                source: Some(value_to_string(part.content)),
            })
            .collect(),
    }
}

fn split_channel_name(name: &str, release_tag: &str) -> (String, String, String) {
    let parts = name.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        [channel, target_type, profile, ..] => {
            ((*channel).into(), (*target_type).into(), (*profile).into())
        }
        [channel, profile] => ((*channel).into(), release_tag.into(), (*profile).into()),
        [channel] => ((*channel).into(), release_tag.into(), "default".into()),
        [] => ("default".into(), release_tag.into(), "default".into()),
    }
}

fn value_to_string(value: Value) -> String {
    match value {
        Value::String(value) => value,
        other => other.to_string(),
    }
}

fn clean_url(value: Option<String>) -> Option<String> {
    value
        .map(|url| url.trim().trim_end_matches('/').to_string())
        .filter(|url| !url.is_empty())
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn channel_candidates(channel: &str, target_type: &str, profile: &str) -> Vec<String> {
    let channel = channel.trim();
    let target_type = target_type.trim();
    let profile = profile.trim();
    if channel.contains('/') {
        return vec![channel.to_string()];
    }

    let mut candidates = Vec::new();
    if !target_type.is_empty() && !profile.is_empty() && profile != "default" {
        candidates.push(format!("{channel}/{target_type}/{profile}"));
    }
    if !profile.is_empty() && profile != "default" {
        candidates.push(format!("{channel}/{profile}"));
    }
    candidates.push(channel.to_string());
    candidates.dedup();
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_state_prefers_target_release_over_legacy_vehicle() {
        let state: ChannelState = serde_json::from_value(serde_json::json!({
            "name": "bleeding",
            "vehicle": {
                "id": 1,
                "tag": "legacy-tag",
                "version": "1.0.0-dev.1",
                "created_at": "now"
            },
            "target_release": {
                "id": 2,
                "tag": "target-tag",
                "version": "2.0.0-dev.1",
                "created_at": "later"
            },
            "config_snapshot": {"schema": "sumo-target-config/v1"}
        }))
        .unwrap();

        let release = state.target_release.or(state.vehicle).unwrap();
        assert_eq!(release.tag, "target-tag");
        assert_eq!(release.version, "2.0.0-dev.1");
        assert_eq!(
            state.config_snapshot.unwrap()["schema"],
            "sumo-target-config/v1"
        );
    }

    #[test]
    fn default_profile_checks_plain_channel() {
        assert_eq!(
            channel_candidates("bleeding", "managed-cvc-rig", "default"),
            vec!["bleeding"]
        );
    }

    #[test]
    fn named_profile_checks_composite_first() {
        assert_eq!(
            channel_candidates("bleeding", "managed-cvc-rig", "test"),
            vec!["bleeding/managed-cvc-rig/test", "bleeding/test", "bleeding"]
        );
    }

    #[test]
    fn composite_channel_splits_into_release_selection() {
        assert_eq!(
            split_channel_name("bleeding/s32g3-qnx71-hpc/integration-test", "ignored"),
            (
                "bleeding".into(),
                "s32g3-qnx71-hpc".into(),
                "integration-test".into()
            )
        );
    }
}
