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
pub struct WlanStatus {
    /// 认证结果: 0 为未登录，1 为已登录
    pub result: i32,
    /// 当前内网分配的 IPv4 地址
    pub v46ip: String,

    /// 已上网时长（s）
    pub time: Option<u64>,
    /// 本次已使用流量 (KB)
    pub flow: Option<u64>,
    /// 网费余额 (单位是0.0001元)
    pub fee: Option<u32>,

    /// 一卡通号
    pub uid: Option<String>,
    /// 姓名
    #[serde(rename = "NID")]
    pub nid: Option<String>,

    /// 上线时间
    pub stime: Option<String>,
    /// 状态更新时间
    pub etime: Option<String>,

    /// 限制最大时长
    pub oltime: Option<u64>,
    /// 限制最大流量
    pub olflow: Option<u64>,

    /// 其他字段
    #[serde(flatten)]
    pub _extra_field: HashMap<String, Value>,
}

#[derive(Deserialize, Debug)]
pub struct LoginStatus {
    result: String,
    msg: String,
    #[serde(rename = "ret_code")]
    _ret_code: Option<i32>
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

pub fn get_wlan_status(client: &Client, debug: bool) -> Result<WlanStatus, LoginError>{
    let pattern = Regex::new(r"\{.*}").unwrap();

    let wlan_res = client
        .get("https://w.seu.edu.cn/drcom/chkstatus?callback=dr1002")
        .send()?;

    let wlan_code = wlan_res.status();
    if !wlan_code.is_success() {
        return Err(LoginError::CodeError(
            wlan_code.as_u16(),
            wlan_code.canonical_reason().unwrap_or_default().to_string(),
        ));
    };
    let text = wlan_res.text()?;

    if debug {
        eprintln!("[DEBUG] raw status response: {text}");
    }

    match pattern.find(text.as_str()) {
        Some(matched) => {
            let status = serde_json::from_str::<WlanStatus>(matched.as_str())?;
            if debug {
                eprintln!("[DEBUG] parsed wlan status: {status:#?}");
            }
            Ok(status)
        }
        None => Err(LoginError::ParseError),
    }
}

pub fn login_query(
    account: &str,
    password: &str,
    v46ip: &str,
    client: &Client,
    debug: bool,
) -> Result<LoginStatus, LoginError> {
    let pattern = Regex::new(r"\{.*}").unwrap();
    let query_params = [
        ("c", "Portal"),
        ("a", "login"),
        ("callback", "dr1003"),
        ("login_method", "1"),
        ("user_account", &format!(",0,{}", account)),
        ("user_password", password),
        ("wlan_user_ip", v46ip),
    ];

    if debug {
        eprintln!("[DEBUG] login request to https://w.seu.edu.cn:801/eportal/ with query params:");
        for (key, value) in query_params {
            eprintln!("  {key} = {value}");
        }
    }

    let login_res = client
        .get("https://w.seu.edu.cn:801/eportal/")
        .query(&query_params)
        .send()?;

    let login_code = login_res.status();
    if !login_code.is_success() {
        return Err(LoginError::CodeError(
            login_code.as_u16(),
            login_code
                .canonical_reason()
                .unwrap_or_default()
                .to_string(),
        ));
    };
    let text = login_res.text()?;

    if debug {
        eprintln!("[DEBUG] raw login response: {text}");
    }

    match pattern.find(&text) {
        Some(matched) => {
            let status = serde_json::from_str::<LoginStatus>(matched.as_str())?;
            if debug {
                eprintln!("[DEBUG] parsed login status: {status:#?}");
            }
            Ok(status)
        }
        None => Err(LoginError::ParseError),
    }
}

pub fn login(
    account: &str,
    password: &str,
    client: &Client,
    debug: bool,
) -> Result<WlanStatus, LoginError> {
    let wlan_status = get_wlan_status(client, debug)?;
    match wlan_status.result {
        1 => return Err(LoginError::AlreadyLoginError),
        0 => (),
        _ => return Err(LoginError::UnknownError),
    };

    let login_status = login_query(
        account,
        password,
        &wlan_status.v46ip,
        client,
        debug,
    )?;

    if login_status.result != "1" {
        let message = String::from_utf8(BASE64_STANDARD.decode(login_status.msg)?)?;

        if debug {
            eprintln!("[DEBUG] decoded error message: {message}");
        }

        return match message.as_str().trim() {
            "ldap auth error" => Err(LoginError::UsernameOrPasswordError),
            "userid error1" => Err(LoginError::UsernameDoesNotExistError),
            "userid error2" => Err(LoginError::PasswordError),
            _ => Err(LoginError::UnknownError),
        };
    }

    let final_status = get_wlan_status(client, debug)?;
    if debug {
        eprintln!("[DEBUG] login successful, final status: {final_status:#?}");
    }
    Ok(final_status)
}
