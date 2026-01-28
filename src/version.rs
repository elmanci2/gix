use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

/// Current version of gix (from Cargo.toml)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for updates
pub const REPO_URL: &str = "https://github.com/elmanci2/gix";
pub const RELEASES_API: &str = "https://api.github.com/repos/elmanci2/gix/releases/latest";

/// Show version information
pub fn show_version() {
    println!("\x1b[1;36m🔀 gix\x1b[0m - Git Profile Manager");
    println!("   Version: \x1b[1;32m{}\x1b[0m", VERSION);
    println!("   Repository: {}", REPO_URL);
    println!();
}

/// Check for updates and optionally update
pub fn handle_update(force: bool) -> Result<()> {
    println!("\x1b[1;36m🔄 Checking for updates...\x1b[0m\n");

    // Try to get latest version from GitHub API
    match get_latest_version() {
        Ok(latest) => {
            let current = semver::Version::parse(VERSION)
                .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
            let latest_ver = semver::Version::parse(&latest)
                .unwrap_or_else(|_| semver::Version::new(0, 0, 0));

            println!("   Current version: \x1b[1m{}\x1b[0m", VERSION);
            println!("   Latest version:  \x1b[1m{}\x1b[0m", latest);

            if latest_ver > current || force {
                if latest_ver > current {
                    println!("\n\x1b[1;33m📦 New version available!\x1b[0m");
                } else {
                    println!("\n\x1b[1;32m✓ Already on latest version.\x1b[0m (force update requested)");
                }

                println!("\nTo update, run one of the following:");
                println!();
                println!("   \x1b[1m# Using the install script:\x1b[0m");
                println!("   curl -fsSL https://raw.githubusercontent.com/elmanci2/gix/main/install.sh | bash");
                println!();
                println!("   \x1b[1m# Using cargo:\x1b[0m");
                println!("   cargo install --git {} --force", REPO_URL);
                println!();
            } else {
                println!("\n\x1b[1;32m✓ You are running the latest version!\x1b[0m");
            }
        }
        Err(e) => {
            println!(
                "\x1b[1;33m⚠ Could not check for updates: {}\x1b[0m",
                e
            );
            println!("\nYou can manually check for updates at: {}/releases", REPO_URL);
        }
    }

    Ok(())
}

/// Get latest version from GitHub releases
fn get_latest_version() -> Result<String> {
    // Use curl to fetch from GitHub API (avoids needing reqwest dependency)
    let output = Command::new("curl")
        .args([
            "-sS",
            "-H", "Accept: application/vnd.github.v3+json",
            "-H", "User-Agent: gix-cli",
            RELEASES_API,
        ])
        .output()
        .context("Failed to check for updates. Make sure curl is installed.")?;

    if !output.status.success() {
        anyhow::bail!("Failed to fetch release information");
    }

    let body = String::from_utf8_lossy(&output.stdout);
    
    // Simple JSON parsing for tag_name
    if let Some(start) = body.find("\"tag_name\"") {
        let rest = &body[start..];
        if let Some(colon) = rest.find(':') {
            let after_colon = &rest[colon + 1..];
            let trimmed = after_colon.trim();
            if let Some(quote_start) = trimmed.find('"') {
                let after_quote = &trimmed[quote_start + 1..];
                if let Some(quote_end) = after_quote.find('"') {
                    let version = &after_quote[..quote_end];
                    // Remove 'v' prefix if present
                    return Ok(version.strip_prefix('v').unwrap_or(version).to_string());
                }
            }
        }
    }

    anyhow::bail!("Could not parse version from response")
}

/// Run diagnostics
pub fn handle_doctor() -> Result<()> {
    println!("\x1b[1;36m🩺 gix Doctor - System Diagnostics\x1b[0m\n");

    let mut all_ok = true;

    // Check git installation
    print!("   Checking git... ");
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("\x1b[1;32m✓\x1b[0m {}", version.trim());
        }
        _ => {
            println!("\x1b[1;31m✗ Git not found!\x1b[0m");
            all_ok = false;
        }
    }

    // Check ssh installation
    print!("   Checking ssh... ");
    match Command::new("ssh").arg("-V").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stderr);
            println!("\x1b[1;32m✓\x1b[0m {}", version.trim());
        }
        _ => {
            println!("\x1b[1;31m✗ SSH not found!\x1b[0m");
            all_ok = false;
        }
    }

    // Check config directory
    print!("   Checking config directory... ");
    match crate::config::get_gix_home_dir() {
        Ok(path) => {
            if path.exists() {
                println!("\x1b[1;32m✓\x1b[0m {}", path.display());
            } else {
                println!("\x1b[1;33m⚠\x1b[0m Not created yet (will be created on first use)");
            }
        }
        Err(_) => {
            println!("\x1b[1;31m✗ Could not determine config path\x1b[0m");
            all_ok = false;
        }
    }

    // Check config file
    print!("   Checking config file... ");
    match crate::config::get_global_config_path() {
        Ok(path) => {
            if path.exists() {
                // Check permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = fs::metadata(&path) {
                        let mode = meta.permissions().mode() & 0o777;
                        if mode <= 0o600 {
                            println!("\x1b[1;32m✓\x1b[0m {} (permissions: {:o})", path.display(), mode);
                        } else {
                            println!(
                                "\x1b[1;33m⚠\x1b[0m {} (permissions {:o} - consider chmod 600)",
                                path.display(),
                                mode
                            );
                        }
                    }
                }
                #[cfg(not(unix))]
                println!("\x1b[1;32m✓\x1b[0m {}", path.display());
            } else {
                println!("\x1b[1;33m⚠\x1b[0m Not created yet");
            }
        }
        Err(_) => {
            println!("\x1b[1;31m✗ Could not determine config path\x1b[0m");
            all_ok = false;
        }
    }

    // Check profiles
    print!("   Checking profiles... ");
    match crate::config::load_config() {
        Ok(config) => {
            if config.profiles.is_empty() {
                println!("\x1b[1;33m⚠\x1b[0m No profiles configured");
            } else {
                println!("\x1b[1;32m✓\x1b[0m {} profile(s) configured", config.profiles.len());
                
                // Validate each profile's SSH key
                for profile in &config.profiles {
                    if let crate::profile::AuthMethod::SSH { key_path } = &profile.auth {
                        let path = std::path::PathBuf::from(key_path);
                        if !path.exists() {
                            println!(
                                "      \x1b[1;33m⚠ Profile '{}': SSH key not found at {}\x1b[0m",
                                profile.profile_name, key_path
                            );
                            all_ok = false;
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("\x1b[1;31m✗ Error loading config: {}\x1b[0m", e);
            all_ok = false;
        }
    }

    // Check current repo
    print!("   Checking current directory... ");
    if crate::git::is_inside_git_repo() {
        println!("\x1b[1;32m✓\x1b[0m Inside a git repository");
    } else {
        println!("\x1b[1;33m⚠\x1b[0m Not inside a git repository");
    }

    println!();
    if all_ok {
        println!("\x1b[1;32m✓ All checks passed!\x1b[0m");
    } else {
        println!("\x1b[1;33m⚠ Some issues were found. Please review the output above.\x1b[0m");
    }

    Ok(())
}
