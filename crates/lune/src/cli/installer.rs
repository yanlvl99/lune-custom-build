//! Package installer with git tag resolution.
//!
//! Installs packages from the central registry to ./lune_packages/

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use console::style;
use serde::{Deserialize, Serialize};

const REGISTRY_REPO: &str = "yanlvl99/lune-custom-build";
const REGISTRY_BRANCH: &str = "main";

/// Package manifest from the registry.
#[derive(Debug, Deserialize)]
struct PackageManifest {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    repository: String,
}

/// Local config file (lune.config.json).
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LuneConfig {
    #[serde(default)]
    pub packages: Vec<String>,
}

/// Alias entry for .luaurc.
#[derive(Debug, Serialize, Deserialize, Default)]
struct LuauRc {
    #[serde(default)]
    aliases: std::collections::HashMap<String, String>,
}

/// Initialize a new Lune project.
pub async fn run_init() -> Result<ExitCode> {
    println!("{}", style("Lune Project Initializer").cyan().bold());

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("lune.config.json");
    let luaurc_path = cwd.join(".luaurc");

    // Create lune.config.json
    if config_path.exists() {
        println!(
            "{} lune.config.json already exists",
            style("[SKIP]").yellow()
        );
    } else {
        let config = LuneConfig::default();
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        println!("{} Created lune.config.json", style("[OK]").green());
    }

    // Create .luaurc
    if luaurc_path.exists() {
        println!("{} .luaurc already exists", style("[SKIP]").yellow());
    } else {
        let luaurc = LuauRc::default();
        std::fs::write(&luaurc_path, serde_json::to_string_pretty(&luaurc)?)?;
        println!("{} Created .luaurc", style("[OK]").green());
    }

    // Create lune_packages directory
    let packages_dir = cwd.join("lune_packages");
    if !packages_dir.exists() {
        std::fs::create_dir_all(&packages_dir)?;
        println!("{} Created lune_packages/", style("[OK]").green());
    }

    println!("\n{} Project initialized!", style("✓").green().bold());
    println!(
        "  Use {} to install packages",
        style("lune --install <pkg>").cyan()
    );

    Ok(ExitCode::SUCCESS)
}

/// Run the installer with given packages or from config.
pub async fn run_install(packages: Vec<String>) -> Result<ExitCode> {
    println!("{}", style("Lune Package Installer").cyan().bold());

    let cwd = std::env::current_dir()?;
    let packages_dir = cwd.join("lune_packages");

    // Determine packages to install
    let packages_to_install = if packages.is_empty() {
        // Read from lune.config.json
        let config_path = cwd.join("lune.config.json");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: LuneConfig = serde_json::from_str(&content)?;
            if config.packages.is_empty() {
                println!(
                    "{} No packages in lune.config.json. Use: lune --install <pkg>",
                    style("[INFO]").blue()
                );
                return Ok(ExitCode::SUCCESS);
            }
            config.packages
        } else {
            println!(
                "{} No lune.config.json found. Run: lune --init",
                style("[WARN]").yellow()
            );
            return Ok(ExitCode::SUCCESS);
        }
    } else {
        packages
    };

    println!(
        "{} Installing {} package(s)",
        style("[1/4]").dim(),
        style(packages_to_install.len()).green()
    );

    // Create packages directory
    if !packages_dir.exists() {
        std::fs::create_dir_all(&packages_dir)?;
    }

    let mut installed: Vec<(String, PathBuf)> = Vec::new();

    for pkg_name in &packages_to_install {
        println!("  {} {}", style("→").cyan(), style(pkg_name).bold());

        match install_package(pkg_name, &packages_dir).await {
            Ok(path) => {
                println!("    {} Installed", style("✓").green());
                installed.push((pkg_name.clone(), path));
            }
            Err(e) => {
                eprintln!("    {} {}", style("✗").red(), e);
            }
        }
    }

    // Update lune.config.json
    println!("{} Updating lune.config.json", style("[3/4]").dim());
    update_config(&cwd, &packages_to_install)?;

    // Generate .luaurc with aliases
    println!("{} Generating .luaurc", style("[4/4]").dim());
    generate_luaurc(&cwd, &installed)?;

    println!(
        "\n{} {} package(s) installed!",
        style("✓").green().bold(),
        installed.len()
    );

    Ok(ExitCode::SUCCESS)
}

