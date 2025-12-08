use std::{env::args_os, process::ExitCode};

use anyhow::Result;
use clap::Parser;

pub(crate) mod build;
pub(crate) mod installer;
pub(crate) mod list;
pub(crate) mod repl;
pub(crate) mod run;
pub(crate) mod utils;

pub use self::{build::BuildCommand, list::ListCommand, repl::ReplCommand, run::RunCommand};

/// Lune Custom Build - A standalone Luau runtime for backend/game-server development
#[derive(Parser, Debug, Clone)]
#[command(name = "lune")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Initialize project (creates lune.config.json and .luaurc)
    #[arg(long)]
    pub init: bool,

    /// Install packages. Without args: reads lune.config.json. With args: installs specified packages
    #[arg(short, long, num_args = 0..)]
    pub install: Option<Vec<String>>,

    /// Script file to run
    #[arg(index = 1)]
    pub script: Option<String>,

    /// Arguments passed to the script
    #[arg(index = 2, num_args = 0..)]
    pub script_args: Vec<String>,

    /// List available scripts
    #[arg(long)]
    pub list: bool,

    /// Build standalone executable from script
    #[arg(long)]
    pub build: Option<std::path::PathBuf>,

    /// Start interactive REPL
    #[arg(long)]
    pub repl: bool,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            init: false,
            install: None,
            script: None,
            script_args: Vec::new(),
            list: false,
            build: None,
            repl: false,
        }
    }
}

impl Cli {
    pub fn new() -> Self {
        // Handle legacy `lune run <script>` syntax for compatibility
        if args_os()
            .nth(1)
            .is_some_and(|arg| arg.eq_ignore_ascii_case("run"))
        {
            let Some(script_path) = args_os()
                .nth(2)
                .and_then(|arg| arg.to_str().map(String::from))
            else {
                return Self::parse();
            };

            let script_args = args_os()
                .skip(3)
                .filter_map(|arg| arg.to_str().map(String::from))
                .collect::<Vec<_>>();

            return Self {
                script: Some(script_path),
                script_args,
                ..Default::default()
            };
        }

        Self::parse()
    }

    pub async fn run(self) -> Result<ExitCode> {
        // Priority: --init > --install > --list > --build > --repl > script

        // Mode: Init project
        if self.init {
            return installer::run_init().await;
        }

        // Mode: Installation
        if let Some(packages) = self.install {
            return installer::run_install(packages).await;
        }

        // Mode: List
        if self.list {
            return ListCommand {}.run().await;
        }

        // Mode: Build
        if let Some(input) = self.build {
            return BuildCommand {
                input,
                output: None,
                target: None,
            }
            .run()
            .await;
        }

        // Mode: REPL (explicit or no script)
        if self.repl || self.script.is_none() {
            return ReplCommand {}.run().await;
        }

        // Mode: Run script (default)
        if let Some(script_path) = self.script {
            return RunCommand {
                script_path,
                script_args: self.script_args,
            }
            .run()
            .await;
        }

        // Fallback to REPL
        ReplCommand {}.run().await
    }
}
