use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use crate::profile::Profile;

/// Global configuration structure
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub profiles: Vec<Profile>,
    #[serde(default = "default_intercepted_commands")]
    pub intercepted_commands: Vec<String>,
    pub default_profile: Option<String>,
}

/// Default commands to intercept
fn default_intercepted_commands() -> Vec<String> {
    vec!["pull".to_string(), "push".to_string(), "fetch".to_string(), "clone".to_string()]
}

/// Local repository configuration
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LocalConfig {
    pub selected_profile: Option<String>,
}

/// Get the global configuration file path (~/.gix/config.json)
pub fn get_global_config_path() -> Result<PathBuf> {
    BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(".gix").join("config.json"))
        .context("Could not determine home directory")
}

/// Get the local repository configuration path (.gix/config.json)
pub fn get_local_config_path() -> PathBuf {
    PathBuf::from(".gix").join("config.json")
}

/// Load global configuration from file
pub fn load_config() -> Result<Config> {
    let path = get_global_config_path()?;
    
    if path.exists() {
        let file = File::open(&path).context("Failed to open config file")?;
        let reader = BufReader::new(file);
        let mut config: Config = serde_json::from_reader(reader)
            .context("Failed to parse config file. It may be corrupted.")?;
        
        // Ensure intercepted_commands has defaults if empty
        if config.intercepted_commands.is_empty() {
            config.intercepted_commands = default_intercepted_commands();
        }
        
        Ok(config)
    } else {
        Ok(Config {
            profiles: vec![],
            intercepted_commands: default_intercepted_commands(),
            default_profile: None,
        })
    }
}

/// Save global configuration to file with secure permissions
pub fn save_config(config: &Config) -> Result<()> {
    let path = get_global_config_path()?;
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Write config to file
    let file = File::create(&path)?;
    serde_json::to_writer_pretty(file, config)?;
    
    // Set secure permissions on Unix (readable only by owner)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }
    
    Ok(())
}

/// Load local repository configuration
pub fn load_local_config() -> Option<LocalConfig> {
    let path = get_local_config_path();
    
    if path.exists() {
        if let Ok(file) = File::open(&path) {
            let reader = BufReader::new(file);
            return serde_json::from_reader(reader).ok();
        }
    }
    
    None
}

/// Save local repository configuration
pub fn save_local_profile_selection(profile_name: &str) -> Result<()> {
    save_local_profile_selection_to_dir(profile_name, std::env::current_dir()?)
}

/// Save local repository configuration to a specific directory
pub fn save_local_profile_selection_to_dir(profile_name: &str, dir: PathBuf) -> Result<()> {
    let path = dir.join(".gix").join("config.json");
    
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let local_config = LocalConfig {
        selected_profile: Some(profile_name.to_string()),
    };
    
    let file = File::create(&path)?;
    serde_json::to_writer_pretty(file, &local_config)?;
    
    Ok(())
}

/// Get the gix directory in home
pub fn get_gix_home_dir() -> Result<PathBuf> {
    BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(".gix"))
        .context("Could not determine home directory")
}
