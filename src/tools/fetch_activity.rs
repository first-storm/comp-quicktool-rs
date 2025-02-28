use colored::Colorize;
use log::info;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::config::ClassConfig;

/// Run the fetch-activity tool to copy or link activity starter files
pub fn run_fetch_activity(config: &mut ClassConfig, args: &[String]) -> Result<(), String> {
    // Ensure we have at least one argument (the activity name)
    if args.is_empty() {
        let course_number = config.class.clone();
        println!("usage: {} fetch-activity activity", course_number);
        return Err(format!("usage: {} fetch-activity activity", course_number));
    }

    let activity_name = &args[0];

    // Path to the "fetch-activity" symlink, which we'll use to find config.sh
    let bin_path = config.bin_path.as_deref().unwrap_or("");
    let original_fetch_activity_softlink = Path::new(bin_path).join("fetch-activity");

    // Ensure fetch-activity exists
    let fetch_activity_path = fs::canonicalize(&original_fetch_activity_softlink)
        .map_err(|e| format!("Failed to canonicalize fetch-activity path: {}", e))?;

    // DEBUG: Print the resolved path
    info!("Resolved fetch-activity path: {:?}", fetch_activity_path);

    // Load the config from config.sh
    let config_sh = fetch_activity_path
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new(""))
        .join("config.sh");

    config
        .load_bash_config(config_sh.to_string_lossy().as_ref())
        .map_err(|e| format!("Could not load bash config: {}", e))?;

    // Build path to the activity directory
    let course_account = config
        .get_custom_config("course_account")
        .ok_or_else(|| "course_account not found in config".to_string())?;

    let activities_path = Path::new("/web")
        .join(course_account)
        .join("current")
        .join("activities")
        .join(activity_name);

    // Ensure activity directory exists
    if !activities_path.exists() {
        println!(
            "Exercise '{}' does not exist. Make sure you spelt it correctly!",
            activity_name
        );
        return Err(format!("Exercise '{}' does not exist", activity_name));
    }

    // Check for files directories
    let files_dir = activities_path.join("files");
    let files_ln_dir = activities_path.join("files.ln");
    let files_cp_dir = activities_path.join("files.cp");

    if files_dir.exists() || files_ln_dir.exists() || files_cp_dir.exists() {
        // Copy files from files/ and files.cp/ directories
        copy_files_from_dirs(&[&files_dir, &files_cp_dir])?;

        // Link files from files.ln/ directory
        link_files_from_dir(&files_ln_dir)?;
    } else {
        // Check for main activity file
        let main_file = activities_path.join(format!("{}.c", activity_name));

        if !main_file.exists() {
            println!(
                "Exercise '{}' does not have any starter code.",
                activity_name
            );
            return Err(format!("No starter code for '{}'", activity_name));
        }

        let target_file_name = activity_name.to_string() + ".c";
        let target_file = Path::new(&target_file_name);
        if target_file.exists() {
            println!(
                "The file '{}.c' already exists in this directory!",
                activity_name
            );
            return Err(format!("File '{}.c' already exists", activity_name));
        }

        // Copy the main file
        fs::copy(&main_file, target_file)
            .map_err(|e| format!("Failed to copy file {}.c: {}", activity_name, e))?;
    }

    println!(
        "Copied '{}' starter code successfully!",
        activity_name.green().bold()
    );
    Ok(())
}

/// Copy files from multiple directories if they exist
fn copy_files_from_dirs(dirs: &[&Path]) -> Result<(), String> {
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }

        for entry in WalkDir::new(dir)
            .follow_links(true)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let file_path = entry.path();
                let file_name = file_path.file_name().unwrap_or_else(|| OsStr::new(""));
                let target_path = Path::new(file_name);

                if target_path.exists() {
                    println!(
                        "The file {} already exists in this directory",
                        file_name.to_string_lossy().red().bold()
                    );
                } else {
                    println!("Copying {}", file_name.to_string_lossy().red().bold());
                    fs::copy(file_path, target_path).map_err(|e| {
                        format!("Failed to copy file {}: {}", file_name.to_string_lossy(), e)
                    })?;
                }
            }
        }
    }
    Ok(())
}

/// Create symlinks to files in the source directory
fn link_files_from_dir(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let file_path = entry.path();
            let file_name = file_path.file_name().unwrap_or_else(|| OsStr::new(""));
            let target_path = Path::new(file_name);

            if target_path.exists() {
                println!(
                    "The file {} already exists in this directory",
                    file_name.to_string_lossy().red().bold()
                );
            } else {
                println!("Linking {}", file_name.to_string_lossy().red().bold());

                std::os::unix::fs::symlink(file_path, target_path).map_err(|e| {
                    format!("Failed to link file {}: {}", file_name.to_string_lossy(), e)
                })?;
            }
        }
    }
    Ok(())
}
