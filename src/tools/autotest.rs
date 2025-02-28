use log::error;
use serde_json::Value;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process::{exit, Command, Stdio};

use crate::config::ClassConfig;

/// Common function to handle both autotest and autotest-stage
pub fn run_test(config: &mut ClassConfig, args: &[String]) -> Result<(), String> {
    // Path to the "autotest" symlink
    let bin_path = config.bin_path.as_deref().unwrap_or("");
    let original_autotest_softlink = Path::new(bin_path).join("autotest");

    // Ensure autotest exists
    if !original_autotest_softlink.exists() {
        return Err(format!("{}: autotest not found", config.class));
    }

    // Resolve symlink to get the real path
    let autotest_path = std::fs::canonicalize(&original_autotest_softlink)
        .map_err(|e| format!("Failed to canonicalize autotest path: {}", e))?;

    // Load the config from config.sh (only once)
    let config_sh = autotest_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("config.sh");
    config
        .load_bash_config(config_sh.to_string_lossy().as_ref())
        .map_err(|e| format!("Could not load bash config: {}", e))?;

    // Determine which functionality to run based on the first argument
    let binary_name = Path::new(&args[0])
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("unknown");
    let passed_args = &args[1..];

    match binary_name {
        "autotest" => run_autotest(config, passed_args),
        "autotest-stage" => run_autotest_stage(config, passed_args),
        _ => Err(String::from(
            "Error: Binary must be called as 'autotest' or 'autotest-stage'",
        )),
    }
}

/// Run the main autotest flow.
fn run_autotest(config: &ClassConfig, args: &[String]) -> Result<(), String> {
    // Build relevant paths
    let activities_dir = Path::new(
        config
            .get_custom_config("public_html_session_directory")
            .unwrap_or(&String::new()),
    )
    .join("activities");

    let autotest_script = Path::new("/usr/local/share/autotest/autotest.py");
    let c_check_path = Path::new(
        config
            .get_custom_config("public_html_session_directory")
            .unwrap_or(&String::new()),
    )
    .join("public/_infra/extern/c_check/c_check.py");

    // Figure out compiler & arguments
    let (compiler, remaining_args) = select_compiler(args);

    // Prepare parameters for autotest
    let parameters = format!(
        "default_compilers = {{'c': [['{compiler}', '-Werror']]}} \
         default_checkers = {{'c': [['python3', '{}']]}}",
        c_check_path.display()
    );

    // Build the command
    let mut command = Command::new("python3");
    command
        // We can set the PATH only on the child process:
        .env(
            "PATH",
            extend_path_with_dir(env::var_os("PATH"), c_check_path.parent()),
        )
        .arg("-I")
        .arg(&autotest_script)
        .arg("--exercise_directory")
        .arg(&activities_dir)
        .arg("--parameters")
        .arg(&parameters);

    // Add remaining arguments
    command.args(&remaining_args);

    // Execute
    run_and_propagate_exit_status(command)
}

