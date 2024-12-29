use crate::config;
pub enum Role {
    SuperUser,
    User,
    Untrusted,
}

pub fn check_uid(uid: String) -> Result<Role, Box<dyn std::error::Error + Send + Sync>> {
    let config = config::get_config()?;
    if uid == config.superuser_uid {
        return Ok(Role::SuperUser);
    }
    if config::get_trusted_users()?.trusted_users.into_iter().find(|x| *x == uid) != None {
        return Ok(Role::User);
    }
    Ok(Role::Untrusted)
}
