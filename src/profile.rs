use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::config::{load_config, save_config, Config};

/// Authentication method for Git operations
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthMethod {
    SSH { key_path: String },
    Token { token: String },
}

/// User profile containing Git identity and authentication
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub email: String,
    pub auth: AuthMethod,
    pub profile_name: String,
}

impl Profile {
    /// Validate the profile configuration
    pub fn validate(&self) -> Result<()> {
        // Validate email format (basic check)
        if !self.email.contains('@') || !self.email.contains('.') {
            anyhow::bail!("Invalid email format: {}", self.email);
        }

        // Validate profile name (no special characters that could cause issues)
        if self.profile_name.is_empty() {
            anyhow::bail!("Profile name cannot be empty");
        }

        if self.profile_name.contains('/') || self.profile_name.contains('\\') {
            anyhow::bail!("Profile name cannot contain path separators");
        }

        // Validate SSH key if applicable
        if let AuthMethod::SSH { key_path } = &self.auth {
            self.validate_ssh_key(key_path)?;
        }

        Ok(())
    }

    /// Validate SSH key exists and has proper permissions
    fn validate_ssh_key(&self, key_path: &str) -> Result<()> {
        let path = PathBuf::from(key_path);
        
        if !path.exists() {
            anyhow::bail!("SSH key not found at: {}", key_path);
        }

        if !path.is_file() {
            anyhow::bail!("SSH key path is not a file: {}", key_path);
        }

        // Check file permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&path)?;
            let mode = metadata.permissions().mode();
            let perms = mode & 0o777;
            
            // SSH keys should have permissions 600 or 400
            if perms > 0o600 {
                println!("\x1b[1;33m⚠ Warning: SSH key has insecure permissions ({:o}). Consider running: chmod 600 {}\x1b[0m", perms, key_path);
            }
        }

        Ok(())
    }

    /// Get SSH key path if using SSH authentication
    #[allow(dead_code)]
    pub fn get_ssh_key_path(&self) -> Option<&str> {
        match &self.auth {
            AuthMethod::SSH { key_path } => Some(key_path),
            AuthMethod::Token { .. } => None,
        }
    }
}

/// List available SSH keys in ~/.ssh directory
pub fn list_ssh_keys() -> Vec<String> {
    if let Some(base_dirs) = BaseDirs::new() {
        let ssh_dir = base_dirs.home_dir().join(".ssh");
        if let Ok(entries) = fs::read_dir(ssh_dir) {
            return entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    if let Some(ext) = path.extension() {
                        ext != "pub" && ext != "known_hosts"
                    } else {
                        let filename = path.file_name().unwrap_or_default().to_string_lossy();
                        path.is_file() 
                            && !filename.starts_with("known_hosts")
                            && !filename.starts_with("config")
                            && !filename.starts_with("authorized_keys")
                    }
                })
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
        }
    }
    vec![]
}

/// Interactive profile selection
pub fn select_profile(config: &Config) -> Option<&Profile> {
    if config.profiles.is_empty() {
        println!("\x1b[1;33m⚠ No profiles configured. Run 'gix profile add' to create one.\x1b[0m");
        return None;
    }

    let selections: Vec<String> = config.profiles.iter()
        .map(|p| format!("{} ({} <{}>)", p.profile_name, p.name, p.email))
        .collect();
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("🔀 Select Git Profile")
        .default(0)
        .items(&selections)
        .interact()
        .unwrap_or(0);
    
    Some(&config.profiles[selection])
}

