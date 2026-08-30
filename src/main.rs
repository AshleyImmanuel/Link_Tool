use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod db;
mod extractor;
mod hasher;
mod lang;
mod parser;
mod resolver;
mod viewer;

#[derive(Parser)]
#[command(name = "link", version, about = "Git for understanding code.")]
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
    /// Show index statistics
    Stats,
    /// Explain a symbol's connections in text form
    Explain {
        /// Symbol name to explain
        symbol: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { quiet } => commands::init::run(quiet),
        Commands::Show { symbol, json, quiet } => commands::show::run(&symbol, json, quiet),
        Commands::List { quiet } => commands::list::run(quiet),
        Commands::Search { query, quiet } => commands::search::run(&query, quiet),
        Commands::Update { quiet } => commands::update::run(quiet),
        Commands::Stats => commands::stats::run(),
        Commands::Explain { symbol } => commands::explain::run(&symbol),
    }
}
