use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

/// Stores configuration for a specific class
#[derive(Debug)]
pub struct ClassConfig {
    pub class: String,
    pub account_name: Option<String>,
    pub home_dir: Option<String>,
    pub bin_path: Option<String>,
    pub man_path: Option<String>,
    pub newclassrc_path: Option<String>,
    pub custom_config: HashMap<String, String>,
}

impl ClassConfig {
    /// Create a new ClassConfig from a class code
    pub fn new(class_code: &str) -> Option<Self> {
        // Parse the class code into a proper class name
        let class = parse_class_code(class_code)?;

        // Derive the account name
        let account_name = derive_account_name(&class);

        // Create base configuration
        let mut config = ClassConfig {
            class,
            account_name,
            home_dir: None,
            bin_path: None,
            man_path: None,
            newclassrc_path: None,
            custom_config: HashMap::new(),
        };

        // If we have an account name, derive the other paths
        if let Some(account) = &config.account_name {
            let home_dir = format!("/home/{}", account);
            config.home_dir = Some(home_dir.clone());
            config.bin_path = Some(format!("{}/bin", home_dir));
            config.man_path = Some(format!("{}/man", home_dir));
            config.newclassrc_path = Some(format!("{}/.newclassrc", home_dir));
        }

        Some(config)
    }

    /// Parse a bash script and load all environment variables into custom_config
    pub fn load_bash_config(&mut self, file_path: &str) -> io::Result<()> {
        let file = File::open(file_path)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;

            // Skip comments, empty lines, and common bash constructs
            if line.trim().is_empty()
                || line.trim().starts_with('#')
                || line.trim().starts_with("unset ")
                || line.trim().starts_with("export ")
                || line.starts_with("#!/")
            {
                continue;
            }

            // Extract variable assignments
            if let Some((name, value)) = self.parse_variable_assignment(&line) {
                self.custom_config.insert(name, value);
            }
        }

        Ok(())
    }

    /// Parse a variable assignment line from a bash script
    fn parse_variable_assignment(&self, line: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            let name = parts[0].trim().to_string();
            let mut value = parts[1].trim().to_string();

            // Handle quoted values
            if (value.starts_with('\'') && value.ends_with('\''))
                || (value.starts_with('"') && value.ends_with('"'))
            {
                value = value[1..value.len() - 1].to_string();
            }

            return Some((name, value));
        }
        None
    }

    /// Get a custom configuration value
    pub fn get_custom_config(&self, key: &str) -> Option<&String> {
        self.custom_config.get(key)
    }

    /// Check if the class has a .newclassrc file
    pub fn has_newclassrc(&self) -> bool {
        if let Some(path) = &self.newclassrc_path {
            Path::new(path).exists()
        } else {
            false
        }
    }

    /// Get configured PATH for this class
    pub fn get_path(&self, original_path: &str) -> String {
        if let Some(bin_path) = &self.bin_path {
            format!("{}:{}", bin_path, original_path)
        } else {
            original_path.to_string()
        }
    }

    /// Get configured MANPATH for this class
    pub fn get_manpath(&self, original_manpath: &str) -> String {
        if let Some(man_path) = &self.man_path {
            format!("{}:{}", man_path, original_manpath)
        } else {
            original_manpath.to_string()
        }
    }
}

/// Parse a class code into a full class name
pub fn parse_class_code(code: &str) -> Option<String> {
    match code {
        c if c.starts_with("109") && c.len() == 4 => Some(format!("DPST{}", c)),
        c if c.len() == 4 && c.chars().all(|ch| ch.is_digit(10)) => Some(format!("COMP{}", c)),
        c if c.starts_with("cs") && c.len() == 6 && c[2..].chars().all(|ch| ch.is_digit(10)) => {
            Some(format!("COMP{}", &c[2..]))
        }
        c if c.len() == 8
            && c[0..4].chars().all(|ch| ch.is_alphabetic())
            && c[4..].chars().all(|ch| ch.is_digit(10)) =>
        {
            Some(c.to_uppercase())
        }
        _ => None,
    }
}

/// Derive account name from class name
fn derive_account_name(class: &str) -> Option<String> {
    if class.len() < 8 {
        return None;
    }

    let prefix = &class[0..4];
    let number = &class[4..8];

    match prefix {
        "COMP" => Some(format!("cs{}", number)),
        "SENG" => Some(format!("se{}", number)),
        "BINF" => Some(format!("bi{}", number)),
        "DPST" => Some(format!("dp{}", number)),
        "ENGG" => Some(format!("en{}", number)),
        "GENE" => Some(format!("ge{}", number)),
        "GSOE" => Some(format!("gs{}", number)),
        "HSCH" => Some(format!("hs{}", number)),
        "INFS" => Some(format!("is{}", number)),
        "REGZ" => Some(format!("rz{}", number)),
        _ => None,
    }
}