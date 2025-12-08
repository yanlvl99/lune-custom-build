//! .luaurc generation for installed packages.

use lune_utils::{AbsolutePath, InstallError};
use serde::Serialize;
use std::collections::HashMap;

use crate::InstalledPackages;

#[derive(Serialize)]
struct LuauRc {
    aliases: HashMap<String, String>,
}

/// Generate .luaurc file with package aliases.
pub async fn generate(
    packages_dir: &AbsolutePath,
    installed: &InstalledPackages,
) -> Result<(), InstallError> {
    let mut aliases = HashMap::new();

    for pkg in &installed.packages {
        // Create alias: @package-name -> ./Packages/package-name
        let alias_name = format!("@{}", pkg.name);
        let relative_path = format!("./{}", pkg.path.as_path().display());
        aliases.insert(alias_name, relative_path);
    }

    let luaurc = LuauRc { aliases };
    let json = serde_json::to_string_pretty(&luaurc)
        .map_err(|e| InstallError::InvalidConfig {
            path: ".luaurc".to_owned(),
            reason: e.to_string(),
        })?;

    let luaurc_path = packages_dir
        .as_path()
        .parent()
        .unwrap_or(packages_dir.as_path())
        .join(".luaurc");

    async_fs::write(&luaurc_path, json)
        .await
        .map_err(InstallError::Io)?;

    Ok(())
}
