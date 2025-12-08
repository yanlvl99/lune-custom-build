//! Lune Package Installer
//!
//! Transactional package installation from GitHub registry.

mod config;
mod git;
mod luaurc;
mod registry;
mod resolver;

pub use config::{LuneConfig, PackageSpec};
pub use registry::RegistryClient;
pub use resolver::PackageResolver;

use lune_utils::{AbsolutePath, InstallError};

/// Main installer orchestrator.
pub struct PackageInstaller {
    config_path: AbsolutePath,
    packages_dir: AbsolutePath,
    registry: RegistryClient,
}

impl PackageInstaller {
    /// Create installer from current directory.
    pub fn from_cwd() -> Result<Self, InstallError> {
        let cwd = std::env::current_dir().map_err(InstallError::Io)?;
        let config_path = AbsolutePath::new(cwd.join("lune.config.json")).map_err(|e| {
            InstallError::InvalidConfig {
                path: "lune.config.json".to_owned(),
                reason: e.to_string(),
            }
        })?;
        let packages_dir =
            AbsolutePath::new(cwd.join("Packages")).map_err(|e| InstallError::InvalidConfig {
                path: "Packages".to_owned(),
                reason: e.to_string(),
            })?;

        Ok(Self {
            config_path,
            packages_dir,
            registry: RegistryClient::default(),
        })
    }

    /// Install all packages from config.
    pub async fn install(&self, force: bool) -> Result<InstalledPackages, InstallError> {
        let config = self.load_config().await?;
        let mut installed = InstalledPackages::default();
        let mut rollback_paths: Vec<AbsolutePath> = Vec::new();

        for (name, spec) in &config.packages {
            match self
                .install_package(name, spec, force, &mut rollback_paths)
                .await
            {
                Ok(path) => {
                    installed.packages.push(InstalledPackage {
                        name: name.clone(),
                        version: spec.version.clone(),
                        path,
                    });
                }
                Err(e) => {
                    // Rollback on failure
                    self.rollback(&rollback_paths).await;
                    return Err(InstallError::TransactionRollback {
                        reason: format!("Failed to install {name}: {e}"),
                    });
                }
            }
        }

        self.generate_luaurc(&installed).await?;
        Ok(installed)
    }

    async fn load_config(&self) -> Result<LuneConfig, InstallError> {
        let content = async_fs::read_to_string(self.config_path.as_path())
            .await
            .map_err(InstallError::Io)?;

        serde_json::from_str(&content).map_err(|e| InstallError::InvalidConfig {
            path: self.config_path.to_string(),
            reason: e.to_string(),
        })
    }

    async fn install_package(
        &self,
        name: &str,
        spec: &PackageSpec,
        force: bool,
        rollback_paths: &mut Vec<AbsolutePath>,
    ) -> Result<AbsolutePath, InstallError> {
        let target_dir = self.packages_dir.join(name);

        // Skip if exists and not forcing
        if !force && async_fs::metadata(target_dir.as_path()).await.is_ok() {
            return Ok(target_dir);
        }

        // Fetch from registry and resolve version
        let manifest = self.registry.fetch_manifest(&spec.source).await?;
        let resolved = PackageResolver::resolve(&manifest, &spec.version)?;

        // Clone repository
        git::clone_shallow(&resolved.clone_url, &target_dir, &resolved.tag)?;
        rollback_paths.push(target_dir.clone());

        Ok(target_dir)
    }

    async fn rollback(&self, paths: &[AbsolutePath]) {
        for path in paths {
            let _ = async_fs::remove_dir_all(path.as_path()).await;
        }
    }

    async fn generate_luaurc(&self, installed: &InstalledPackages) -> Result<(), InstallError> {
        luaurc::generate(&self.packages_dir, installed).await
    }
}

/// Result of installation.
#[derive(Debug, Default)]
pub struct InstalledPackages {
    pub packages: Vec<InstalledPackage>,
}

#[derive(Debug)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub path: AbsolutePath,
}