/// Run the autotest-stage flow.
fn run_autotest_stage(config: &ClassConfig, args: &[String]) -> Result<(), String> {
    // Accept an optional "1091" prefix, then optional compiler, then a stage prefix, then a command
    let compiler_options = ["dcc", "gcc", "clang"];
    let mut idx = 0;

    // If the first arg is "1091", skip it
    if idx < args.len() && args[idx] == "1091" {
        idx += 1;
    }

    let mut compiler = None;
    if idx < args.len() && compiler_options.contains(&args[idx].as_str()) {
        compiler = Some(args[idx].clone());
        idx += 1;
    }

    // We need at least 2 more arguments: prefix + the subcommand
    if args.len() < idx + 2 {
        error!("Usage: autotest-stage [compiler] stage_prefix command...");
        return Err("Invalid arguments for autotest-stage".to_string());
    }

    let stage_prefix = &args[idx];
    let command_args = &args[idx + 1..];

    // Disallow .c files in arguments
    if args.iter().any(|arg| arg.contains(".c")) {
        error!("autotest-stage does not accept .c file names in arguments.");
        error!("Please remove .c file references; they must already be in the directory.");
        return Err("Invalid .c files in arguments".to_string());
    }

    // Prepare paths
    let activities_dir = Path::new(
        config
            .get_custom_config("public_html_session_directory")
            .unwrap_or(&String::new()),
    )
    .join("activities");

    let autotest_script = Path::new("/usr/local/share/autotest/autotest.py");
    let c_check_path = Path::new(
        config
            .get_custom_config("public_html_session_directory")
            .unwrap_or(&String::new()),
    )
    .join("public/_infra/extern/c_check/c_check.py");

    // Determine compiler or default to clang
    let chosen_compiler = compiler.unwrap_or_else(|| "clang".to_string());

    let parameters = format!(
        "default_compilers = {{'c': [['{compiler}', '-Werror']]}} \
         default_checkers = {{'c': [['python3', '{}']]}}",
        c_check_path.display(),
        compiler = chosen_compiler
    );

    // First call: gather tests with --print_test_names
    let mut test_command = Command::new("python3");
    test_command
        .env(
            "PATH",
            extend_path_with_dir(env::var_os("PATH"), c_check_path.parent()),
        )
        .arg("-I")
        .arg(&autotest_script)
        .arg("--exercise_directory")
        .arg(&activities_dir)
        .arg("--parameters")
        .arg(&parameters)
        .args(command_args)
        .arg("--print_test_names")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = test_command.output().map_err(|e| {
        error!("Failed to run autotest command: {}", e);
        format!("Failed to execute autotest: {}", e)
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Autotest failed: {}", stderr);
        return Err(stderr.to_string());
    }

    // Parse JSON output to get test labels
    let json_output = String::from_utf8_lossy(&output.stdout);
    let json_value: Value = serde_json::from_str(&json_output).map_err(|e| {
        error!("Failed to parse JSON output: {}", e);
        error!("Output was: {}", json_output);
        "JSON parsing error".to_string()
    })?;

    let tests = json_value
        .get(0)
        .and_then(|obj| obj.get("labels"))
        .ok_or_else(|| {
            error!("Could not find 'labels' in autotest output");
            "No labels found in autotest output".to_string()
        })?;

    // Filter labels that start with the given stage_prefix
    let run_labels: Vec<String> = tests
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|label_val| label_val.as_str().map(str::to_string))
        .filter(|label_str| label_str.starts_with(stage_prefix))
        .collect();

    if run_labels.is_empty() {
        error!(
            "Could not find any autotests that start with {}!",
            stage_prefix
        );
        return Err(format!("No tests found with prefix '{}'", stage_prefix));
    }

    // Second call: run only these filtered labels
    let mut final_command = Command::new("python3");
    final_command
        .env(
            "PATH",
            extend_path_with_dir(env::var_os("PATH"), c_check_path.parent()),
        )
        .arg("-I")
        .arg(&autotest_script)
        .arg("--exercise_directory")
        .arg(&activities_dir)
        .arg("--parameters")
        .arg(&parameters)
        .args(command_args)
        .arg("-l")
        .args(run_labels);

    run_and_propagate_exit_status(final_command)
}

/// Utility to pick the compiler from arguments (dcc/gcc/clang) if present.
fn select_compiler(args: &[String]) -> (&str, Vec<String>) {
    if !args.is_empty() {
        let first_arg = args[0].as_str();
        if ["dcc", "gcc", "clang"].contains(&first_arg) {
            let mut remaining = args.to_vec();
            remaining.remove(0);
            (first_arg, remaining)
        } else {
            ("clang", args.to_vec())
        }
    } else {
        ("clang", vec![])
    }
}

/// Extend an existing PATH with an optional directory.
fn extend_path_with_dir(original_path: Option<std::ffi::OsString>, dir: Option<&Path>) -> String {
    let mut new_path = String::new();

    if let Some(path_val) = original_path {
        new_path.push_str(path_val.to_string_lossy().as_ref());
    }
    if let Some(dir_path) = dir {
        // Prepend `:` if original path was not empty
        if !new_path.is_empty() {
            new_path.push(':');
        }
        new_path.push_str(&dir_path.to_string_lossy());
    }
    new_path
}

/// Run the command and propagate its exit status if it fails.
/// Returns `Ok(())` if the command exits successfully, or an `Err` if it fails to start.
fn run_and_propagate_exit_status(mut command: Command) -> Result<(), String> {
    match command.status() {
        Ok(status) => {
            if !status.success() {
                exit(status.code().unwrap_or(1));
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to execute process: {}", e)),
    }
}