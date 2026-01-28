use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::config::{load_config, load_local_config, save_local_profile_selection, Config};
use crate::profile::{select_profile, AuthMethod, Profile};

/// Check if currently inside a git repository
pub fn is_inside_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the root path of the current git repository
pub fn get_git_root() -> Option<PathBuf> {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
}

/// Detect which profile is configured for the current repository
pub fn detect_profile(config: &Config) -> Option<&Profile> {
    // 1. Check local .gix/config.json
    if let Some(local_config) = load_local_config() {
        if let Some(name) = local_config.selected_profile {
            if let Some(p) = config.profiles.iter().find(|p| p.profile_name == name) {
                return Some(p);
            }
        }
    }

    // 2. Check global default profile
    if let Some(default_name) = &config.default_profile {
        if let Some(p) = config.profiles.iter().find(|p| &p.profile_name == default_name) {
            return Some(p);
        }
    }

    // 3. Fallback to git config
    if !is_inside_git_repo() {
        return None;
    }

    // Try to read local git config
    let output = Command::new("git")
        .args(["config", "--local", "user.email"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if email.is_empty() {
        return None;
    }

    config.profiles.iter().find(|p| p.email == email)
}

/// Apply profile configuration to the local repository
pub fn apply_local_config(profile: &Profile) -> Result<()> {
    // Save to .gix/config.json
    save_local_profile_selection(&profile.profile_name)?;

    // Configure git user settings
    Command::new("git")
        .args(["config", "--local", "user.name", &profile.name])
        .output()
        .context("Failed to set user.name")?;

    Command::new("git")
        .args(["config", "--local", "user.email", &profile.email])
        .output()
        .context("Failed to set user.email")?;

    // Configure authentication
    match &profile.auth {
        AuthMethod::SSH { key_path } => {
            let ssh_command = format!("ssh -i {} -o IdentitiesOnly=yes", key_path);
            Command::new("git")
                .args(["config", "--local", "core.sshCommand", &ssh_command])
                .output()
                .context("Failed to set core.sshCommand")?;
        }
        AuthMethod::Token { .. } => {
            // Unset SSH command if previously set
            Command::new("git")
                .args(["config", "--local", "--unset", "core.sshCommand"])
                .output()
                .ok(); // Ignore if not present
        }
    }

    Ok(())
}

/// Handle the 'gix use' command
pub fn handle_use_command(name: Option<String>) -> Result<()> {
    if !is_inside_git_repo() {
        println!("\x1b[1;31m✗ Not inside a git repository. Cannot apply local config.\x1b[0m");
        return Ok(());
    }

    let config = load_config()?;
    
    if config.profiles.is_empty() {
        println!("\x1b[1;33m⚠ No profiles configured. Run 'gix profile add' first.\x1b[0m");
        return Ok(());
    }

    let profile = if let Some(n) = name {
        config
            .profiles
            .iter()
            .find(|p| p.profile_name == n)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", n))?
            .clone()
    } else {
        select_profile(&config)
            .ok_or_else(|| anyhow::anyhow!("No profile selected"))?
            .clone()
    };

    apply_local_config(&profile)?;
    
    println!(
        "\n\x1b[1;32m✓ Switched to profile: {} ({})\x1b[0m",
        profile.profile_name, profile.email
    );
    
    Ok(())
}

/// Handle the 'gix status' command
pub fn handle_status_command() -> Result<()> {
    if !is_inside_git_repo() {
        println!("\x1b[1;33m⚠ Not inside a git repository.\x1b[0m");
        return Ok(());
    }

    let config = load_config()?;
    
    println!("\x1b[1;36m📊 Repository Status\x1b[0m\n");
    
    if let Some(root) = get_git_root() {
        println!("   📁 Repository: {}", root.display());
    }

    if let Some(profile) = detect_profile(&config) {
        println!(
            "   👤 Profile: \x1b[1;32m{}\x1b[0m",
            profile.profile_name
        );
        println!("   📧 Email: {}", profile.email);
        println!("   🏷️  Name: {}", profile.name);
        
        match &profile.auth {
            AuthMethod::SSH { key_path } => {
                let exists = PathBuf::from(key_path).exists();
                let status = if exists { "\x1b[1;32m✓\x1b[0m" } else { "\x1b[1;31m✗\x1b[0m" };
                println!("   🔐 Auth: SSH {} {}", key_path, status);
            }
            AuthMethod::Token { .. } => {
                println!("   🔑 Auth: HTTPS Token");
            }
        }
    } else {
        println!("   \x1b[1;33m⚠ No known profile detected.\x1b[0m");
        
        // Show raw git config
        let output = Command::new("git")
            .args(["config", "user.email"])
            .output()?;
        
        if output.status.success() {
            let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !email.is_empty() {
                println!("   📧 Git email: {}", email);
            }
        }
        
        println!("\n   Run '\x1b[1mgix use\x1b[0m' to configure a profile for this repository.");
    }
    
    println!();
    Ok(())
}

/// Handle git command passthrough with profile injection
pub fn handle_git_command(args: Vec<String>) -> Result<()> {
    let config = load_config()?;

    // Check if we should intercept this command
    if let Some(cmd) = args.first() {
        if !config.intercepted_commands.contains(cmd) {
            // Pass-through without interception
            let status = Command::new("git")
                .args(&args)
                .status()
                .context("Failed to run git command")?;
            
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
            return Ok(());
        }
    }

    // Interception logic
    let current_profile = detect_profile(&config);
    let is_clone = args.first().map(|s| s == "clone").unwrap_or(false);

    let profile = if let Some(p) = current_profile {
        // If we are cloning, we might want to confirm if we really want to use the default profile
        // but for now let's respect the default if it exists.
        println!(
            "\x1b[1;36m🔀 Using profile:\x1b[0m \x1b[1;32m{}\x1b[0m ({})",
            p.profile_name, p.email
        );
        
        // Warn if SSH key is missing
        if let AuthMethod::SSH { key_path } = &p.auth {
            if !PathBuf::from(key_path).exists() {
                println!(
                    "\x1b[1;33m⚠ Warning: SSH key not found at: {}\x1b[0m",
                    key_path
                );
            }
        }
        p.clone()
    } else {
        if is_clone {
             println!("\x1b[1;36m⬇️ Cloning repository...\x1b[0m");
             println!("\x1b[1;33m⚠ No default profile configured.\x1b[0m");
        } else {
             println!("\x1b[1;33m⚠ No profile detected for this repository.\x1b[0m");
        }
        
        let p = select_profile(&config)
            .ok_or_else(|| anyhow::anyhow!("No profile available"))?;

        // Ask to save persistence ONLY if we are inside a repo AND NOT cloning
        // If we are cloning, we handle persistence AFTER the clone
        if is_inside_git_repo() && !is_clone {
            let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Configure this repository to always use this profile?")
                .default(true)
                .interact()?;

            if confirm {
                apply_local_config(p)?;
                println!(
                    "\x1b[1;32m✓ Repository configured!\x1b[0m Future commands will use this profile."
                );
            }
        }
        p.clone()
    };

    // Log usage
    log_usage(&profile, &args)?;

    // Construct Git Command
    let mut git_cmd = Command::new("git");

    // Set authentication
    match &profile.auth {
        AuthMethod::SSH { key_path } => {
            let ssh_cmd = format!("ssh -i {} -o IdentitiesOnly=yes", key_path);
            git_cmd.env("GIT_SSH_COMMAND", ssh_cmd);
        }
        AuthMethod::Token { token } => {
            // Use git credential approve to inject token
            inject_token_credential(&profile.name, token)?;
        }
    }

    // Set user config for this command
    git_cmd.arg("-c").arg(format!("user.name={}", profile.name));
    git_cmd.arg("-c").arg(format!("user.email={}", profile.email));

    // Append original args
    git_cmd.args(&args);

    // Execute
    let status = git_cmd.status().context("Failed to run git command")?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    
    // Post-clone configuration
    if is_clone && status.success() {
        // Try to detect the directory created by git clone
        if let Some(dir) = detect_cloned_dir(&args) {
            println!("\x1b[1;36m⚙️  Configuring new repository...\x1b[0m");
            match crate::config::save_local_profile_selection_to_dir(&profile.profile_name, dir.clone()) {
                Ok(_) => {
                     // Also apply git local config
                     if let Err(e) = apply_local_config_to_dir(&profile, &dir) {
                         println!("\x1b[1;33m⚠ Failed to apply local git config: {}\x1b[0m", e);
                     } else {
                         println!("\x1b[1;32m✓ Repository '{}' configured with profile '{}'\x1b[0m", dir.display(), profile.profile_name);
                     }
                },
                Err(e) => println!("\x1b[1;33m⚠ Failed to save profile config: {}\x1b[0m", e),
            }
        }
    }

    Ok(())
}

/// Detect directory created by git clone
fn detect_cloned_dir(args: &[String]) -> Option<PathBuf> {
    // Determine the directory name
    // git clone [options] <repository> [<directory>]
    
    // 1. Check if the last arg is a directory (not strictly reliable if flags follow, but standard practice)
    if let Some(last) = args.last() {
        if !last.starts_with('-') && !last.starts_with("http") && !last.starts_with("git@") && !last.ends_with(".git") {
            // Likely a directory argument
            let path = PathBuf::from(last);
            if path.exists() && path.is_dir() {
                return Some(path);
            }
        }
    }
    
    // 2. Try to derive from repository URL
    // Find the arg that looks like a repo URL
    for arg in args.iter().rev() {
        if arg.ends_with(".git") || arg.starts_with("git@") || arg.starts_with("http") {
             // Extract name from URL
             // e.g. https://github.com/user/repo.git -> repo
             let name = arg.split('/').last()?
                .trim_end_matches(".git");
             
             let path = PathBuf::from(name);
             if path.exists() && path.is_dir() {
                 return Some(path);
             }
        }
    }
    
    None
}

/// Apply profile configuration to a specific directory
fn apply_local_config_to_dir(profile: &Profile, dir: &PathBuf) -> Result<()> {
    // Configure git user settings
    Command::new("git")
        .current_dir(dir)
        .args(["config", "--local", "user.name", &profile.name])
        .output()
        .context("Failed to set user.name")?;

    Command::new("git")
        .current_dir(dir)
        .args(["config", "--local", "user.email", &profile.email])
        .output()
        .context("Failed to set user.email")?;

    // Configure authentication
    match &profile.auth {
        AuthMethod::SSH { key_path } => {
            let ssh_command = format!("ssh -i {} -o IdentitiesOnly=yes", key_path);
            Command::new("git")
                .current_dir(dir)
                .args(["config", "--local", "core.sshCommand", &ssh_command])
                .output()
                .context("Failed to set core.sshCommand")?;
        }
        AuthMethod::Token { .. } => {
             Command::new("git")
                .current_dir(dir)
                .args(["config", "--local", "--unset", "core.sshCommand"])
                .output()
                .ok(); 
        }
    }

    Ok(())
}

/// Inject token credential into git credential cache
fn inject_token_credential(username: &str, token: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if url.starts_with("https://") {
                if let Some(host_start) = url.strip_prefix("https://") {
                    let host = host_start.split('/').next().unwrap_or("github.com");

                    let mut child = Command::new("git")
                        .args(["credential", "approve"])
                        .stdin(std::process::Stdio::piped())
                        .spawn()?;

                    if let Some(mut stdin) = child.stdin.take() {
                        write!(
                            stdin,
                            "protocol=https\nhost={}\nusername={}\npassword={}\n",
                            host, username, token
                        )?;
                    }
                    child.wait()?;
                }
            }
        }
    }

    Ok(())
}

