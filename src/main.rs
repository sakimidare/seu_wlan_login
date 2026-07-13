use std::ops::Deref;
use clap::Parser;
use reqwest::blocking::Client;
use seu_wlan_login::{get_account_interactively, get_password_interactively, login, Mode};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "env")]
    mode: String,
    #[arg(short, long)]
    account: Option<String>,
    #[arg(short, long)]
    password: Option<String>
}


fn main() {
    let args = Args::parse();
    let mode = Mode::from(args.mode.deref());
    let account = match mode {
        Mode::Cli => args.account.expect("--account does not provide an account"),
        Mode::Env => std::env::var("SEU_WLAN_ACCOUNT").expect("SEU_WLAN_ACCOUNT variable does not exist"),
        Mode::Inter => get_account_interactively().unwrap(),
        Mode::Other(s) => panic!("unknown mode")
    };

    let password = match mode {
        Mode::Cli => args.password.expect("--password does not provide a password"),
        Mode::Env => std::env::var("SEU_WLAN_PASSWORD").expect("SEU_WLAN_PASSWORD variable does not exist"),
        Mode::Inter => get_password_interactively().unwrap(),
        Mode::Other(s) => unreachable!()
    };
    login(account, password, Client::new()).unwrap();
}