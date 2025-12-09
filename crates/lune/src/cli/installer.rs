//! Package installer with zip download.
//!
//! Installs packages from the central registry to ./lune_packages/

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use console::style;
use directories::UserDirs;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use lune_std::LuneStandardLibrary;

const REGISTRY_REPO: &str = "yanlvl99/lune-custom-build";
const REGISTRY_BRANCH: &str = "main";

/// Package manifest from the registry.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
struct PackageManifest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    repository: String,
}

/// Local package info (lune-pkg.json).
#[derive(Debug, Serialize, Deserialize)]
pub struct LunePkgInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub repository: String,
}

/// Package entry with optional version lock.
/// Supports both "pkg-name" and "pkg-name@1.0.0" formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "String", into = "String")]
pub struct PackageSpec {
    pub name: String,
    pub version: Option<String>,
}

impl TryFrom<String> for PackageSpec {
    type Error = std::convert::Infallible;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if let Some((name, version)) = s.split_once('@') {
            Ok(Self {
                name: name.to_string(),
                version: Some(version.to_string()),
            })
        } else {
            Ok(Self {
                name: s,
                version: None,
            })
        }
    }
}

impl From<PackageSpec> for String {
    fn from(spec: PackageSpec) -> Self {
        match spec.version {
            Some(v) => format!("{}@{}", spec.name, v),
            None => spec.name,
        }
    }
}

impl std::fmt::Display for PackageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}@{}", self.name, v),
            None => write!(f, "{}", self.name),
        }
    }
}

/// Local config file (lune.config.json).
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LuneConfig {
    #[serde(default)]
    pub packages: Vec<PackageSpec>,
}

/// Alias entry for .luaurc.
#[derive(Debug, Serialize, Deserialize, Default)]
struct LuauRc {
    #[serde(default)]
    aliases: std::collections::HashMap<String, String>,
}

