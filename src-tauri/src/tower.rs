use crate::model::{LinkStatus, TowerLinkage, VehicleConfig};
use reqwest::StatusCode;
use serde_json::Value;
use std::time::Duration;

pub async fn check_tower_linkage(
    config: &VehicleConfig,
    tower1_url: Option<String>,
    tower2_url: Option<String>,
) -> TowerLinkage {
    let tower1_device = check_tower1_device(&config.id, tower1_url).await;
    let tower2_channel = check_tower2_channel(
        &config.deployment.channel,
        &config.deployment.profile,
        tower2_url,
    )
    .await;
    TowerLinkage {
        tower1_device,
        tower2_channel,
    }
}

async fn check_tower1_device(device_id: &str, base_url: Option<String>) -> LinkStatus {
    let Some(base_url) = clean_url(base_url) else {
        return LinkStatus::skipped("Tower 1 URL not configured");
    };
    if device_id.trim().is_empty() {
        return LinkStatus::missing("vehicle id is empty");
    }

    let client = http_client();
    let url = format!("{}/devices/{}", base_url, urlencoding::encode(device_id));
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => LinkStatus::ok("device exists in Tower 1"),
        Ok(resp) if resp.status() == StatusCode::NOT_FOUND => {
            LinkStatus::missing("device is not registered in Tower 1")
        }
        Ok(resp) => LinkStatus::unavailable(format!("Tower 1 returned HTTP {}", resp.status())),
        Err(err) => LinkStatus::unavailable(format!("Tower 1 unavailable: {err}")),
    }
}

async fn check_tower2_channel(
    channel: &str,
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
    let candidates = channel_candidates(channel, profile);
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
                    return LinkStatus::ok(format!("Tower 2 has desired tree for {candidate}"));
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
        "Tower 2 channel/profile not found ({})",
        candidates.join(", ")
    ))
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

fn channel_candidates(channel: &str, profile: &str) -> Vec<String> {
    let channel = channel.trim();
    let profile = profile.trim();
    if profile.is_empty() || profile == "default" || channel.contains('/') {
        vec![channel.to_string()]
    } else {
        vec![format!("{channel}/{profile}"), channel.to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_checks_plain_channel() {
        assert_eq!(channel_candidates("bleeding", "default"), vec!["bleeding"]);
    }

    #[test]
    fn named_profile_checks_composite_first() {
        assert_eq!(
            channel_candidates("bleeding", "test"),
            vec!["bleeding/test", "bleeding"]
        );
    }
}
