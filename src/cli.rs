use crate::config::ClassConfig;
use crate::shell;
use crate::tools::{autotest, fetch_activity};
use std::env;
use std::process::exit;

/// Parse command line arguments and determine class code and remaining arguments
fn parse_args() -> (String, Vec<String>) {
    let args: Vec<String> = env::args().collect();
    let program_name = args[0].split('/').last().unwrap_or("quicktool");

    if program_name == "quicktool" {
        if args.len() < 2 {
            eprintln!("Usage: quicktool classname [command]");
            exit(2);
        }
        (args[1].clone(), args[2..].to_vec())
    } else {
        (program_name.to_string(), args[1..].to_vec())
    }
}

/// Get class configuration or exit with error if not valid
fn get_class_config(class_code: &str, program_name: &str) -> ClassConfig {
    match ClassConfig::new(class_code) {
        Some(c) => c,
        None => {
            eprintln!(
                "{}{}: {} is not a valid class",
                if program_name == "quicktool" {
                    "quicktool "
                } else {
                    ""
                },
                class_code,
                class_code
            );
            exit(2);
        }
    }
}

/// Set up environment variables for the class
fn setup_environment(class_config: &ClassConfig) {
    // Save original environment variables
    let noclass_path =
        env::var("noclass_PATH").unwrap_or_else(|_| env::var("PATH").unwrap_or_default());
    let noclass_manpath =
        env::var("noclass_MANPATH").unwrap_or_else(|_| env::var("MANPATH").unwrap_or_default());
    let noclass_ps1 =
        env::var("noclass_PS1").unwrap_or_else(|_| env::var("PS1").unwrap_or_default());

    // Prepare new environment
    let ps1 = format!("({}) {}", class_config.class, noclass_ps1);

    // Set paths based on class configuration
    let path = class_config.get_path(&noclass_path);
    let manpath = class_config.get_manpath(&noclass_manpath);

    if path == noclass_path && manpath == noclass_manpath {
        eprintln!(
            "Warning: no path information for class {}",
            class_config.class
        );
    }

    // Export environment variables
    env::set_var("PATH", &path);
    env::set_var("MANPATH", &manpath);
    env::set_var("PS1", &ps1);
    env::set_var("noclass_PATH", &noclass_path);
    env::set_var("noclass_MANPATH", &noclass_manpath);
    env::set_var("noclass_PS1", &noclass_ps1);

    if let Some(account) = &class_config.account_name {
        env::set_var("GIVECLASS", account);
    }
}

/// Display help information
fn show_help(class_config: &ClassConfig) {
    println!("Usage: {} [command]", class_config.class);
    println!("Commands:");
    println!("  help            Display this help message");
    println!("  autotest        Run autotest for the current directory");
    println!("  autotest-stage  Run autotest for a specific stage");
    println!("  fetch-activity  Fetch activity starter code");
    println!("  ...             Run a command in the class environment");
    println!("");
    println!("If no command is specified, a shell with the class environment will be started.");
}

pub fn run() {
    // Parse command line arguments
    let (class_code, remaining_args) = parse_args();
    let program_name = env::args()
        .nth(0)
        .unwrap_or_default()
        .split('/')
        .last()
        .unwrap_or("quicktool")
        .to_string();

    // Get class configuration
    let mut class_config = get_class_config(&class_code, &program_name);

    // Setup environment for the class
    setup_environment(&class_config);

    // Handle commands based on the first argument
    match remaining_args.get(0).map(|s| s.as_str()) {
        None => {
            shell::run_shell(&class_config);
        }
        Some("help") => {
            show_help(&class_config);
        }
        Some("autotest") | Some("autotest-stage") => {
            match autotest::run_test(&mut class_config, &remaining_args) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error: {}", e);
                    exit(1);
                }
            }
        }
        Some("fetch-activity") => {
            match fetch_activity::run_fetch_activity(&mut class_config, &remaining_args[1..]) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error: {}", e);
                    exit(1);
                }
            }
        }
        Some(_) => {
            shell::execute_command(&class_config, &remaining_args);
        }
    }
}