/// Install a single package.
async fn install_package(name: &str, packages_dir: &Path) -> Result<PathBuf> {
    // Fetch manifest from registry
    let manifest_url = format!(
        "https://raw.githubusercontent.com/{}/{}/manifest/{}.json",
        REGISTRY_REPO, REGISTRY_BRANCH, name
    );
    let manifest = fetch_manifest(&manifest_url)?;

    // Resolve latest git tag
    let tag = resolve_latest_tag(&manifest.repository)?;
    println!("    {} tag: {}", style("↳").dim(), style(&tag).yellow());

    // Clone to lune_packages/{name}
    let target_dir = packages_dir.join(name);
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
    }

    clone_repository(&manifest.repository, &tag, &target_dir)?;

    Ok(target_dir)
}

/// Fetch package manifest from registry.
fn fetch_manifest(url: &str) -> Result<PackageManifest> {
    let resp = reqwest::blocking::get(url)
        .with_context(|| format!("Failed to fetch manifest from {url}"))?;

    if !resp.status().is_success() {
        anyhow::bail!("Package not found in registry ({})", resp.status());
    }

    resp.json::<PackageManifest>()
        .context("Failed to parse manifest")
}

/// Resolve the latest git tag using semver.
fn resolve_latest_tag(repo_url: &str) -> Result<String> {
    use git2::Repository;
    use semver::Version;

    // Create a temporary bare repo to list remote refs
    let temp_dir = std::env::temp_dir().join(format!(".lune_tag_resolve_{}", std::process::id()));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)?;
    }

    let repo = Repository::init_bare(&temp_dir)?;
    let mut remote = repo.remote_anonymous(repo_url)?;
    remote.connect(git2::Direction::Fetch)?;

    let refs = remote.list()?;
    let mut versions: Vec<(Version, String)> = refs
        .iter()
        .filter_map(|head| {
            let name = head.name();
            // Extract tag name from refs/tags/v1.0.0 or refs/tags/1.0.0
            name.strip_prefix("refs/tags/")
                .map(|tag| tag.trim_start_matches('v'))
                .and_then(|ver| Version::parse(ver).ok())
                .map(|ver| {
                    (
                        ver,
                        name.strip_prefix("refs/tags/").unwrap_or(name).to_owned(),
                    )
                })
        })
        .collect();

    // Sort by version descending
    versions.sort_by(|a, b| b.0.cmp(&a.0));

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);

    versions
        .first()
        .map(|(_, tag)| tag.clone())
        .ok_or_else(|| anyhow::anyhow!("No valid semver tags found in repository"))
}

/// Clone repository at specific tag with depth 1.
fn clone_repository(repo_url: &str, tag: &str, target: &Path) -> Result<()> {
    use git2::FetchOptions;
    use git2::build::RepoBuilder;

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.depth(1);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_opts);
    builder.branch(tag);

    builder
        .clone(repo_url, target)
        .with_context(|| format!("Failed to clone {repo_url} at tag {tag}"))?;

    Ok(())
}

/// Update lune.config.json with installed packages.
fn update_config(cwd: &Path, packages: &[String]) -> Result<()> {
    let config_path = cwd.join("lune.config.json");

    let mut config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str::<LuneConfig>(&content).unwrap_or_default()
    } else {
        LuneConfig::default()
    };

    // Add new packages
    for pkg in packages {
        if !config.packages.contains(pkg) {
            config.packages.push(pkg.clone());
        }
    }

    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

/// Generate .luaurc with package aliases.
fn generate_luaurc(cwd: &Path, installed: &[(String, PathBuf)]) -> Result<()> {
    let luaurc_path = cwd.join(".luaurc");

    // Read existing
    let mut luaurc = if luaurc_path.exists() {
        let content = std::fs::read_to_string(&luaurc_path)?;
        serde_json::from_str::<LuauRc>(&content).unwrap_or_default()
    } else {
        LuauRc::default()
    };

    // Add aliases for installed packages
    for (name, path) in installed {
        let entry = find_entry_point(path);
        let relative = pathdiff::diff_paths(&entry, cwd).unwrap_or_else(|| entry.clone());

        let alias = format!("@{}", name);
        luaurc
            .aliases
            .insert(alias, format!("./{}", relative.display()));
    }

    std::fs::write(&luaurc_path, serde_json::to_string_pretty(&luaurc)?)?;
    Ok(())
}

/// Find entry point for a package (init.luau, main.luau, lib/init.luau).
fn find_entry_point(pkg_path: &Path) -> PathBuf {
    for candidate in ["init.luau", "main.luau", "lib/init.luau", "src/init.luau"] {
        let path = pkg_path.join(candidate);
        if path.exists() {
            return path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| pkg_path.to_path_buf());
        }
    }
    pkg_path.to_path_buf()
}
