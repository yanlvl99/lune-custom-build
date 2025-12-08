//! Semantic version resolution for packages.

use lune_utils::InstallError;
use semver::{Version, VersionReq};

use crate::registry::{PackageManifest, VersionEntry};

/// Resolved package ready for installation.
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub tag: String,
    pub clone_url: String,
}

/// Package version resolver.
pub struct PackageResolver;

impl PackageResolver {
    /// Find the best matching version for a constraint.
    pub fn resolve(
        manifest: &PackageManifest,
        constraint: &str,
    ) -> Result<ResolvedPackage, InstallError> {
        let req = VersionReq::parse(constraint).map_err(|e| InstallError::InvalidConfig {
            path: String::new(),
            reason: format!("Invalid version constraint '{constraint}': {e}"),
        })?;

        // Find all matching versions
        let mut matching: Vec<(&VersionEntry, Version)> = manifest
            .versions
            .iter()
            .filter_map(|entry| {
                let v = entry.version.trim_start_matches('v');
                Version::parse(v).ok().filter(|ver| req.matches(ver)).map(|ver| (entry, ver))
            })
            .collect();

        // Sort by version descending (highest first)
        matching.sort_by(|a, b| b.1.cmp(&a.1));

        let best = matching.first().ok_or_else(|| InstallError::NoCompatibleVersion {
            package: manifest.name.clone(),
            constraint: constraint.to_owned(),
        })?;

        Ok(ResolvedPackage {
            name: manifest.name.clone(),
            version: best.0.version.clone(),
            tag: best.0.tag.clone(),
            clone_url: manifest.repository.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_caret_version() {
        let manifest = PackageManifest {
            name: "test".to_owned(),
            repository: "https://github.com/test/test".to_owned(),
            versions: vec![
                VersionEntry { version: "1.0.0".to_owned(), tag: "v1.0.0".to_owned(), checksum: None },
                VersionEntry { version: "1.1.0".to_owned(), tag: "v1.1.0".to_owned(), checksum: None },
                VersionEntry { version: "2.0.0".to_owned(), tag: "v2.0.0".to_owned(), checksum: None },
            ],
            description: None,
        };

        let resolved = PackageResolver::resolve(&manifest, "^1.0.0").unwrap();
        assert_eq!(resolved.version, "1.1.0"); // Highest 1.x
    }
}
