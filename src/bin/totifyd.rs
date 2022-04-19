use config_file::FromConfigFile;
use dbus::arg::{ArgAll, PropMap, RefArg, Variant};
use dbus::blocking::Connection;
use dbus_crossroads::{Context, Crossroads};
use serde_derive::Deserialize;
use std::time::Duration;
use teloxide::prelude::*;
use tokio;

#[derive(Debug, Deserialize)]
struct Config {
    bot: BotConfigInner,
}

#[derive(Debug, Deserialize)]
struct BotConfigInner {
    token: String,
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

fn run_dbus_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("dbus started");
    let conn = Connection::new_session()?;
    let r = conn.request_name("com.example.dbustest", false, false, true)?;
    println!("{:?}", r);
    let mut cr = Crossroads::new();
    let token = cr.register("com.example.dbustest", |b| {
        b.method(
            "Hello",
            ("name",),
            ("reply",),
            |_, _, (name,): (String,)| match local_simple_notification(
                "rebound",
                &name,
                Duration::from_secs(5),
            ) {
                Ok(reply) => Ok((reply,)),
                Err(_) => Err(dbus::MethodErr::failed("Send to notify")),
            },
        );
    });
    println!("{:?}", token);
    cr.insert("/hello", &[token], ());
    cr.serve(&conn)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_config_file("../../myconfig.toml")?;
    println!("{:?}", config.bot.token);

    //
    // Set dbus listener
    //
    // let dbus_server = tokio::task::spawn_blocking(|| run_dbus_server());
    // run_dbus_server()?;
    //
    // set telegram bot
    //
    let bot = Bot::new(config.bot.token).auto_send();
    teloxide::repl(bot, |message: Message, bot: AutoSend<Bot>| async move {
        let mut dbus_server = tokio::task::spawn_blocking(|| Ok(()));
        let mut sever_runnig = false;
        if let teloxide::types::MessageKind::Common(common_message) = message.kind {
            if let teloxide::types::MediaKind::Text(media_text) = common_message.media_kind {
                println!("{:#?}", media_text.text);
                match media_text.text.as_str() {
                    "/start" => {
                        println!("server_running={} starting...", sever_runnig);
                        if !sever_runnig {
                            bot.send_message(message.chat.id, "Start notifications.")
                                .await?;
                            dbus_server = tokio::task::spawn_blocking(|| run_dbus_server());
                            sever_runnig = true;
                            println!("startingnotify");
                        }
                    }
                    "/stop" => {
                        println!("server_running={} stoping...", sever_runnig);
                        if sever_runnig {
                            bot.send_message(message.chat.id, "Stop notifications.")
                                .await?;
                            dbus_server.abort();
                            sever_runnig = false;
                            println!("stopingnotify");
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
        // bot.send_dice(message.chat.id).await?;
        respond(())
    })
    .await;
    println!("TELEGRAM STARTED");
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen to ctrl_c");
    Ok(())
}
