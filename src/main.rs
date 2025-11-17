use clap::Parser;

mod tasks;
mod validators;

pub const VERSION: &str = "0.0.1";

#[derive(Parser)]
#[command(name = "lux")]
#[command(version = VERSION)]
struct CLI {

}

fn main() {
    let _cli = CLI::parse();

}


