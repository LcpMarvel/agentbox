use serde::{Deserialize, Serialize};
use std::path::Path;

const REPO: &str = "LcpMarvel/agentbox";

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeInfo {
    pub current: String,
    pub latest: String,
    pub download_url: String,
    pub has_update: bool,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Detect the current platform target triple.
pub fn detect_target() -> String {
    let arch = std::env::consts::ARCH;
    let os = match std::env::consts::OS {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-gnu",
        other => other,
    };
    format!("{}-{}", arch, os)
}

/// Check GitHub for the latest release version.
pub async fn check_latest_version() -> Result<UpgradeInfo, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "agentbox-upgrade")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned status {}", resp.status()));
    }

    let release: GitHubRelease = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse release JSON: {}", e))?;

    let latest_tag = release.tag_name.trim_start_matches('v').to_string();

    let current = current_version().to_string();
    let has_update = match (
        semver::Version::parse(&current),
        semver::Version::parse(&latest_tag),
    ) {
        (Ok(cur), Ok(lat)) => lat > cur,
        _ => latest_tag != current,
    };

    let target = detect_target();
    let asset_name = format!("agentbox-{}-{}.tar.gz", release.tag_name, target);
    let download_url = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_else(|| {
            format!(
                "https://github.com/{}/releases/latest/download/{}",
                REPO, asset_name
            )
        });

    Ok(UpgradeInfo {
        current,
        latest: latest_tag,
        download_url,
        has_update,
    })
}

/// Download the release tarball and replace the current binary.
pub async fn download_and_replace(url: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "agentbox-upgrade")
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download returned status {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download body: {}", e))?;

    let decoder = flate2::read::GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(decoder);

    let tmp_dir = std::env::temp_dir().join("agentbox-upgrade");
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir)
            .map_err(|e| format!("Failed to clean temp dir: {}", e))?;
    }
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    archive
        .unpack(&tmp_dir)
        .map_err(|e| format!("Failed to extract archive: {}", e))?;

    // Find the agentbox binary in the extracted archive
    let new_binary = find_binary(&tmp_dir)?;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;

    // Atomic replace: rename new over old
    // On Unix, we can rename over the current executable if we first remove it
    let backup = current_exe.with_extension("old");
    if backup.exists() {
        let _ = std::fs::remove_file(&backup);
    }
    std::fs::rename(&current_exe, &backup)
        .map_err(|e| format!("Failed to backup current binary: {}", e))?;

    if let Err(e) = std::fs::copy(&new_binary, &current_exe) {
        // Restore backup on failure
        let _ = std::fs::rename(&backup, &current_exe);
        return Err(format!("Failed to install new binary: {}", e));
    }

    // Set executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755));
    }

    // Clean up
    let _ = std::fs::remove_file(&backup);
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(())
}

fn find_binary(dir: &Path) -> Result<std::path::PathBuf, String> {
    // Look for "agentbox" binary in the directory (possibly nested)
    for entry in walkdir(dir) {
        if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
            if name == "agentbox" {
                return Ok(entry);
            }
        }
    }
    Err("Could not find 'agentbox' binary in the downloaded archive".to_string())
}

fn walkdir(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(&path));
            } else {
                results.push(path);
            }
        }
    }
    results
}
