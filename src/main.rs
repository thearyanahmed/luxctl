use clap::{arg, Parser, Subcommand};

use lux::VERSION;

#[derive(Parser)]
#[command(name = "lux")]
#[command(version = VERSION)]
struct CLI {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(short ='p', long)]
        project: String,

        #[arg(short = 't', long)]
        task: String,
    }
}


// scratch pad:
// lux run -p project_slug -t task_slug 
// same as: lux run --project $project_slug --task $task_slug
//
// we should log as well, and maybe the user can have it as verbose log

fn main() {
    env_logger::init();

    let cli = CLI::parse();

    match cli.commands {
        Commands::Run { project, task } => {
            log::info!("Running task '{}' for project '{}'", task, project);
        }
    }
}


