//! CDDA-BR — Cataclysm: Dark Days Ahead, Bevy/Rust port.
//!
//! CLI entry point with debug dump subcommands.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cdda", version, about = "Cataclysm: Dark Days Ahead")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the game (default).
    Run,
    /// Dump the Bevy schedule graph as a DOT file.
    #[command(name = "schedule-graph")]
    ScheduleGraph {
        /// Output file (defaults to stdout).
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Dump the Bevy render graph as a DOT file.
    #[command(name = "render-graph")]
    RenderGraph {
        /// Output file (defaults to stdout).
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Dump debug graphs (compat with bevy_mod_debugdump::CommandLineArgs)
    #[command(name = "dump")]
    Dump {
        /// Which graph: "schedule" (default) or "render"
        #[arg(default_value = "schedule")]
        kind: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Run) {
        Command::Run => {
            cdda_app::run();
            Ok(())
        }

        Command::ScheduleGraph { output } => {
            let mut app = bevy::app::App::new();
            app.add_plugins(cdda_app::CddaPlugin);
            use bevy_mod_debugdump::schedule_graph::Settings;
            let dot = bevy_mod_debugdump::schedule_graph_dot(
                &mut app,
                bevy::app::Update,
                &Settings::default(),
            );
            if let Some(path) = output {
                std::fs::write(&path, &dot)?;
                eprintln!("Schedule graph written to {path}");
            } else {
                print!("{dot}");
            }
            Ok(())
        }

        Command::RenderGraph { output } => {
            let mut app = bevy::app::App::new();
            app.add_plugins(cdda_app::CddaPlugin);
            use bevy_mod_debugdump::render_graph;
            let dot =
                bevy_mod_debugdump::render_graph_dot(&mut app, &render_graph::Settings::default());
            if let Some(path) = output {
                std::fs::write(&path, &dot)?;
                eprintln!("Render graph written to {path}");
            } else {
                print!("{dot}");
            }
            Ok(())
        }

        Command::Dump { kind } => {
            let mut app = bevy::app::App::new();
            app.add_plugins(cdda_app::CddaPlugin);
            match kind.as_str() {
                "schedule" | "s" => {
                    bevy_mod_debugdump::print_schedule_graph(&mut app, bevy::app::Update);
                }
                "render" | "r" => {
                    bevy_mod_debugdump::print_render_graph(&mut app);
                }
                other => {
                    eprintln!("Unknown dump kind \"{other}\". Use \"schedule\" or \"render\".");
                    std::process::exit(1);
                }
            }
            Ok(())
        }
    }
}
