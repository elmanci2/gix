//! # gix - Git Profile Manager
//!
//! A powerful CLI tool for managing multiple Git identities with different
//! SSH keys and user configurations.
//!
//! ## Features
//! - Multiple profile management (work, personal, open source)
//! - SSH key and HTTPS token authentication
//! - Automatic profile detection per repository
//! - Seamless git command interception

mod cli;
mod config;
mod git;
mod profile;
mod version;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Commands};
use git::{handle_commands_config, handle_git_command, handle_status_command, handle_use_command};
use profile::handle_profile_command;
use version::{handle_doctor, handle_update, show_version};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Profile { action }) => handle_profile_command(action),
        Some(Commands::Commands) => handle_commands_config(),
        Some(Commands::Use { name }) => handle_use_command(name),
        Some(Commands::Status) => handle_status_command(),
        Some(Commands::Version) => {
            show_version();
            Ok(())
        }
        Some(Commands::Update { force }) => handle_update(force),
        Some(Commands::Doctor) => handle_doctor(),
        None => {
            if cli.git_args.is_empty() {
                // If no args, show help
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
                Ok(())
            } else {
                handle_git_command(cli.git_args)
            }
        }
    }
}
