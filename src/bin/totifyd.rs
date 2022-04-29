use anyhow::{Context, Result};
use config_file::FromConfigFile;
use dbus::arg::{ArgAll, PropMap, RefArg, Variant};
use dbus::blocking::Connection;
use dbus_crossroads::Crossroads;
use lazy_static::lazy_static;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use teloxide::prelude::*;
use tokio;

static TOTIFY_BOT_DATA_FILE: &'static str = "../../totify_bot_data.toml";

static TELEDATA_FILE: &'static str = "../../myconfig.toml";

// lazy_static! {
//     static ref BOT_TOKEN: &'static str = load_bot_data(&TOTIFY_BOT_DATA_FILE).unwrap_or_else(|_| {
//                         println!("Failed to load bot token from {}", TOTIFY_BOT_DATA_FILE);
//                         std::process::exit(1)
//                     }).token;
// }

lazy_static! {
    static ref BOT_TOKEN: String = load_bot_data(&TOTIFY_BOT_DATA_FILE)
        .unwrap_or_else(|_| {
            println!("Failed to load bot token from {}", TOTIFY_BOT_DATA_FILE);
            std::process::exit(1)
        })
        .token;
}

// lazy_static! {
//     static ref BOT_TOKEN: String = String::from(
//         load_bot_data(&TOTIFY_BOT_DATA_FILE)
//             .unwrap_or_else(|_| {
//                 println!("Failed to load bot token from {}", TOTIFY_BOT_DATA_FILE);
//                 std::process::exit(1)
//             })
//             .token
//     );
// }

type UserName = String;