/// Handle profile-related commands
pub fn handle_profile_command(action: crate::cli::ProfileAction) -> Result<()> {
    let mut config = load_config()?;

    match action {
        crate::cli::ProfileAction::List => {
            if config.profiles.is_empty() {
                println!("\x1b[1;33m📋 No profiles configured.\x1b[0m");
                println!("   Run '\x1b[1mgix profile add\x1b[0m' to create your first profile.");
            } else {
                println!("\x1b[1;36m📋 Configured profiles:\x1b[0m\n");
                for (i, profile) in config.profiles.iter().enumerate() {
                    let auth_info = match &profile.auth {
                        AuthMethod::SSH { key_path } => {
                            let key_exists = PathBuf::from(key_path).exists();
                            let status = if key_exists { "✓" } else { "✗" };
                            format!("SSH: {} {}", key_path, status)
                        }
                        AuthMethod::Token { .. } => "Token: ••••••••".to_string(),
                    };
                    println!(
                        "  \x1b[1;32m{}\x1b[0m. \x1b[1m{}\x1b[0m",
                        i + 1,
                        profile.profile_name
                    );
                    println!("     👤 {} <{}>", profile.name, profile.email);
                    println!("     🔑 {}\n", auth_info);
                }
            }
        }
        crate::cli::ProfileAction::Add => {
            println!("\x1b[1;36m➕ Add New Profile\x1b[0m\n");

            let profile_name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Profile Name (e.g. Work, Personal)")
                .interact_text()?;

            // Check for duplicate names
            if config.profiles.iter().any(|p| p.profile_name == profile_name) {
                anyhow::bail!("A profile with name '{}' already exists", profile_name);
            }

            let user_name: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Git User Name")
                .interact_text()?;

            let email: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Git User Email")
                .validate_with(|input: &String| {
                    if input.contains('@') && input.contains('.') {
                        Ok(())
                    } else {
                        Err("Please enter a valid email address")
                    }
                })
                .interact_text()?;

            // Auth Method Selection
            let auth_methods = vec!["🔐 SSH Key", "🔑 HTTPS Token"];
            let auth_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Authentication Method")
                .items(&auth_methods)
                .default(0)
                .interact()
                .unwrap_or(0);

            let auth = if auth_selection == 0 {
                create_ssh_auth(&email)?
            } else {
                create_token_auth()?
            };

            let new_profile = Profile {
                profile_name,
                name: user_name,
                email,
                auth,
            };

            // Validate before saving
            new_profile.validate()?;

            config.profiles.push(new_profile);
            save_config(&config)?;
            println!("\n\x1b[1;32m✓ Profile added successfully!\x1b[0m");
        }
        crate::cli::ProfileAction::Delete { name } => {
            let profile_name = if let Some(n) = name {
                n
            } else {
                let selections: Vec<&String> = config.profiles.iter().map(|p| &p.profile_name).collect();
                if selections.is_empty() {
                    println!("\x1b[1;33m⚠ No profiles to delete.\x1b[0m");
                    return Ok(());
                }
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("🗑️  Select profile to DELETE")
                    .items(&selections)
                    .interact()?;
                selections[selection].clone()
            };

            // Double confirmation for safety
            println!("\n\x1b[1;31m⚠ WARNING: This action cannot be undone!\x1b[0m");
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Are you sure you want to delete '{}'?", profile_name))
                .default(false)
                .interact()?
            {
                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Type 'yes' to confirm deletion")
                    .default(false)
                    .interact()?
                {
                    config.profiles.retain(|p| p.profile_name != profile_name);
                    save_config(&config)?;
                    println!("\x1b[1;32m✓ Profile deleted.\x1b[0m");
                } else {
                    println!("Deletion cancelled.");
                }
            } else {
                println!("Deletion cancelled.");
            }
        }
        crate::cli::ProfileAction::Edit { name } => {
            let profile_name = if let Some(n) = name {
                n
            } else {
                let selections: Vec<&String> = config.profiles.iter().map(|p| &p.profile_name).collect();
                if selections.is_empty() {
                    println!("\x1b[1;33m⚠ No profiles to edit.\x1b[0m");
                    return Ok(());
                }
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("✏️  Select profile to EDIT")
                    .items(&selections)
                    .interact()?;
                selections[selection].clone()
            };

            if let Some(idx) = config.profiles.iter().position(|p| p.profile_name == profile_name) {
                let p = &mut config.profiles[idx];

                println!("\x1b[1;36m✏️  Editing profile: {}\x1b[0m\n", p.profile_name);

                p.profile_name = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Profile Name")
                    .default(p.profile_name.clone())
                    .interact_text()?;

                p.name = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Git User Name")
                    .default(p.name.clone())
                    .interact_text()?;

                p.email = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Git User Email")
                    .default(p.email.clone())
                    .validate_with(|input: &String| {
                        if input.contains('@') && input.contains('.') {
                            Ok(())
                        } else {
                            Err("Please enter a valid email address")
                        }
                    })
                    .interact_text()?;

                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Update authentication settings?")
                    .default(false)
                    .interact()?
                {
                    let auth_methods = vec!["🔐 SSH Key", "🔑 HTTPS Token"];
                    let auth_selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Authentication Method")
                        .items(&auth_methods)
                        .default(0)
                        .interact()
                        .unwrap_or(0);

                    if auth_selection == 0 {
                        p.auth = create_ssh_auth(&p.email)?;
                    } else {
                        p.auth = create_token_auth()?;
                    }
                }

                // Validate before saving
                p.validate()?;

                save_config(&config)?;
                println!("\n\x1b[1;32m✓ Profile updated.\x1b[0m");
            } else {
                println!("\x1b[1;31m✗ Profile not found.\x1b[0m");
            }
        }
    }
    Ok(())
}

