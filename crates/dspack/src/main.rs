use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dspack",
    version,
    about = "Make a plain web chatbot useful for coding, using copy and paste"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build a markdown prompt from this repo and copy it to the clipboard
    Pack,
    /// Parse a model reply from the clipboard and apply the changes you accept
    Apply,
    /// Revert the last applied batch
    Undo,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Pack => println!("pack: not implemented yet (roadmap M2)"),
        Command::Apply => println!("apply: not implemented yet (roadmap M1)"),
        Command::Undo => println!("undo: not implemented yet (roadmap M3)"),
    }
}
