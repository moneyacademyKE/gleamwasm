use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "gleam-wasm",
    about = "Direct Gleam-to-Wasm GC compiler backend"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Build {
        #[arg(long, default_value = "wasm-web")]
        target: Option<String>,

        #[arg(short, long)]
        input: String,

        #[arg(short, long)]
        output: String,

        #[arg(long)]
        emit_wat: bool,
    },
}
