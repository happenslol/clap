//! Complete commands within shells

use std::io::Write;

/// Complete commands within bash
pub mod bash;
/// Completion code common to all shells
pub mod complete;
/// Complete commands within zsh
pub mod zsh;

#[derive(clap::Subcommand)]
#[clap(hide = true)]
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum CompleteCommand {
    /// Register shell completions for this program
    Completions(CompletionsArgs),

    #[clap(subcommand, hide(true), name = "__complete")]
    Complete(CompleteShell),
}

#[derive(Debug, Clone, clap::Args)]
#[allow(missing_docs)]
pub struct CompletionsArgs {
    /// Shell for which to write completion-registration
    #[clap(value_enum)]
    shell: CompletionsShell,

    /// Path to write completion-registration to
    out_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
#[allow(missing_docs)]
pub enum CompletionsShell {
    Bash,
    Zsh,
}

#[derive(Debug, Clone, clap::Subcommand)]
#[allow(missing_docs)]
pub enum CompleteShell {
    Bash(bash::CompleteArgs),
    Zsh(zsh::CompleteArgs),
}

impl CompleteCommand {
    /// Process the completion request
    pub fn run(&self, cmd: &mut clap::Command) -> std::convert::Infallible {
        self.try_run(cmd).unwrap_or_else(|e| e.exit());
        std::process::exit(0)
    }

    /// Process the completion request
    pub fn try_run(&self, cmd: &mut clap::Command) -> clap::Result<()> {
        debug!("CompleteCommand::try_complete: {:?}", self);

        use CompleteCommand::*;
        match self {
            Completions(args) => register(cmd, args),

            Complete(CompleteShell::Bash(args)) => bash::complete(cmd, args),
            Complete(CompleteShell::Zsh(_args)) => todo!(),
        }
    }
}

fn register(cmd: &mut clap::Command, args: &CompletionsArgs) -> clap::Result<()> {
    let mut buf = Vec::new();
    let name = cmd.get_name();
    let bin = cmd.get_bin_name().unwrap_or_else(|| cmd.get_name());

    match args.shell {
        CompletionsShell::Bash => {
            bash::register(name, [bin], bin, &bash::Behavior::default(), &mut buf)?
        }
        CompletionsShell::Zsh => zsh::register(name, [bin], bin, &mut buf)?,
    };

    let write_stdout = match &args.out_path {
        Some(path) if path == std::path::Path::new("-") => true,
        None => true,
        _ => false
    };

    if write_stdout {
        std::io::stdout().write_all(&buf)?;
        return Ok(())
    }

    let out_path = args.out_path.as_ref().unwrap();
    if out_path.is_dir() {
        let filename = match args.shell {
            CompletionsShell::Bash => bash::file_name(name),
            CompletionsShell::Zsh => zsh::file_name(name),
        };

        let out_path = out_path.join(filename);
        std::fs::write(out_path, buf)?;
    } else {
        std::fs::write(out_path, buf)?;
    }

    Ok(())
}