/// Initialize a new Lune project.
pub fn run_init() -> Result<ExitCode> {
    println!("{}", style("Lune Project Initializer").cyan().bold());

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("lune.config.json");
    let luaurc_path = cwd.join(".luaurc");

    // Get version and user home for typedefs path
    let version = env!("CARGO_PKG_VERSION");
    let user_dirs = UserDirs::new().context("Failed to find user home directory")?;
    let typedefs_dir = user_dirs
        .home_dir()
        .join(".lune")
        .join(".typedefs")
        .join(version);
    let typedefs_path = format!("~/.lune/.typedefs/{}/", version);

    // Generate type definitions
    if !typedefs_dir.exists() {
        std::fs::create_dir_all(&typedefs_dir)?;
    }

    let mut generated_count = 0;
    for lib in LuneStandardLibrary::ALL {
        let typedef_file = typedefs_dir.join(format!("{}.luau", lib.name()));
        if !typedef_file.exists() {
            std::fs::write(&typedef_file, lib.typedefs())?;
            generated_count += 1;
        }
    }

    if generated_count > 0 {
        println!(
            "{} Generated {} type definition(s) in {}",
            style("[OK]").green(),
            generated_count,
            typedefs_dir.display()
        );
    } else {
        println!(
            "{} Type definitions already exist",
            style("[SKIP]").yellow()
        );
    }

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

    // Create .luaurc with @lune alias
    if luaurc_path.exists() {
        let content = std::fs::read_to_string(&luaurc_path)?;
        let mut luaurc = serde_json::from_str::<LuauRc>(&content).unwrap_or_default();
        if !luaurc.aliases.contains_key("lune") {
            luaurc
                .aliases
                .insert("lune".to_owned(), typedefs_path.clone());
            std::fs::write(&luaurc_path, serde_json::to_string_pretty(&luaurc)?)?;
            println!("{} Updated .luaurc with @lune alias", style("[OK]").green());
        } else {
            println!(
                "{} .luaurc already has @lune alias",
                style("[SKIP]").yellow()
            );
        }
    } else {
        let mut luaurc = LuauRc::default();
        luaurc
            .aliases
            .insert("lune".to_owned(), typedefs_path.clone());
        std::fs::write(&luaurc_path, serde_json::to_string_pretty(&luaurc)?)?;
        println!("{} Created .luaurc with @lune alias", style("[OK]").green());
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

    // Parse package specs from command line
    let specs_from_args: Vec<PackageSpec> = packages
        .into_iter()
        .filter_map(|s| PackageSpec::try_from(s).ok())
        .collect();

    // Determine packages to install
    let packages_to_install = if specs_from_args.is_empty() {
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
        specs_from_args
    };

    println!(
        "{} Installing {} package(s)",
        style("[1/4]").dim(),
        style(packages_to_install.len()).green()
    );

    if !packages_dir.exists() {
        std::fs::create_dir_all(&packages_dir)?;
    }

    let mut installed: Vec<(String, PathBuf)> = Vec::new();

    for spec in &packages_to_install {
        println!("  {} {}", style("→").cyan(), style(&spec).bold());

        match install_package_with_version(&spec.name, spec.version.as_deref(), &packages_dir).await
        {
            Ok(path) => {
                println!("    {} Installed", style("✓").green());
                installed.push((spec.name.clone(), path));
            }
            Err(e) => {
                eprintln!("    {} {}", style("✗").red(), e);
            }
        }
    }

    println!("{} Updating lune.config.json", style("[3/4]").dim());
    update_config(&cwd, &packages_to_install)?;

    println!("{} Generating .luaurc", style("[4/4]").dim());
    generate_luaurc(&cwd, &installed)?;

    println!(
        "\n{} {} package(s) installed!",
        style("✓").green().bold(),
        installed.len()
    );

    Ok(ExitCode::SUCCESS)
}

/// Update all packages to latest versions.
#[allow(clippy::unused_async)]
pub async fn run_update() -> Result<ExitCode> {
    println!("{}", style("Lune Package Updater").cyan().bold());

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("lune.config.json");

    if !config_path.exists() {
        println!(
            "{} No lune.config.json found. Run: lune --init",
            style("[WARN]").yellow()
        );
        return Ok(ExitCode::SUCCESS);
    }

    let content = std::fs::read_to_string(&config_path)?;
    let mut config: LuneConfig = serde_json::from_str(&content)?;

    if config.packages.is_empty() {
        println!("{} No packages to update", style("[INFO]").blue());
        return Ok(ExitCode::SUCCESS);
    }

    let packages_dir = cwd.join("lune_packages");
    let mut updated_count = 0;

    for spec in &mut config.packages {
        println!(
            "  {} Checking {}",
            style("→").cyan(),
            style(&spec.name).bold()
        );

        let pkg_dir = packages_dir.join(&spec.name);
        let pkg_info_path = pkg_dir.join("lune-pkg.json");

        // Get current version
        let current_version = if pkg_info_path.exists() {
            let info_content = std::fs::read_to_string(&pkg_info_path)?;
            let info: LunePkgInfo = serde_json::from_str(&info_content)?;
            Some(info.version)
        } else {
            None
        };

        // Get manifest
        let manifest_url = format!(
            "https://raw.githubusercontent.com/{}/{}/manifest/{}.json",
            REGISTRY_REPO, REGISTRY_BRANCH, spec.name
        );

        let manifest = match fetch_manifest(&manifest_url) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("    {} Failed to fetch: {}", style("✗").red(), e);
                continue;
            }
        };

        // Use locked version if specified, otherwise get latest
        let target_version = match &spec.version {
            Some(v) => v.clone(),
            None => resolve_latest_tag_via_api(&manifest.repository)?,
        };

        let needs_update = current_version.as_ref() != Some(&target_version);

        if needs_update {
            println!(
                "    {} Updating {} -> {}",
                style("↳").dim(),
                current_version.as_deref().unwrap_or("unknown"),
                style(&target_version).yellow()
            );

            if pkg_dir.exists() {
                std::fs::remove_dir_all(&pkg_dir)?;
            }

            match download_and_extract(
                &manifest.repository,
                &target_version,
                &spec.name,
                &packages_dir,
            ) {
                Ok(()) => {
                    let pkg_info = LunePkgInfo {
                        name: spec.name.clone(),
                        version: target_version.clone(),
                        description: manifest.description.clone(),
                        repository: manifest.repository.clone(),
                    };
                    let pkg_info_path = packages_dir.join(&spec.name).join("lune-pkg.json");
                    std::fs::write(&pkg_info_path, serde_json::to_string_pretty(&pkg_info)?)?;

                    // Update locked version in config
                    spec.version = Some(target_version);

                    println!("    {} Updated", style("✓").green());
                    updated_count += 1;
                }
                Err(e) => {
                    eprintln!("    {} {}", style("✗").red(), e);
                }
            }
        } else {
            println!(
                "    {} Already up to date ({})",
                style("✓").dim(),
                target_version
            );
        }
    }

    // Save updated config with locked versions
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

    // Regenerate .luaurc
    let installed: Vec<(String, PathBuf)> = config
        .packages
        .iter()
        .map(|spec| (spec.name.clone(), packages_dir.join(&spec.name)))
        .collect();
    generate_luaurc(&cwd, &installed)?;

    println!(
        "\n{} {} package(s) updated!",
        style("✓").green().bold(),
        updated_count
    );

    Ok(ExitCode::SUCCESS)
}