#[derive(Debug, Serialize, Deserialize)]
struct TeleData {
    bot_token: String,
    users_data: HashMap<UserName, UserData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserData {
    chat_id: teloxide::types::ChatId,
    paused: bool,
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

fn make_on_fail_teledata_file(file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    match load_teledata(file_name) {
        Err(err) => {
            println!("Making teledata file because of: {}", err);
            let teledata = TeleData {
                bot_token: BOT_TOKEN.clone(),
                users_data: HashMap::new(),
            };
            save_teledata(file_name, &teledata)?;
            Ok(())
        }
        Ok(_) => Ok(()),
    }
}

fn load_teledata(file_name: &str) -> Result<TeleData, Box<dyn std::error::Error>> {
    Ok(serde_json::from_reader(File::open(file_name)?)?)
}

fn save_teledata(file_name: &str, teledata: &TeleData) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer(File::create(file_name)?, teledata)?;
    Ok(())
}

fn local_simple_notification(
    title: &str,
    content: &str,
    duration: Duration,
) -> Result<u32, Box<dyn std::error::Error>> {
    let conn = Connection::new_session()?;
    let notifications_proxy = conn.with_proxy(
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        Duration::from_secs(5),
    );
    let (message_nr,): (u32,) = notifications_proxy.method_call(
        "org.freedesktop.Notifications",
        "Notify",
        (
            "",
            0u32,
            "",
            title,
            content,
            vec![""],
            PropMap::from([(
                String::from("urgency"),
                Variant::<Box<dyn RefArg>>(Box::new(1u32)),
            )]),
            duration.as_millis() as i32,
        ),
    )?;
    Ok(message_nr)
}

// def notify(chat_id)
fn run_dbus_server(tele_bot_token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // fn run_dbus_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("dbus started");
    println!("token {}", tele_bot_token);
    let conn = Connection::new_session()?;
    let r = conn.request_name("ar.uba.fcen", false, false, true)?;
    if r != dbus::blocking::stdintf::org_freedesktop_dbus::RequestNameReply::PrimaryOwner {
        return Err("Not PimaryOwner when requesting dbus name.".into());
    }
    let mut cr = Crossroads::new();
    let dbus_iface_token = cr.register("ar.uba.fcen", |b| {
        b.method(
            "TeleNotify",
            ("user", "message"),
            ("reply",),
            |c1, c2, (user, message): (String, String)| {
                println!("{:?}", c1);
                println!("{:?}", c2);
                match local_simple_notification(&user, &message, Duration::from_secs(5)) {
                    Ok(reply) => Ok((reply,)),
                    Err(_) => Err(dbus::MethodErr::failed("Send to notify")),
                }
            },
        );
    });
    println!("{:?}", dbus_iface_token);
    cr.insert("/ar/uba/fcen", &[dbus_iface_token], ());
    cr.serve(&conn)?;
    Ok(())
}

fn check_user_pass(user: &str, pass: &str) -> Result<bool> {
    let shadow_file_path = "/etc/shadow";
    let file = File::open(shadow_file_path).context("Error opening passwords file.")?;
    let reader = BufReader::new(file);
    let lines = reader.lines();
    for line in lines {
        let line_string = line.context("Error parsing passwords file.")?;
        if line_string.starts_with(&format!("{}:", user)) {
            println!("{}", line_string);
            let encripted = line_string
                .split(":")
                .nth(1)
                .context("Error geting salt in passwords file.")?;
            let algorithm = encripted
                .split("$")
                .nth(1)
                .context("Error geting salt in passwords file.")?;
            let salt = encripted
                .split("$")
                .nth(2)
                .context("Error geting salt in passwords file.")?;
            let data = encripted
                .split("$")
                .nth(3)
                .context("Error geting salt in passwords file.")?;
            println!("e---- {}", encripted);
            println!("a---- {}", algorithm);
            println!("s---- {}", salt);
            println!("d---- {}", data);
            let encripted_test = std::process::Command::new("openssl").args([
                "passwd".to_string(),
                String::from(format!("-{}", algorithm)),
                "-salt",
                salt,
                pass,
            ]).output().context("Error running openssl.")?;
            println!("t---- {:?}", encripted_test);
            return Ok(true);
        }
    }
    Ok(false)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    make_on_fail_teledata_file(TELEDATA_FILE)?;
    // let users: Arc<RwLock<HashMap<String, i64>>> = Arc::new(RwLock::new({
    //     let mut m = HashMap::new();
    //     m
    // }));
    // let users = Box::new(HashMap::new());
    // let users: Arc<RwLock<HashMap<String, teloxide::types::ChatId>>> =
    //     Arc::new(RwLock::new(HashMap::new()));
    // let users_r = Arc::clone(&users);
    println!("BOT_TOKEN = {}", *BOT_TOKEN);
    //
    // Set dbus listener
    //
    // let bot_token = config.bot.token.clone();
    println!("A");
    let dbus_server = tokio::task::spawn_blocking(move || {
        println!("B");
        run_dbus_server(&BOT_TOKEN);
        // let server = run_dbus_server();
        println!("C");
    });
    // run_dbus_server();
    //
    // set telegram bot
    //
    println!("D");
    let bot = Bot::new(format!("{}", *BOT_TOKEN)).auto_send();
    let mut sever_runnig = false;
    teloxide::repl(
        bot,
        move |message: Message, bot: AutoSend<Bot>| async move {
            println!("start");
            // println!("{:#?}", message);
            // |message: Message, bot: AutoSend<Bot>| async {
            let mut teledata: TeleData = load_teledata(TELEDATA_FILE).unwrap_or_else(|s| {
                println!("---{}---", s);
                std::process::exit(1)
            });
            let ref mut users = teledata.users_data;
            let chat_map: HashMap<teloxide::types::ChatId, UserName> =
                users.iter().map(|(k, v)| (v.chat_id, k.clone())).collect();
            if let teloxide::types::MessageKind::Common(common_message) = message.kind {
                if let teloxide::types::MediaKind::Text(media_text) = common_message.media_kind {
                    let words: Vec<_> = media_text.text.split_whitespace().collect();
                    let first_word = words.get(0).unwrap_or(&"").to_string();

                    match first_word.as_str() {
                        "/register" => {
                            let user = words.get(1).unwrap_or(&"").to_string();
                            let pass = words.get(2).unwrap_or(&"").to_string();
                            if user == "" {
                                bot.send_message(message.chat.id, "Nobody to register.")
                                    .await?;
                            } else {
                                let new_user = UserData {
                                    chat_id: message.chat.id,
                                    paused: false,
                                };
                                if check_user_pass(&user, &pass).unwrap_or_else(|s| {
                                    println!("Error checking password: {}", s);
                                    std::process::exit(1)
                                }) {
                                    println!("user found!");
                                } else {
                                    println!("user bot found...");
                                }
                                users.insert(user.clone(), new_user);
                                bot.send_message(message.chat.id, format!("Welcome {}!", user))
                                    .await?;
                            }
                        }
                        "/unregister" => {
                            if chat_map.contains_key(&message.chat.id) {
                                let user = words.get(1).unwrap_or(&"").to_string();
                                if user == "" || &user != chat_map.get(&message.chat.id).unwrap() {
                                    bot.send_message(
                                        message.chat.id,
                                        "You must specify your username.",
                                    )
                                    .await?;
                                } else {
                                    users.remove(&user);
                                    bot.send_message(
                                        message.chat.id,
                                        format!("You are no longer registered"),
                                    )
                                    .await?;
                                }
                            } else {
                                bot.send_message(message.chat.id, "You are not registered. ")
                                    .await?;
                            }
                        }
                        "/unpause" => {
                            if chat_map.contains_key(&message.chat.id) {
                                if users[&chat_map[&message.chat.id]].paused {
                                    bot.send_message(
                                        message.chat.id,
                                        "You will start to recive notifications.",
                                    )
                                    .await?;
                                    users.get_mut(&chat_map[&message.chat.id]).unwrap().paused =
                                        false;
                                }
                            } else {
                                bot.send_message(message.chat.id, "You are not registered. ")
                                    .await?;
                            }
                        }
                        "/pause" => {
                            if chat_map.contains_key(&message.chat.id) {
                                if !users[&chat_map[&message.chat.id]].paused {
                                    bot.send_message(
                                        message.chat.id,
                                        "You will stop to recive notifications.",
                                    )
                                    .await?;
                                    users.get_mut(&chat_map[&message.chat.id]).unwrap().paused =
                                        true;
                                }
                            } else {
                                bot.send_message(message.chat.id, "You are not registered. ")
                                    .await?;
                            }
                        }
                        "/kill" => {
                            bot.send_message(message.chat.id, "Killing bot.").await?;
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                    // if media_text.text == "/stop" {
                    //     bot.send_message(message.chat.id, "Stoping.").await?;
                    //     std::process::exit(0);
                    // }
                }
            }
            save_teledata(TELEDATA_FILE, &teledata).unwrap_or_else(|s| {
                println!("Error when saving teledata: {}", s);
                std::process::exit(1)
            });
            // drop(users_w);
            // bot.send_dice(message.chat.id).await?;
            println!("end");
            respond(())
        },
    )
    .await;
    println!("TELEGRAM STARTED");
    // save_teledata(&TELEDATA_FILE)?;
    // tokio::signal::ctrl_c()
    //     .await
    //     .expect("failed to listen to ctrl_c");
    Ok(())
}
