use serde::Serialize;
use serde::Deserialize;
use std::io::prelude::*;

#[derive(Deserialize)]
pub struct Config {
    pub telegram_bot_token: String,
    pub deepseek_api_token: String,
    pub superuser_uid: String,
}

#[derive(Serialize, Deserialize)]
pub struct TrustedUsers {
    pub trusted_users: Vec<String>,
}

pub fn get_config() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    let config = toml::from_str::<Config>(std::fs::read_to_string("config.toml")?.as_str())?;
    Ok(config)
}

pub fn get_trusted_users() -> Result<TrustedUsers, Box<dyn std::error::Error + Send + Sync>> {
    let trusted_users = toml::from_str::<TrustedUsers>(std::fs::read_to_string("trustedusers.toml")?.as_str())?;
    Ok(trusted_users)
}

pub fn set_trusted_users(trusted_users: TrustedUsers) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut file = std::fs::File::create("trustedusers.toml")?;
    file.write_all(toml::to_string(&trusted_users)?.as_bytes())?;
    Ok(())
}

pub fn add_trusted_user(uid: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut trusted_users = get_trusted_users()?;
    if trusted_users.trusted_users.iter().find(|x| **x == uid) == None {
        trusted_users.trusted_users.push(uid);
    }
    set_trusted_users(trusted_users)?;
    Ok(())
}

pub fn del_trusted_user(uid: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut trusted_users = get_trusted_users()?;
    if let Some(idx) = trusted_users.trusted_users.iter().position(|x| **x == uid) {
        trusted_users.trusted_users.remove(idx);
    }
    set_trusted_users(trusted_users)?;
    Ok(())
}