/// Log profile usage to ~/.gix/usage.log
fn log_usage(profile: &Profile, args: &[String]) -> Result<()> {
    use chrono::Local;
    use std::fs::OpenOptions;

    let log_path = crate::config::get_gix_home_dir()?.join("usage.log");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let command = args.join(" ");
    let cwd = std::env::current_dir().unwrap_or_default();

    writeln!(
        file,
        "[{}] Profile: {} | Cmd: git {} | Dir: {:?}",
        timestamp, profile.profile_name, command, cwd
    )?;

    Ok(())
}

/// Handle commands configuration
pub fn handle_commands_config() -> Result<()> {
    use dialoguer::MultiSelect;

    let mut config = load_config()?;

    let all_commands = vec![
        "pull", "push", "clone", "fetch", "commit", "merge", "rebase", "checkout",
    ];

    let defaults: Vec<bool> = all_commands
        .iter()
        .map(|cmd| config.intercepted_commands.contains(&cmd.to_string()))
        .collect();

    println!("\x1b[1;36m⚙️  Configure Intercepted Commands\x1b[0m\n");
    println!("Select which git commands gix should intercept to apply profile settings:\n");

    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Commands to intercept")
        .items(&all_commands)
        .defaults(&defaults)
        .interact()?;

    config.intercepted_commands = selections
        .into_iter()
        .map(|i| all_commands[i].to_string())
        .collect();

    crate::config::save_config(&config)?;
    
    println!(
        "\n\x1b[1;32m✓ Updated intercepted commands:\x1b[0m {:?}",
        config.intercepted_commands
    );
    
    Ok(())
}