/// Uninstall packages (supports multiple packages).
#[allow(clippy::unused_async)]
pub async fn run_uninstall(packages: Vec<String>) -> Result<ExitCode> {
    println!("{}", style("Lune Package Uninstaller").cyan().bold());

    if packages.is_empty() {
        println!(
            "{} No packages specified. Usage: lune --uninstall <pkg1> [pkg2] ...",
            style("[WARN]").yellow()
        );
        return Ok(ExitCode::SUCCESS);
    }

    let cwd = std::env::current_dir()?;
    let packages_dir = cwd.join("lune_packages");
    let config_path = cwd.join("lune.config.json");
    let luaurc_path = cwd.join(".luaurc");

    let mut uninstalled_count = 0;

    for pkg_name in &packages {
        println!("  {} {}", style("→").cyan(), style(pkg_name).bold());

        let pkg_dir = packages_dir.join(pkg_name);
        if pkg_dir.exists() {
            std::fs::remove_dir_all(&pkg_dir)?;
            println!("    {} Removed directory", style("✓").green());
            uninstalled_count += 1;
        } else {
            println!("    {} Package not found", style("⚠").yellow());
        }
    }

    // Update lune.config.json
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let mut config: LuneConfig = serde_json::from_str(&content).unwrap_or_default();
        config.packages.retain(|p| !packages.contains(&p.name));
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        println!("{} Updated lune.config.json", style("[OK]").green());
    }

    // Update .luaurc
    if luaurc_path.exists() {
        let content = std::fs::read_to_string(&luaurc_path)?;
        let mut luaurc: LuauRc = serde_json::from_str(&content).unwrap_or_default();
        for pkg_name in &packages {
            luaurc.aliases.remove(pkg_name);
        }
        std::fs::write(&luaurc_path, serde_json::to_string_pretty(&luaurc)?)?;
        println!("{} Updated .luaurc", style("[OK]").green());
    }

    println!(
        "\n{} {} package(s) uninstalled!",
        style("✓").green().bold(),
        uninstalled_count
    );

    Ok(ExitCode::SUCCESS)
}

