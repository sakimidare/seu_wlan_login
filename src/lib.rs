use base64::prelude::*;
use clap::ValueEnum;
use regex::Regex;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq)]
#[clap(rename_all = "kebab-case")]
pub enum Mode {
    Cli,
    Env,
    Inter,
}

#[derive(Error, Debug)]
pub enum InfoError {
    #[error("error reading credentials interactively: {0}")]
    IoError(#[from] io::Error),
    #[error("username is not provided: {0}")]
    UsernameNotProvidedError(String),
    #[error("password is not provided: {0}")]
    PasswordNotProvidedError(String),
    #[error("username is invalid")]
    UsernameInvalidError,
    #[error("password is invalid")]
    PasswordInvalidError,
}

#[derive(Error, Debug)]
pub enum LoginError {
    #[error("failed to connect to login site: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("response was {0}: {1}")]
    CodeError(u16, String),
    #[error("parsing GET text error")]
    ParseError,
    #[error("parsing json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("decode base64 error: {0}")]
    DecodeError(#[from] base64::DecodeError),
    #[error("base64 bytes to string error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("you have already logged in")]
    AlreadyLoginError,
    #[error("username or password error")]
    UsernameOrPasswordError,
    #[error("the username does not exist")]
    UsernameDoesNotExistError,
    #[error("the password is wrong")]
    PasswordError,
    #[error("unknown error")]
    UnknownError,
}

#[derive(Deserialize, Debug)]
struct WlanStatus {
    result: i32,
    v46ip: String,
    #[serde(flatten)]
    _extra_field: HashMap<String, Value>,
}

#[derive(Deserialize, Debug)]
struct LoginStatus {
    result: String,
    msg: String,
    _ret_code: i32,
}

// dr1003({"result":"0","msg":"bGRhcCBhdXRoIGVycm9y","ret_code":1})

pub fn get_account_interactively() -> Result<String, InfoError> {
    print!("Input your username: ");
    io::stdout().flush()?;
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer.trim().to_string())
}

pub fn get_password_interactively() -> Result<String, InfoError> {
    let password = rpassword::prompt_password("Input your password: ")?;
    Ok(password)
}

pub fn login(account: String, password: String, client: Client) -> Result<(), LoginError> {
    let pattern = Regex::new(r"\{.*}").unwrap();

    let wlan_res = client
        .get("https://w.seu.edu.cn/drcom/chkstatus?callback=dr1002")
        .send()?;

    let wlan_code = wlan_res.status();
    if wlan_code.is_success() {
        return Err(LoginError::CodeError(
            wlan_code.as_u16(),
            wlan_code.canonical_reason().unwrap_or_default().to_string(),
        ));
    };

    let wlan_status = match pattern.find(wlan_res.text()?.as_str()) {
        Some(matched) => serde_json::from_str::<WlanStatus>(matched.as_str())?,
        None => return Err(LoginError::ParseError),
    };

    match wlan_status.result {
        1 => return Err(LoginError::AlreadyLoginError),
        0 => (),
        _ => return Err(LoginError::UnknownError),
    };

    let login_url = format!(
        "https://w.seu.edu.cn:801/eportal/?c=Portal&a=login&callback=dr1003\
        &login_method=1\
        &user_account=%2C0%2C{}\
        &user_password={}\
        &wlan_user_ip={}",
        account, password, wlan_status.v46ip
    );

    let login_res = client.get(login_url).send()?;

    let login_code = login_res.status();
    if login_code.is_success() {
        return Err(LoginError::CodeError(
            login_code.as_u16(),
            login_code
                .canonical_reason()
                .unwrap_or_default()
                .to_string(),
        ));
    };

    let login_status = match pattern.find(login_res.text()?.as_str()) {
        Some(matched) => serde_json::from_str::<LoginStatus>(matched.as_str())?,
        None => return Err(LoginError::ParseError),
    };

    if login_status.result != "1" {
        let message = String::from_utf8(BASE64_STANDARD.decode(login_status.msg)?)?;

        return match message.as_str() {
            "ldap auth error" => Err(LoginError::UsernameOrPasswordError),
            "userid error1" => Err(LoginError::UsernameDoesNotExistError),
            "userid error2" => Err(LoginError::PasswordError),
            _ => Err(LoginError::UnknownError),
        };
    }
    Ok(())
}
