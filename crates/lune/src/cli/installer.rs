//! Package installer with zip download.
//!
//! Installs packages from the central registry to ./lune_packages/
use std::collections::{HashMap, HashSet, VecDeque};
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
#[derive(Debug, Clone, Deserialize, Serialize)] // <--- SÓ UMA DESSA
#[allow(dead_code)]
struct PackageManifest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    repository: String,
    #[serde(default)]
    dependencies: HashMap<String, String>,
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
    println!("\n{}", style("  Lune Project Initializer").bold());
    println!("{}", style("  ========================").dim());

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
        println!("{:>12} {} type definitions", style("Generated").green().bold(), generated_count);
    } else {
        println!("{:>12} Type definitions (already exist)", style("Skipped").yellow().bold());
    }

    // Create lune.config.json
    if config_path.exists() {
        println!("{:>12} lune.config.json (already exists)", style("Skipped").yellow().bold());
    } else {
        let config = LuneConfig::default();
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        println!("{:>12} lune.config.json", style("Created").green().bold());
    }

    // Create .luaurc with @lune alias
    let mut luaurc = if luaurc_path.exists() {
        let content = std::fs::read_to_string(&luaurc_path)?;
        serde_json::from_str::<LuauRc>(&content).unwrap_or_default()
    } else {
        LuauRc::default()
    };

    if !luaurc.aliases.contains_key("lune") {
        luaurc
            .aliases
            .insert("lune".to_owned(), typedefs_path.clone());
        std::fs::write(&luaurc_path, serde_json::to_string_pretty(&luaurc)?)?;
        println!("{:>12} .luaurc with @lune alias", style("Updated").green().bold());
    } else {
        println!("{:>12} .luaurc alias (already exists)", style("Skipped").yellow().bold());
    }

    // Create lune_packages directory
    let packages_dir = cwd.join("lune_packages");
    if !packages_dir.exists() {
        std::fs::create_dir_all(&packages_dir)?;
        println!("{:>12} lune_packages/", style("Created").green().bold());
    }

    println!("\n{:>12} Project ready. Use 'lune --install <pkg>'", style("Success").green().bold());

    Ok(ExitCode::SUCCESS)
}

// SUBSTITUA A FUNÇÃO run_install POR ESTA:
pub async fn run_install(packages: Vec<String>) -> Result<ExitCode> {
    // Cabeçalho bonito
    println!("\n{}", style("  Lune Package Installer").bold());
    println!("{}", style("  ======================").dim());

    let cwd = std::env::current_dir()?;
    let packages_dir = cwd.join("lune_packages");

    // Parse args
    let specs_from_args: Vec<PackageSpec> = packages
        .into_iter()
        .filter_map(|s| PackageSpec::try_from(s).ok())
        .collect();

    // Determine queue
    let mut packages_queue: VecDeque<PackageSpec> = if specs_from_args.is_empty() {
        let config_path = cwd.join("lune.config.json");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: LuneConfig = serde_json::from_str(&content)?;
            if config.packages.is_empty() {
                println!("{:>12} No packages to install", style("Info").blue().bold());
                return Ok(ExitCode::SUCCESS);
            }
            VecDeque::from(config.packages)
        } else {
            println!("{:>12} No config found. Run lune --init", style("Warn").yellow().bold());
            return Ok(ExitCode::SUCCESS);
        }
    } else {
        VecDeque::from(specs_from_args)
    };
    
    let explicit_packages: Vec<PackageSpec> = packages_queue.iter().cloned().collect();

    if !packages_dir.exists() {
        std::fs::create_dir_all(&packages_dir)?;
    }

    let mut installed_paths: Vec<(String, PathBuf)> = Vec::new();
    let mut visited_packages: HashSet<String> = HashSet::new();

    // LOOP
    while let Some(spec) = packages_queue.pop_front() {
        if visited_packages.contains(&spec.name) {
            continue;
        }

        // LOG: Resolving (Cyan)
        println!("{:>12} {}", style("Resolving").cyan().bold(), style(&spec.name).bold());

        match install_package_with_version(&spec.name, spec.version.as_deref(), &packages_dir).await
        {
            Ok((path, dependencies)) => {
                // LOG: Installed (Green)
                println!("{:>12} {} {}\n", 
                    style("Installed").green().bold(), 
                    spec.name,
                    style(spec.version.as_deref().unwrap_or("latest")).dim()
                );
                
                visited_packages.insert(spec.name.clone());
                installed_paths.push((spec.name.clone(), path));

                if !dependencies.is_empty() {
                    for (dep_name, dep_ver) in dependencies {
                        if !visited_packages.contains(&dep_name) {
                            // LOG: Found (Dim/Blue)
                            println!("{:>12} dependency: {}@{}", 
                                style("Found").blue().dim(), 
                                dep_name, 
                                dep_ver
                            );

                            let version_opt = if dep_ver == "latest" || dep_ver == "*" {
                                None
                            } else {
                                Some(dep_ver)
                            };

                            packages_queue.push_back(PackageSpec {
                                name: dep_name,
                                version: version_opt,
                            });
                        }
                    }
                }
            }
            Err(e) => {
                // LOG: Error (Red)
                println!("{:>12} {} -> {}", style("Failed").red().bold(), spec.name, e);
            }
        }
    }

    // Config Update
    println!("{:>12} lune.config.json", style("Updating").cyan().bold());
    update_config(&cwd, &explicit_packages)?;

    // Luaurc Update
    println!("{:>12} .luaurc definition paths", style("Mapping").cyan().bold());
    generate_luaurc(&cwd, &installed_paths)?;

    println!("\n{:>12} All packages ready.\n", style("Finished").green().bold());

    Ok(ExitCode::SUCCESS)
}