/// List installed packages.
pub fn run_list_packages() -> Result<ExitCode> {
    println!("{}", style("Installed Packages").cyan().bold());

    let cwd = std::env::current_dir()?;
    let packages_dir = cwd.join("lune_packages");

    if !packages_dir.exists() {
        println!("{} No packages installed", style("[INFO]").blue());
        return Ok(ExitCode::SUCCESS);
    }

    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(&packages_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let pkg_name = entry.file_name().to_string_lossy().to_string();
                let pkg_info_path = entry.path().join("lune-pkg.json");

                if pkg_info_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&pkg_info_path) {
                        if let Ok(info) = serde_json::from_str::<LunePkgInfo>(&content) {
                            println!(
                                "  {} {}@{}",
                                style("•").cyan(),
                                style(&info.name).bold(),
                                style(&info.version).dim()
                            );
                            count += 1;
                            continue;
                        }
                    }
                }
                // Fallback if no lune-pkg.json
                println!("  {} {}", style("•").cyan(), style(&pkg_name).bold());
                count += 1;
            }
        }
    }

    if count == 0 {
        println!("{} No packages installed", style("[INFO]").blue());
    } else {
        println!("\n{} {} package(s) installed", style("Total:").dim(), count);
    }

    Ok(ExitCode::SUCCESS)
}

/// Show package info.
pub fn run_package_info(name: &str) -> Result<ExitCode> {
    println!("{}", style(format!("Package: {}", name)).cyan().bold());

    let cwd = std::env::current_dir()?;
    let pkg_dir = cwd.join("lune_packages").join(name);
    let pkg_info_path = pkg_dir.join("lune-pkg.json");

    if !pkg_dir.exists() {
        println!(
            "{} Package '{}' not installed",
            style("[ERROR]").red(),
            name
        );
        return Ok(ExitCode::FAILURE);
    }

    if pkg_info_path.exists() {
        let content = std::fs::read_to_string(&pkg_info_path)?;
        let info: LunePkgInfo = serde_json::from_str(&content)?;

        println!("  {} {}", style("Name:").dim(), info.name);
        println!("  {} {}", style("Version:").dim(), info.version);
        if let Some(desc) = &info.description {
            println!("  {} {}", style("Description:").dim(), desc);
        }
        println!("  {} {}", style("Repository:").dim(), info.repository);
    } else {
        println!("  {} {}", style("Name:").dim(), name);
        println!("  {} No metadata available", style("Version:").dim());
    }

    // Show files
    println!("\n  {}:", style("Files").dim());
    if let Ok(entries) = std::fs::read_dir(&pkg_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.path().is_dir();
            if is_dir {
                println!("    {} {}/", style("📁").dim(), file_name);
            } else {
                println!("    {} {}", style("📄").dim(), file_name);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Install a single package via zip download with optional version.
#[allow(clippy::unused_async)]
async fn install_package_with_version(
    name: &str,
    version: Option<&str>,
    packages_dir: &Path,
) -> Result<PathBuf> {
    let manifest_url = format!(
        "https://raw.githubusercontent.com/{}/{}/manifest/{}.json",
        REGISTRY_REPO, REGISTRY_BRANCH, name
    );
    let manifest = fetch_manifest(&manifest_url)?;

    let tag = match version {
        Some(v) => v.to_string(),
        None => resolve_latest_tag_via_api(&manifest.repository)?,
    };
    println!("    {} tag: {}", style("↳").dim(), style(&tag).yellow());

    let target_dir = packages_dir.join(name);
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
    }

    download_and_extract(&manifest.repository, &tag, name, packages_dir)?;

    // Save lune-pkg.json
    let pkg_info = LunePkgInfo {
        name: name.to_string(),
        version: tag.clone(),
        description: manifest.description.clone(),
        repository: manifest.repository.clone(),
    };
    let pkg_info_path = target_dir.join("lune-pkg.json");
    std::fs::write(&pkg_info_path, serde_json::to_string_pretty(&pkg_info)?)?;

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

/// Resolve latest tag using GitHub API.
fn resolve_latest_tag_via_api(repo_url: &str) -> Result<String> {
    let repo_path = repo_url
        .trim_end_matches(".git")
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/");

    let api_url = format!("https://api.github.com/repos/{}/tags", repo_path);

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&api_url)
        .header("User-Agent", "lune-installer")
        .send()
        .with_context(|| format!("Failed to fetch tags from {api_url}"))?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to fetch tags ({})", resp.status());
    }

    #[derive(Deserialize)]
    struct GitHubTag {
        name: String,
    }

    let tags: Vec<GitHubTag> = resp.json()?;

    if tags.is_empty() {
        anyhow::bail!("No tags found in repository");
    }

    // Sort by semver
    use semver::Version;
    let mut versions: Vec<(Version, String)> = tags
        .iter()
        .filter_map(|t| {
            let ver_str = t.name.trim_start_matches('v');
            Version::parse(ver_str).ok().map(|v| (v, t.name.clone()))
        })
        .collect();

    versions.sort_by(|a, b| b.0.cmp(&a.0));

    versions
        .first()
        .map(|(_, tag)| tag.clone())
        .ok_or_else(|| anyhow::anyhow!("No valid semver tags found"))
}

/// Download and extract zip from GitHub releases.
fn download_and_extract(
    repo_url: &str,
    tag: &str,
    pkg_name: &str,
    packages_dir: &Path,
) -> Result<()> {
    let repo_path = repo_url
        .trim_end_matches(".git")
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/");

    let zip_url = format!(
        "https://github.com/{}/archive/refs/tags/{}.zip",
        repo_path, tag
    );

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&zip_url)
        .header("User-Agent", "lune-installer")
        .send()
        .with_context(|| format!("Failed to download {zip_url}"))?;

    if !resp.status().is_success() {
        anyhow::bail!("Failed to download zip ({})", resp.status());
    }

    let bytes = resp.bytes()?;
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;

    let target_dir = packages_dir.join(pkg_name);
    std::fs::create_dir_all(&target_dir)?;

    // Get root folder name from first entry
    let root_prefix = archive
        .by_index(0)?
        .name()
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = file.name();

        let relative_path = file_path
            .strip_prefix(&format!("{}/", root_prefix))
            .unwrap_or(file_path);

        if relative_path.is_empty() {
            continue;
        }

        let out_path = target_dir.join(relative_path);

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;
        }
    }

    Ok(())
}

