use clap::{Parser, Subcommand};

/// CLI structure and command definitions for gix
#[derive(Parser, Debug)]
#[command(name = "gix")]
#[command(version)]
#[command(author = "elmanci2")]
#[command(about = "🔀 A powerful Git profile manager for switching between SSH keys and user configurations")]
#[command(long_about = r#"
gix - Git Profile Manager

Easily manage multiple Git identities (work, personal, open source) with different
SSH keys and user configurations. Switch between profiles seamlessly.

EXAMPLES:
    gix profile add          Add a new profile
    gix profile list         List all profiles
    gix use                   Select and apply a profile to current repo
    gix status               Show current profile in use
    gix push                 Push with the correct profile
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// If no subcommand is provided, these args are passed to git
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub git_args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage profiles (add, list, edit, delete)
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// Configure which git commands to intercept
    Commands,
    /// Switch to a specific profile (configures current repo)
    Use {
        /// Name of the profile to use
        name: Option<String>,
    },
    /// Set a global default profile
    Set {
        /// Name of the profile to set as default
        name: Option<String>,
    },
    /// Show current profile status
    Status,
    /// Show version information
    Version,
    /// Check for updates and update gix
    Update {
        /// Force update even if already on latest version
        #[arg(short, long)]
        force: bool,
    },
    /// Run diagnostics to check gix setup
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    /// List all configured profiles
    List,
    /// Add a new profile
    Add,
    /// Edit an existing profile
    Edit {
        /// Name of the profile to edit
        name: Option<String>,
    },
    /// Delete a profile
    Delete {
        /// Name of the profile to delete
        name: Option<String>,
    },
}
