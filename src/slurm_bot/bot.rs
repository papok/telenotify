use crate::slurm_bot::storage::*;
use crate::statics::BOT_TOKEN;
use crate::types::UserName;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use teloxide::{prelude::*, utils::command::BotCommands};

pub async fn start_slurm_bot() {
    let bot = Bot::new(format!("{}", *BOT_TOKEN)).auto_send();
    teloxide::commands_repl(bot, command_handler_error_detector, Command::ty()).await;
}

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Display this text.")]
    Help,
    #[command(
        description = "Register with username and password into system to recive updates on your jobs."
    )]
    Register(String),
    #[command(description = "Unregister with username from system.")]
    Unregister(String),
    #[command(description = "Pause notifications.")]
    Pause,
    #[command(description = "Unpause notifications.")]
    Unpause,
    #[command(description = "Kill the bot server.")]
    Kill,
}

async fn command_handler_error_detector(
    bot: AutoSend<Bot>,
    message: Message,
    command: Command,
) -> Result<()> {
    if let Err(e) = command_handler(bot, message, command).await {
        println!("Error in command handler: {:?}", e);
        std::process::exit(1)
    }
    Ok(())
}

async fn command_handler(bot: AutoSend<Bot>, message: Message, command: Command) -> Result<()> {
    let mut teledata: TeleData = load_teledata()?;
    let ref mut users = teledata.users_data;
    let chat_map: HashMap<teloxide::types::ChatId, UserName> =
        users.iter().map(|(k, v)| (v.chat_id, k.clone())).collect();
    match command {
        Command::Help => {
            bot.send_message(message.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Register(text) => {
            let words: Vec<_> = text.split_whitespace().collect();
            let user = words.get(0).unwrap_or(&"").to_string();
            let pass = words.get(1).unwrap_or(&"").to_string();
            if chat_map.contains_key(&message.chat.id) {
                bot.send_message(
                    message.chat.id,
                    "An user is already registered in this chat.",
                )
                .await?;
            } else if user == "" {
                bot.send_message(message.chat.id, "Nobody to register.")
                    .await?;
            } else if !is_user_pass_ok(&user, &pass)? {
                bot.send_message(message.chat.id, "Wrong password or username.")
                    .await?;
                // chequear si se puede acceder desde otro telefono si pongo un sleep para evitar ataques
            } else {
                let new_user = UserData {
                    chat_id: message.chat.id,
                    active: true,
                };
                users.insert(user.clone(), new_user);
                bot.send_message(message.chat.id, format!("Welcome {}!", user))
                    .await?;
            }
        }
        Command::Unregister(text) => {
            let words: Vec<_> = text.split_whitespace().collect();
            if chat_map.contains_key(&message.chat.id) {
                let user = words.get(0).unwrap_or(&"").to_string();
                if user == "" || &user != chat_map.get(&message.chat.id).unwrap() {
                    bot.send_message(message.chat.id, "You must specify your username.")
                        .await?;
                } else {
                    users.remove(&user);
                    bot.send_message(message.chat.id, format!("You are no longer registered"))
                        .await?;
                }
            } else {
                bot.send_message(message.chat.id, "You are not registered. ")
                    .await?;
            }
        }
        Command::Pause => {
            if chat_map.contains_key(&message.chat.id) {
                if users[&chat_map[&message.chat.id]].active {
                    bot.send_message(message.chat.id, "You will stop to recive notifications.")
                        .await?;
                    users.get_mut(&chat_map[&message.chat.id]).unwrap().active = false;
                }
            } else {
                bot.send_message(message.chat.id, "You are not registered. ")
                    .await?;
            }
        }
        Command::Unpause => {
            if chat_map.contains_key(&message.chat.id) {
                if !users[&chat_map[&message.chat.id]].active {
                    bot.send_message(message.chat.id, "You will start to recive notifications.")
                        .await?;
                    users.get_mut(&chat_map[&message.chat.id]).unwrap().active = true;
                }
            } else {
                bot.send_message(message.chat.id, "You are not registered. ")
                    .await?;
            }
        }
        Command::Kill => {
            bot.send_message(message.chat.id, "Killing bot.").await?;
            std::process::exit(0);
        }
    };
    save_teledata(&teledata).context("Could not save ")?;
    Ok(())
}

fn is_user_pass_ok(user: &str, pass: &str) -> Result<bool> {
    let shadow_file_path = "/etc/shadow";
    let file = File::open(shadow_file_path).context("Error opening passwords file.")?;
    let reader = BufReader::new(file);
    let lines = reader.lines();
    for line in lines {
        let line_string = line.context("Error parsing passwords file.")?;
        if line_string.starts_with(&format!("{}:", user)) {
            let encripted = line_string
                .split(":")
                .nth(1)
                .context("Error geting salt in passwords file.")?
                .trim();
            let algorithm = encripted
                .split("$")
                .nth(1)
                .context("Error geting salt in passwords file.")?;
            let salt = encripted
                .split("$")
                .nth(2)
                .context("Error geting salt in passwords file.")?;

            let encripted_test = std::process::Command::new("openssl")
                .args([
                    "passwd".to_string(),
                    String::from(format!("-{}", algorithm)),
                    "-salt".to_string(),
                    salt.to_string(),
                    pass.to_string(),
                ])
                .output()
                .context("Error running openssl.")?
                .stdout;
            let encripted_test = std::str::from_utf8(&encripted_test)?.trim();
            return Ok(encripted == encripted_test);
        }
    }
    Ok(false)
}