/// Handle the 'gix set' command to configure global default profile
pub fn handle_set_command(name: Option<String>) -> Result<()> {
    let mut config = load_config()?;

    if config.profiles.is_empty() {
        println!("\x1b[1;33m⚠ No profiles configured. Run 'gix profile add' first.\x1b[0m");
        return Ok(());
    }

    if let Some(n) = name {
        // Find profile by name
        if !config.profiles.iter().any(|p| p.profile_name == n) {
            anyhow::bail!("Profile '{}' not found", n);
        }
        
        config.default_profile = Some(n.clone());
        save_config(&config)?;
        println!("\x1b[1;32m✓ Global default profile set to: {}\x1b[0m", n);
    } else {
        // Interactive selection
        let mut selections: Vec<String> = config.profiles.iter()
            .map(|p| format!("{} ({} <{}>)", p.profile_name, p.name, p.email))
            .collect();
        
        // Add option to unset default
        selections.push("🚫 No default (Clear)".to_string());
        
        // Determine current default index
        let default_idx = if let Some(def) = &config.default_profile {
            config.profiles.iter().position(|p| &p.profile_name == def).unwrap_or(0)
        } else {
            0
        };

        println!("\x1b[1;36m🌍 Select Global Default Profile\x1b[0m\n");
        println!("This profile will be used for repositories that don't have a specific gix profile configured.\n");

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select default profile")
            .default(default_idx)
            .items(&selections)
            .interact()?;

        if selection == selections.len() - 1 {
            // "No default" selected
            config.default_profile = None;
            save_config(&config)?;
            println!("\x1b[1;32m✓ Global default profile cleared.\x1b[0m");
        } else {
            let profile = &config.profiles[selection];
            config.default_profile = Some(profile.profile_name.clone());
            save_config(&config)?;
            println!("\x1b[1;32m✓ Global default profile set to: {}\x1b[0m", profile.profile_name);
        }
    }

    Ok(())
}

/// Create SSH authentication configuration
fn create_ssh_auth(email: &str) -> Result<AuthMethod> {
    let mut keys = list_ssh_keys();
    keys.push("🆕 Create new SSH key".to_string());
    keys.push("📁 Custom path...".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select SSH Key")
        .items(&keys)
        .default(0)
        .interact()
        .unwrap_or(0);

    let ssh_key = if selection == keys.len() - 2 {
        // Create New SSH Key
        let key_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Key Name (e.g. id_ed25519_work)")
            .interact_text()?;

        let passphrase: String = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Passphrase (empty for none)")
            .allow_empty_password(true)
            .interact()?;

        let home = BaseDirs::new()
            .context("Could not determine home directory")?
            .home_dir()
            .to_path_buf();
        let key_path = home.join(".ssh").join(&key_name);
        let key_path_str = key_path.to_string_lossy().to_string();

        // Ensure .ssh directory exists
        fs::create_dir_all(home.join(".ssh"))?;

        let mut cmd = Command::new("ssh-keygen");
        cmd.args(["-t", "ed25519", "-f", &key_path_str, "-C", email]);
        if passphrase.is_empty() {
            cmd.args(["-N", ""]);
        } else {
            cmd.args(["-N", &passphrase]);
        }

        let status = cmd.status().context("Failed to generate SSH key")?;
        if !status.success() {
            anyhow::bail!("ssh-keygen failed");
        }

        println!("\n\x1b[1;32m✓ SSH key generated at: {}\x1b[0m", key_path_str);
        println!("\n\x1b[1;36m📋 Add this public key to your Git provider:\x1b[0m\n");
        println!("{}", fs::read_to_string(format!("{}.pub", key_path_str))?);

        key_path_str
    } else if selection == keys.len() - 1 {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter SSH Key Path")
            .interact_text()?
    } else {
        keys[selection].clone()
    };

    Ok(AuthMethod::SSH { key_path: ssh_key })
}

/// Create token authentication configuration
fn create_token_auth() -> Result<AuthMethod> {
    let token: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Personal Access Token")
        .interact()?;

    if token.is_empty() {
        anyhow::bail!("Token cannot be empty");
    }

    Ok(AuthMethod::Token { token })
}