/// Update lune.config.json with installed packages.
fn update_config(cwd: &Path, packages: &[PackageSpec]) -> Result<()> {
    let config_path = cwd.join("lune.config.json");

    let mut config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str::<LuneConfig>(&content).unwrap_or_default()
    } else {
        LuneConfig::default()
    };

    for pkg in packages {
        if !config.packages.iter().any(|p| p.name == pkg.name) {
            config.packages.push(pkg.clone());
        }
    }

    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

/// Generate .luaurc with package aliases.
fn generate_luaurc(cwd: &Path, installed: &[(String, PathBuf)]) -> Result<()> {
    let luaurc_path = cwd.join(".luaurc");

    let mut luaurc = if luaurc_path.exists() {
        let content = std::fs::read_to_string(&luaurc_path)?;
        serde_json::from_str::<LuauRc>(&content).unwrap_or_default()
    } else {
        LuauRc::default()
    };

    for (name, path) in installed {
        let entry = find_entry_point(path);
        let relative = pathdiff::diff_paths(&entry, cwd).unwrap_or_else(|| entry.clone());

        luaurc
            .aliases
            .insert(name.clone(), format!("./{}", relative.display()));
    }

    std::fs::write(&luaurc_path, serde_json::to_string_pretty(&luaurc)?)?;
    Ok(())
}

/// Find entry point for a package.
fn find_entry_point(pkg_path: &Path) -> PathBuf {
    // Direct candidates
    for candidate in ["init.luau", "main.luau", "lib/init.luau", "src/init.luau"] {
        let path = pkg_path.join(candidate);
        if path.exists() {
            return path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| pkg_path.to_path_buf());
        }
    }

    // Search one level deep for init.luau
    if let Ok(entries) = std::fs::read_dir(pkg_path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let init_path = entry.path().join("init.luau");
                if init_path.exists() {
                    return entry.path();
                }
            }
        }
    }

    pkg_path.to_path_buf()
}
