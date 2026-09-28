#[path = "../cli.rs"]
mod cli;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: firmium-cli <command> [arg]");
        std::process::exit(1);
    }

    std::process::exit(cli::run_cli(&args.join(" ")));
}