/// Update all packages to latest versions.
#[allow(clippy::unused_async)]
pub async fn run_update() -> Result<ExitCode> {
    println!("\n{}", style("  Lune Package Updater").bold());
    println!("{}", style("  ====================").dim());

    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("lune.config.json");

    if !config_path.exists() {
        println!("{:>12} No lune.config.json found", style("Error").red().bold());
        return Ok(ExitCode::SUCCESS);
    }

    let content = std::fs::read_to_string(&config_path)?;
    let mut config: LuneConfig = serde_json::from_str(&content)?;

    if config.packages.is_empty() {
        println!("{:>12} No packages to update", style("Info").blue().bold());
        return Ok(ExitCode::SUCCESS);
    }

    let packages_dir = cwd.join("lune_packages");
    let mut updated_count = 0;

    for spec in &mut config.packages {
        // Log: Checking package...
        println!("{:>12} {}...", style("Checking").cyan().bold(), spec.name);

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
            Err(_) => {
                println!("{:>12} Failed to fetch manifest for {}", style("Error").red().bold(), spec.name);
                continue;
            }
        };

        // Resolve version
        let target_version = match &spec.version {
            Some(v) => v.clone(),
            None => resolve_latest_tag_via_api(&manifest.repository)?,
        };

        let needs_update = current_version.as_ref() != Some(&target_version);

        if needs_update {
            let old_ver = current_version.as_deref().unwrap_or("?");
            
            // Log: Updating v1 -> v2
            println!("{:>12} {} -> {}", 
                style("Updating").green().bold(), 
                style(old_ver).dim(), 
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

                    spec.version = Some(target_version);
                    updated_count += 1;
                }
                Err(e) => {
                    println!("{:>12} {}", style("Failed").red().bold(), e);
                }
            }
        } else {
            println!("{:>12} Up to date ({})", style("Skipped").dim(), target_version);
        }
    }

    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    
    // Regenerate .luaurc
    let installed: Vec<(String, PathBuf)> = config
        .packages
        .iter()
        .map(|spec| (spec.name.clone(), packages_dir.join(&spec.name)))
        .collect();
    generate_luaurc(&cwd, &installed)?;

    println!("\n{:>12} {} packages updated", style("Finished").green().bold(), updated_count);

    Ok(ExitCode::SUCCESS)
}

/// Uninstall packages (supports multiple packages).
#[allow(clippy::unused_async)]
pub async fn run_uninstall(packages: Vec<String>) -> Result<ExitCode> {
    println!("\n{}", style("  Lune Package Uninstaller").bold());
    println!("{}", style("  ========================").dim());

    if packages.is_empty() {
        println!("{:>12} Usage: lune --uninstall <pkg>", style("Warn").yellow().bold());
        return Ok(ExitCode::SUCCESS);
    }

    let cwd = std::env::current_dir()?;
    let packages_dir = cwd.join("lune_packages");
    let config_path = cwd.join("lune.config.json");
    let luaurc_path = cwd.join(".luaurc");

    let mut uninstalled_count = 0;

    for pkg_name in &packages {
        let pkg_dir = packages_dir.join(pkg_name);
        if pkg_dir.exists() {
            std::fs::remove_dir_all(&pkg_dir)?;
            println!("{:>12} {}", style("Removed").green().bold(), pkg_name);
            uninstalled_count += 1;
        } else {
            println!("{:>12} {} not found", style("Skipped").yellow().bold(), pkg_name);
        }
    }

    // Update config
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let mut config: LuneConfig = serde_json::from_str(&content).unwrap_or_default();
        config.packages.retain(|p| !packages.contains(&p.name));
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        println!("{:>12} lune.config.json", style("Cleaned").cyan().bold());
    }

    // Update .luaurc
    if luaurc_path.exists() {
        let content = std::fs::read_to_string(&luaurc_path)?;
        let mut luaurc: LuauRc = serde_json::from_str(&content).unwrap_or_default();
        for pkg_name in &packages {
            luaurc.aliases.remove(pkg_name);
        }
        std::fs::write(&luaurc_path, serde_json::to_string_pretty(&luaurc)?)?;
        println!("{:>12} .luaurc aliases", style("Cleaned").cyan().bold());
    }

    println!("\n{:>12} {} packages removed", style("Finished").green().bold(), uninstalled_count);

    Ok(ExitCode::SUCCESS)
}
/// List installed packages.
pub fn run_list_packages() -> Result<ExitCode> {
    println!("\n{}", style("  Installed Packages").bold());
    println!("{}", style("  ==================").dim());

    let cwd = std::env::current_dir()?;
    let packages_dir = cwd.join("lune_packages");

    if !packages_dir.exists() {
        println!("{:>12} No packages installed", style("Empty").dim());
        return Ok(ExitCode::SUCCESS);
    }

    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(&packages_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let pkg_name = entry.file_name().to_string_lossy().to_string();
                let pkg_info_path = entry.path().join("lune-pkg.json");
                
                let version = if let Ok(content) = std::fs::read_to_string(&pkg_info_path) {
                    if let Ok(info) = serde_json::from_str::<LunePkgInfo>(&content) {
                        info.version
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                };

                // Formato:   nome-do-pacote   v1.0.0
                println!("{:>16}   {}", style(pkg_name).bold(), style(version).dim());
                count += 1;
            }
        }
    }

    if count == 0 {
        println!("{:>12} No packages installed", style("Empty").dim());
    } else {
        println!("\n{:>12} packages total", count);
    }

    Ok(ExitCode::SUCCESS)
}

