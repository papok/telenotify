use crate::statics::{TELEDATA_FILE, BOT_TOKEN};
use crate::types::UserName;
use anyhow::{Context, Result};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;

#[derive(Debug, Serialize, Deserialize)]
pub struct TeleData {
    bot_token: String,
    pub users_data: HashMap<UserName, UserData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    pub chat_id: teloxide::types::ChatId,
    pub active: bool,
}

pub fn make_on_fail_teledata_file() -> Result<(), Box<dyn std::error::Error>> {
    match load_teledata() {
        Err(err) => {
            println!("Making teledata file because of: {}", err);
            let teledata = TeleData {
                bot_token: BOT_TOKEN.clone(),
                users_data: HashMap::new(),
            };
            save_teledata(&teledata)?;
            Ok(())
        }
        Ok(_) => Ok(()),
    }
}

pub fn load_teledata() -> Result<TeleData> {
    Ok(serde_json::from_reader(
        File::open(TELEDATA_FILE).context(format!("Failed to open '{}' file.", TELEDATA_FILE))?,
    )
    .context("Failed to deserialize data.")?)
}

pub fn save_teledata(teledata: &TeleData) -> Result<()> {
    serde_json::to_writer(
        File::create(TELEDATA_FILE).context(format!("Failed to write '{}' file.", TELEDATA_FILE))?,
        teledata,
    )
    .context("Failed to serialize data.")?;
    Ok(())
}
