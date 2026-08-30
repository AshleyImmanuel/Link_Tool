use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::ExitCode;

mod commands;
mod db;
mod error;
mod extractor;
mod framework;
mod hasher;
mod history;
mod intel;
mod lang;
mod parser;
mod resolver;
mod scan;
mod snapshot;
mod ui;
mod viewer;

#[derive(Parser)]
#[command(name = "linkmap", version, about = "Git for understanding code.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan current directory, parse files, build index
    Init {
        /// Suppress output
        #[arg(long)]
        quiet: bool,
    },
    /// Show graph for a symbol (opens in browser)
    Show {
        /// Symbol name to visualize
        symbol: String,
        /// Prefer a specific definition by file path when multiple exist
        #[arg(long)]
        file: Option<String>,
        /// Output JSON to stdout instead of opening browser
        #[arg(long)]
        json: bool,
        /// Suppress status output
        #[arg(long)]
        quiet: bool,
    },
    /// List all indexed symbols
    List {
        /// Suppress header, show names only
        #[arg(long)]
        quiet: bool,
    },
    /// Fuzzy search symbols by name
    Search {
        /// Search query
        query: String,
        /// Suppress header
        #[arg(long)]
        quiet: bool,
    },
    /// Re-index only changed files
    Update {
        /// Suppress output
        #[arg(long)]
        quiet: bool,
    },
    /// Show project-local Link command history
    History {
        /// Show all recorded commands for this project
        #[arg(long)]
        all: bool,
        /// Maximum number of history entries to print
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Show index stats, heuristic rules, and local git change summary
    Stats,
    /// Explain a symbol's connections and local impact hints in text form
    Explain {
        /// Symbol name to explain
        symbol: String,
    },
    /// Write a portable structure snapshot (offline artifact)
    Snapshot {
        /// Output path (defaults to .link/snapshot.json)
        #[arg(long)]
        out: Option<String>,
        /// Suppress output
        #[arg(long)]
        quiet: bool,
    },
    /// Diff two snapshots and print a structural change report
    Diff {
        /// Snapshot file path
        from: String,
        /// Snapshot file path
        to: String,
        /// Output JSON diff to stdout
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", error::format_error(&err));
            ExitCode::from(error::exit_code(&err))
        }
    }
}

fn run() -> Result<()> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let cli = Cli::parse();
    let invocation = history::format_invocation(&raw_args);
    let cwd = std::env::current_dir().ok();

    let quiet = command_quiet(&cli.command);
    ui::disclaimer(quiet);

    let result = match cli.command {
        Commands::Init { quiet } => commands::init::run(quiet),
        Commands::Show {
            symbol,
            file,
            json,
            quiet,
        } => commands::show::run(&symbol, file.as_deref(), json, quiet),
        Commands::List { quiet } => commands::list::run(quiet),
        Commands::Search { query, quiet } => commands::search::run(&query, quiet),
        Commands::Update { quiet } => commands::update::run(quiet),
        Commands::History { all, limit } => commands::history::run(all, limit),
        Commands::Stats => commands::stats::run(),
        Commands::Explain { symbol } => commands::explain::run(&symbol),
        Commands::Snapshot { out, quiet } => commands::snapshot::run(out.as_deref(), quiet),
        Commands::Diff { from, to, json } => commands::diff::run(&from, &to, json),
    };

    if let Some(cwd) = cwd {
        let link_dir = cwd.join(".link");
        let _ = history::record_command(&link_dir, &cwd, &invocation, result.is_ok());
    }

    result
}

fn command_quiet(command: &Commands) -> bool {
    match command {
        Commands::Init { quiet } => *quiet,
        Commands::Show { quiet, .. } => *quiet,
        Commands::List { quiet } => *quiet,
        Commands::Search { quiet, .. } => *quiet,
        Commands::Update { quiet } => *quiet,
        Commands::Snapshot { quiet, .. } => *quiet,
        // Other commands don't currently support --quiet, so we show the disclaimer.
        Commands::History { .. }
        | Commands::Stats
        | Commands::Explain { .. }
        | Commands::Diff { .. } => false,
    }
}
