use clap::Parser;
use reqwest::blocking::Client;
use seu_wlan_login::{
    InfoError, Mode, get_account_interactively, get_password_interactively, login,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_enum, default_value_t = Mode::Env)]
    mode: Mode,

    #[arg(short, long)]
    account: Option<String>,

    #[arg(short, long)]
    password: Option<String>,
}

impl Args {
    fn resolve_credentials(&self) -> Result<(String, String), InfoError> {
        let account = match self.mode {
            Mode::Cli => self.account.clone().ok_or_else(|| {
                InfoError::UsernameNotProvidedError(
                    "mode is 'cli' but --account was not provided.".to_string(),
                )
            }),
            Mode::Env => std::env::var("SEU_WLAN_ACCOUNT").map_err(|_| {
                InfoError::UsernameNotProvidedError(
                    "environment variable 'SEU_WLAN_ACCOUNT' not found.".to_string(),
                )
            }),
            Mode::Inter => get_account_interactively(),
        }?;

        let password = match self.mode {
            Mode::Cli => self.password.clone().ok_or_else(|| {
                InfoError::PasswordNotProvidedError(
                    "mode is 'cli' but --password was not provided.".to_string(),
                )
            }),
            Mode::Env => std::env::var("SEU_WLAN_PASSWORD").map_err(|_| {
                InfoError::PasswordNotProvidedError(
                    "environment variable 'SEU_WLAN_PASSWORD' not found.".to_string(),
                )
            }),
            Mode::Inter => get_password_interactively(),
        }?;

        Ok((account, password))
    }
}

fn main() {
    let args = Args::parse();
    let (account, password) = match args.resolve_credentials() {
        Ok(creds) => creds,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    login(account, password, Client::new()).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    println!("Login successful!")
}
