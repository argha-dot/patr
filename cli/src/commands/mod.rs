use std::{fmt::Display, io::Write};

use clap::{Args, Parser, Subcommand, ValueEnum};

use self::{login::LoginArgs, workspaced::WorkspacedCommands};

/// The command to get information about the current logged in user.
mod info;
/// The command to login to your Patr account.
mod login;
/// The command to logout of your Patr account.
mod logout;
/// All commands that are meant for a workspace.
mod workspaced;

/// A trait that defines the functionality of a command.
/// Every command must implement this trait.
#[async_trait::async_trait]
pub trait CommandExecutor {
	async fn execute(
		self,
		global_args: &GlobalArgs,
		output_writer: impl Write + Send,
	) -> anyhow::Result<()>;
}

/// A list of all possible output types generated by the CLI.
#[derive(Debug, Clone, Default, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum OutputType {
	/// A plain text output.
	#[default]
	Text,
	/// A JSON output, minified.
	Json,
	/// A JSON output, pretty printed.
	PrettyJson,
}

impl Display for OutputType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			OutputType::Text => write!(f, "text"),
			OutputType::Json => write!(f, "json"),
			OutputType::PrettyJson => write!(f, "pretty-json"),
		}
	}
}

/// A list of all the arguments that can be passed to the CLI.
#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct AppArgs {
	/// All global arguments that can be used across all commands.
	#[command(flatten)]
	pub global_args: GlobalArgs,
	/// A command that is called on the CLI.
	#[command(subcommand)]
	pub command: GlobalCommands,
}

/// A global list of all the arguments that can be passed to the CLI.
#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
	/// The output type of each command. Defaults to text.
	#[arg(short = 'o', default_value_t = OutputType::Text)]
	pub output: OutputType,
}

/// A list of all the commands that can be called on the CLI.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GlobalCommands {
	/// Login to your Patr account.
	#[command(alias = "signin", alias = "sign-in")]
	Login(LoginArgs),
	/// Logout of your Patr account.
	Logout,
	/// Get information about the current logged in user.
	#[command(alias = "whoami")]
	Info,
	/// All the commands that are meant for a workspace
	#[command(flatten)]
	Workspaced(WorkspacedCommands),
}

#[async_trait::async_trait]
impl CommandExecutor for GlobalCommands {
	async fn execute(
		self,
		global_args: &GlobalArgs,
		writer: impl Write + Send,
	) -> anyhow::Result<()> {
		match self {
			Self::Login(args) => {
				login::execute(global_args, args, writer).await
			}
			Self::Logout => logout::execute(global_args).await,
			Self::Info => info::execute(global_args).await,
			Self::Workspaced(commands) => {
				commands.execute(global_args, writer).await
			}
		}
	}
}
