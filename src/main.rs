use std::path::PathBuf;

use clap::{Parser, command};


#[derive(Parser, Debug)]
#[command(name = "Local VPT")]
#[command(author, version, about, long_about = None)]
struct Arguments {
    output: PathBuf,
    #[arg(short, long)]
    volume: PathBuf,
    #[arg(short, long)]
    transfer_function: Option<PathBuf>,
    #[arg(short, long)]
    camera_position: Option<Vec<i32>>
}

fn main() {
    let cli = Arguments::parse();

    println!("Starting...")
}
