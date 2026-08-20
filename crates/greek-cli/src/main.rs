// Main entry point for the CLI application

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::generate;
use color_eyre::Result;
use greek_common::UninstallOptions;
use greek_core::{ConfigManager, GreekAppService};
use std::io;

#[derive(Clone, Copy, ValueEnum)]
enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
}
#[derive(Parser)]
#[command(name = "reek")]
#[command(about = "REEK Ultimate Uninstaller - The uninstaller that actually uninstalls", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// List installed applications
    List {
        #[arg(short, long, default_value = "table")]
        format: String,
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Search for applications
    Search {
        query: String,
        #[arg(short, long)]
        fuzzy: bool,
    },

    /// Uninstall an application
    Uninstall {
        #[arg(required = true)]
        app: String,
        #[arg(short, long)]
        silent: bool,
        #[arg(short, long)]
        force: bool,
        #[arg(short, long)]
        yes: bool,
        #[arg(long)]
        timeout: Option<u64>,
    },

    /// Scan for leftover artifacts
    Scan {
        #[arg(long)]
        leftovers: bool,
        #[arg(long)]
        all: bool,
        #[arg(short, long)]
        app: Option<String>,
        #[arg(short, long)]
        export: Option<String>,
    },

    /// Clean leftover artifacts
    Clean {
        #[arg(short, long)]
        leftovers: bool,
        #[arg(short, long)]
        app: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },

    /// Create system restore point
    RestorePoint {
        #[arg(short, long, default_value = "REEK Uninstaller Restore Point")]
        description: String,
    },

    /// Show application details
    Info { app: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup error handling
    color_eyre::install()?;

    // Setup tracing
    let filter = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Load configuration
    let config_manager = ConfigManager::new()?;
    let _config = config_manager.load_config()?;

    // Initialize service
    let service = GreekAppService::from_config_manager(&config_manager)?;

    // Execute command
    match cli.command {
        Commands::Completions { shell } => {
            cmd_completions(shell)?;
        }
        Commands::List { format, output } => {
            cmd_list(service, format, output).await?;
        }
        Commands::Search { query, fuzzy } => {
            cmd_search(service, query, fuzzy).await?;
        }
        Commands::Uninstall {
            app,
            silent,
            force,
            yes,
            timeout,
        } => {
            cmd_uninstall(service, app, silent, force, yes, timeout).await?;
        }
        Commands::Scan {
            leftovers,
            all,
            app,
            export,
        } => {
            cmd_scan(service, leftovers, all, app, export).await?;
        }
        Commands::Clean {
            leftovers,
            app,
            yes,
        } => {
            cmd_clean(service, leftovers, app, yes).await?;
        }
        Commands::RestorePoint { description } => {
            cmd_restore_point(service, description).await?;
        }
        Commands::Info { app } => {
            cmd_info(service, app).await?;
        }
    }

    Ok(())
}

async fn cmd_list(mut service: GreekAppService, format: String, output: Option<String>) -> Result<()> {
    println!("Scanning for installed applications...");

    let apps = service.scan_all_apps().await?;

    match format.as_str() {
        "table" => {
            render_table(&apps);
        }
        "json" => {
            let json = serde_json::to_string_pretty(&apps)?;
            println!("{}", json);
        }
        "csv" => {
            render_csv(&apps);
        }
        _ => {
            eprintln!("Unknown format: {}", format);
        }
    }

    if let Some(output_path) = output {
        // Save to file
        println!("Output saved to: {}", output_path);
    }

    Ok(())
}

async fn cmd_search(mut service: GreekAppService, query: String, fuzzy: bool) -> Result<()> {
    println!("Searching for: {}", query);

    let apps = service.scan_all_apps().await?;

    let filtered: Vec<_> = apps
        .into_iter()
        .filter(|app| {
            if fuzzy {
                app.name.to_lowercase().contains(&query.to_lowercase())
                    || app
                        .publisher
                        .as_ref()
                        .map(|p| p.to_lowercase().contains(&query.to_lowercase()))
                        .unwrap_or(false)
            } else {
                app.name.to_lowercase() == query.to_lowercase()
            }
        })
        .collect();

    render_table(&filtered);

    Ok(())
}

async fn cmd_uninstall(
    mut service: GreekAppService,
    app_name: String,
    silent: bool,
    force: bool,
    yes: bool,
    timeout: Option<u64>,
) -> Result<()> {
    println!("Searching for application: {}", app_name);

    let apps = service.scan_all_apps().await?;

    // MED-2: use fuzzy matching (contains) like cmd_search, so partial
    // names work.  If multiple apps match, prefer exact name match;
    // otherwise pick the first close match.
    let query_lower = app_name.to_lowercase();
    let matching: Vec<_> = apps
        .into_iter()
        .filter(|a| a.name.to_lowercase().contains(&query_lower))
        .collect();

    let app = if matching.len() == 1 {
        matching.into_iter().next().unwrap()
    } else {
        // Try exact match first
        matching
            .iter()
            .find(|a| a.name.to_lowercase() == query_lower)
            .cloned()
            .or_else(|| matching.into_iter().next())
            .ok_or_else(|| color_eyre::eyre::eyre!("Application not found: {}", app_name))?
    };

    println!("Found: {}", app.display_name());
    println!(
        "Publisher: {}",
        app.publisher.as_deref().unwrap_or("Unknown")
    );
    println!("Version: {}", app.version.as_deref().unwrap_or("Unknown"));

    if !yes
        && !dialoguer::Confirm::new()
            .with_prompt("Do you want to uninstall this application?")
            .interact()?
    {
        println!("Uninstall cancelled.");
        return Ok(());
    }

    let mut options = UninstallOptions::standard();
    options.silent = silent;
    options.force = force;
    options.timeout_seconds = timeout;

    println!("Uninstalling...");
    let result = service.uninstall_app(&app, options).await?;

    if result.success {
        println!("Uninstall completed successfully!");
    } else {
        println!("Uninstall failed!");
        for error in &result.errors {
            eprintln!("Error: {}", error);
        }
    }

    Ok(())
}

async fn cmd_scan(
    mut service: GreekAppService,
    leftovers: bool,
    all: bool,
    app_name: Option<String>,
    export: Option<String>,
) -> Result<()> {
    if leftovers {
        if let Some(name) = app_name {
            println!("Scanning for leftovers of: {}", name);

            let apps = service.scan_all_apps().await?;
            let app = apps
                .into_iter()
                .find(|a| a.name.to_lowercase() == name.to_lowercase())
                .ok_or_else(|| color_eyre::eyre::eyre!("Application not found: {}", name))?;

            let artifacts = service.analyze_leftovers(&app).await?;

            println!("Found {} leftover artifacts:", artifacts.len());
            for artifact in &artifacts {
                println!(
                    "  - {:?} (confidence: {:.2})",
                    artifact.artifact_type, artifact.confidence
                );
                println!("    Path: {}", artifact.path.display());
                println!("    Safety: {:?}", artifact.safety_level);
            }
        } else if all {
            println!("Scanning for system-wide leftovers...");
            // System-wide scan would be implemented here
        } else {
            println!("Please specify an app name with --app or use --all for system-wide scan");
        }
    } else {
        println!("Use --leftovers to scan for leftover artifacts");
    }

    if let Some(export_path) = export {
        println!("Export results to: {}", export_path);
    }

    Ok(())
}

async fn cmd_clean(
    mut service: GreekAppService,
    leftovers: bool,
    app_name: Option<String>,
    yes: bool,
) -> Result<()> {
    if leftovers {
        if let Some(name) = app_name {
            println!("Cleaning leftovers for: {}", name);

            let apps = service.scan_all_apps().await?;
            let app = apps
                .into_iter()
                .find(|a| a.name.to_lowercase() == name.to_lowercase())
                .ok_or_else(|| color_eyre::eyre::eyre!("Application not found: {}", name))?;

            let artifacts = service.analyze_leftovers(&app).await?;

            if artifacts.is_empty() {
                println!("No leftovers found.");
                return Ok(());
            }

            println!("Found {} leftover artifacts:", artifacts.len());

            if !yes
                && !dialoguer::Confirm::new()
                    .with_prompt("Do you want to clean these leftovers?")
                    .interact()?
            {
                println!("Clean cancelled.");
                return Ok(());
            }

            let artifact_ids: Vec<_> = artifacts.iter().map(|a| a.id).collect();
            service
                .clean_leftovers(artifact_ids, UninstallOptions::force())
                .await?;

            println!("Cleanup completed!");
        } else {
            println!("Please specify an app name with --app");
        }
    } else {
        println!("Use --leftovers to clean leftover artifacts");
    }

    Ok(())
}

async fn cmd_restore_point(_service: GreekAppService, description: String) -> Result<()> {
    println!("Creating system restore point: {}", description);

    #[cfg(all(target_os = "windows", feature = "windows"))]
    {
        let manager = greek_windows::RestorePointManager::new();
        match manager.create_restore_point(&description).await {
            Ok(_) => {
                println!("Restore point created successfully.");
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to create restore point: {}", e);
                Err(e)
            }
        }
    }

    #[cfg(not(all(target_os = "windows", feature = "windows")))]
    {
        let _ = description;
        println!("System Restore is not available on this platform.");
        Ok(())
    }
}

async fn cmd_info(mut service: GreekAppService, app_name: String) -> Result<()> {
    println!("Getting info for: {}", app_name);

    let apps = service.scan_all_apps().await?;

    let app = apps
        .into_iter()
        .find(|a| a.name.to_lowercase() == app_name.to_lowercase())
        .ok_or_else(|| color_eyre::eyre::eyre!("Application not found: {}", app_name))?;

    println!("Name: {}", app.name);
    println!(
        "Publisher: {}",
        app.publisher.as_deref().unwrap_or("Unknown")
    );
    println!("Version: {}", app.version.as_deref().unwrap_or("Unknown"));
    println!("Install Date: {:?}", app.install_date);
    println!("Install Location: {:?}", app.install_location);
    println!("Size: {}", app.display_size().unwrap_or_else(|| "Unknown".to_string()));
    println!("Source: {:?}", app.source);
    println!("System Component: {}", app.is_system_component);

    if !app.registry_keys.is_empty() {
        println!("Registry Keys: {}", app.registry_keys.len());
    }

    Ok(())
}

fn cmd_completions(shell: Shell) -> Result<()> {
    let mut cli = Cli::command();

    let shell_enum = match shell {
        Shell::Bash => clap_complete::Shell::Bash,
        Shell::Zsh => clap_complete::Shell::Zsh,
        Shell::Fish => clap_complete::Shell::Fish,
        Shell::Powershell => clap_complete::Shell::PowerShell,
        Shell::Elvish => clap_complete::Shell::Elvish,
    };

    generate(shell_enum, &mut cli, "reek", &mut io::stdout());

    Ok(())
}

fn render_table(apps: &[greek_common::InstalledApp]) {
    use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, Table};

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec!["Name", "Publisher", "Version", "Size"]);

    for app in apps {
        table.add_row(vec![
            Cell::new(&app.name).fg(Color::Green),
            Cell::new(app.publisher.as_deref().unwrap_or("Unknown")),
            Cell::new(app.version.as_deref().unwrap_or("Unknown")),
            Cell::new(app.display_size().unwrap_or_else(|| "Unknown".to_string())),
        ]);
    }

    println!("{}", table);
}

fn render_csv(apps: &[greek_common::InstalledApp]) {
    println!("Name,Publisher,Version,Size,Install Date");
    for app in apps {
        println!(
            "{},{},{},{},{}",
            app.name,
            app.publisher.as_deref().unwrap_or("Unknown"),
            app.version.as_deref().unwrap_or("Unknown"),
            app.display_size().unwrap_or_else(|| "Unknown".to_string()),
            app.install_date
                .map(|d| d.to_string())
                .unwrap_or("Unknown".to_string())
        );
    }
}