/// Show package info.
pub fn run_package_info(name: &str) -> Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    let pkg_dir = cwd.join("lune_packages").join(name);
    let pkg_info_path = pkg_dir.join("lune-pkg.json");

    println!("\n{}", style(format!("  Package: {}", name)).bold());
    println!("{}", style("  ========").dim());

    if !pkg_dir.exists() {
        println!("{:>12} Package not installed", style("Error").red().bold());
        return Ok(ExitCode::FAILURE);
    }

    if pkg_info_path.exists() {
        let content = std::fs::read_to_string(&pkg_info_path)?;
        let info: LunePkgInfo = serde_json::from_str(&content)?;

        // Alinhamento à direita das chaves
        println!("{:>12} {}", style("Name").blue().bold(), info.name);
        println!("{:>12} {}", style("Version").blue().bold(), style(info.version).yellow());
        
        if let Some(desc) = &info.description {
            println!("{:>12} {}", style("Description").blue().bold(), desc);
        }
        println!("{:>12} {}", style("Repository").blue().bold(), style(info.repository).underlined());
    } else {
        println!("{:>12} No metadata found", style("Warn").yellow());
    }

    println!("\n{:>12}", style("File Structure").dim());
    if let Ok(entries) = std::fs::read_dir(&pkg_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.path().is_dir();
            
            if is_dir {
                println!("{:>12} {}/", style("Dir").dim(), file_name);
            } else {
                println!("{:>12} {}", style("File").dim(), file_name);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

// SUBSTITUA A FUNÇÃO install_package_with_version POR ESTA:
#[allow(clippy::unused_async)]
async fn install_package_with_version(
    name: &str,
    version: Option<&str>,
    packages_dir: &Path,
) -> Result<(PathBuf, HashMap<String, String>)> {
    let manifest_url = format!(
        "https://raw.githubusercontent.com/{}/{}/manifest/{}.json",
        REGISTRY_REPO, REGISTRY_BRANCH, name
    );
    
    // Silencioso aqui, só erro se falhar
    let manifest = fetch_manifest(&manifest_url)?;

    let tag = match version {
        Some(v) => v.to_string(),
        None => resolve_latest_tag_via_api(&manifest.repository)?,
    };

    // LOG: Downloading (Blue)
    println!("{:>12} {} from GitHub...", style("Downloading").blue().bold(), style(&tag).yellow());

    let target_dir = packages_dir.join(name);
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
    }

    // Chama a função que extrai (e agora loga arquivos)
    download_and_extract(&manifest.repository, &tag, name, packages_dir)?;

    let pkg_info = LunePkgInfo {
        name: name.to_string(),
        version: tag.clone(),
        description: manifest.description.clone(),
        repository: manifest.repository.clone(),
    };
    let pkg_info_path = target_dir.join("lune-pkg.json");
    std::fs::write(&pkg_info_path, serde_json::to_string_pretty(&pkg_info)?)?;

    Ok((target_dir, manifest.dependencies))
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

// SUBSTITUA A FUNÇÃO download_and_extract POR ESTA:
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

    let root_prefix = archive
        .by_index(0)?
        .name()
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();

    // Iterando os arquivos
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_path = file.name().to_string(); // Clone pra usar no log

        let relative_path = file_path
            .strip_prefix(&format!("{}/", root_prefix))
            .unwrap_or(&file_path);

        if relative_path.is_empty() {
            continue;
        }

        let out_path = target_dir.join(relative_path);

        // LOG: Extracting (Magenta / Purple)
        // Usamos .dim() para não ficar gritando na tela, já que são muitos arquivos
        if !file.is_dir() {
            println!("{:>12} {}", style("Extracting").magenta(), style(relative_path).dim());
        }

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
