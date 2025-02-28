use crate::config::ClassConfig;
use colored::*;
use std::env;
use std::process::{exit, Command};

/// Run a new shell for the class
pub fn run_shell(class_config: &ClassConfig) {
    println!(
        "{} {}...",
        "quicktool starting new subshell for class",
        class_config.class.green().bold()
    );

    println!(
        "{} This shell is configured for the class environment and quicktool built-in tools will NOT work in this shell.",
        "WARNING:".red().bold()
    );
    
    // Get user's shell or default to bash
    let shell = env::var("SHELL").unwrap_or_else(|_| String::from("/bin/bash"));

    // If .newclassrc exists, source it via -c; otherwise, just run the shell
    if let Some(newclassrc_path) = class_config.newclassrc_path.as_ref().filter(|_| class_config.has_newclassrc()) {
        let cmd_string = format!("source {} && exec $SHELL --norc", newclassrc_path);
        let status = Command::new(&shell)
            .arg("-c")
            .arg(cmd_string)
            .status();

        if let Err(_) = status {
            eprintln!("quicktool: cannot find shell {}: giving up", shell);
            exit(1);
        }
    } else {
        // If no .newclassrc, just start the shell with --norc
        let status = Command::new(&shell)
            .arg("--norc")
            .status();

        if let Err(_) = status {
            eprintln!("quicktool: cannot find shell {}: giving up", shell);
            exit(1);
        }
    }
}

/// Execute a command with the class environment
pub fn execute_command(class_config: &ClassConfig, args: &[String]) {
    // Source .newclassrc if it exists
    if let Some(newclassrc_path) = class_config.newclassrc_path.as_ref().filter(|_| class_config.has_newclassrc()) {
        execute_with_newclassrc(newclassrc_path, args);
    } else {
        // Regular command execution without .newclassrc
        execute_direct_command(args);
    }
}

/// Execute a command with .newclassrc sourcing
fn execute_with_newclassrc(newclassrc_path: &str, args: &[String]) {
    let shell = env::var("SHELL").unwrap_or_else(|_| String::from("/bin/bash"));
    let cmd_str = format!("source {} && exec {}", newclassrc_path, args.join(" "));

    if let Err(e) = Command::new(&shell).arg("-c").arg(cmd_str).status() {
        eprintln!("quicktool: error executing command: {}", e);
        exit(1);
    }
}

/// Execute a command directly without .newclassrc
fn execute_direct_command(args: &[String]) {
    if let Err(e) = Command::new(&args[0]).args(&args[1..]).status() {
        eprintln!("quicktool: error executing command: {}", e);
        exit(1);
    }
}
