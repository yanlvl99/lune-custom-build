//! GitHub registry client for fetching package manifests.

use lune_utils::InstallError;
use serde::Deserialize;

const DEFAULT_REGISTRY: &str =
    "https://raw.githubusercontent.com/yanlvl99/lune-custom-build/main/manifest";

/// Registry client for fetching package metadata.
pub struct RegistryClient {
    base_url: String,
    client: reqwest::Client,
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_REGISTRY.to_owned(),
            client: reqwest::Client::new(),
        }
    }
}

impl RegistryClient {
    /// Create client with custom registry URL.
    #[must_use]
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            base_url: url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Fetch package manifest from registry.
    pub async fn fetch_manifest(&self, source: &str) -> Result<PackageManifest, InstallError> {
        // Parse source: "github:user/repo" -> fetch from registry
        let package_name = source
            .strip_prefix("github:")
            .map(|s| s.replace('/', "_"))
            .unwrap_or_else(|| source.to_owned());

        let url = format!("{}/{}.json", self.base_url, package_name);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(InstallError::from)?;

        if !response.status().is_success() {
            return Err(InstallError::PackageNotFound { name: package_name });
        }

        response
            .json::<PackageManifest>()
            .await
            .map_err(|e| InstallError::RegistryFetchFailed(e.to_string()))
    }
}

/// Package manifest from registry.
#[derive(Debug, Deserialize)]
pub struct PackageManifest {
    /// Package name.
    pub name: String,

    /// Repository URL.
    pub repository: String,

    /// Available versions with their git tags.
    pub versions: Vec<VersionEntry>,

    /// Package description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Single version entry in manifest.
#[derive(Debug, Deserialize)]
pub struct VersionEntry {
    /// Semantic version.
    pub version: String,

    /// Git tag for this version.
    pub tag: String,

    /// SHA256 checksum (optional).
    #[serde(default)]
    pub checksum: Option<String>,
}
