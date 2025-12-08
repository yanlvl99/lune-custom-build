//! Git operations for cloning repositories.

use git2::{build::RepoBuilder, FetchOptions, Repository};
use lune_utils::{AbsolutePath, InstallError};

/// Clone a repository with shallow depth.
pub fn clone_shallow(
    url: &str,
    target: &AbsolutePath,
    tag: &str,
) -> Result<Repository, InstallError> {
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.depth(1);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_opts);
    builder.branch(tag);

    builder.clone(url, target.as_path()).map_err(|e| InstallError::GitCloneFailed {
        url: url.to_owned(),
        message: e.message().to_owned(),
    })
}

/// List remote tags from a repository URL.
pub fn list_remote_tags(url: &str) -> Result<Vec<String>, InstallError> {
    let repo = Repository::init_bare(std::env::temp_dir().join(".lune_remote_temp"))
        .map_err(InstallError::from)?;

    let mut remote = repo.remote_anonymous(url).map_err(InstallError::from)?;

    remote.connect(git2::Direction::Fetch).map_err(InstallError::from)?;

    let tags: Vec<String> = remote
        .list()
        .map_err(InstallError::from)?
        .iter()
        .filter_map(|head| {
            head.name()
                .strip_prefix("refs/tags/")
                .map(|s| s.to_owned())
        })
        .collect();

    Ok(tags)
}
