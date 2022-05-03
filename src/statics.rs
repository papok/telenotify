use lazy_static::lazy_static;
use serde_derive::Deserialize;
use config_file::FromConfigFile;



pub static TOTIFY_BOT_DATA_FILE: &'static str = "../../totify_bot_data.toml";

pub static TELEDATA_FILE: &'static str = "../../myconfig.json";

lazy_static! {
    pub static ref BOT_TOKEN: String = load_bot_data(&TOTIFY_BOT_DATA_FILE)
        .unwrap_or_else(|_| {
            println!("Failed to load bot token from {}", TOTIFY_BOT_DATA_FILE);
            std::process::exit(1)
        })
        .token;
}

#[derive(Debug, Deserialize)]
struct BotConfig {
    bot: BotData,
}

#[derive(Debug, Deserialize)]
struct BotData {
    token: String,
}

fn load_bot_data(file_name: &str) -> Result<BotData, Box<dyn std::error::Error>> {
    Ok(BotConfig::from_config_file(file_name)?.bot)
}