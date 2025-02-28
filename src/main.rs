mod cli;
mod config;
mod shell;
mod tools;

fn main() {
    env_logger::init();
    cli::run();
}
