use clap::Parser;

mod tasks;
mod validators;
mod version;

#[derive(Parser)]
#[command(name = "lux")]
#[command(version = version::version())]
struct CLI {

}

fn main() {
    let _cli = CLI::parse();

}


