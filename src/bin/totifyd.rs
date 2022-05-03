use anyhow::{Context, Result};
use dbus::arg::{ArgAll, PropMap, RefArg, Variant};
use dbus::blocking::Connection;
use dbus_crossroads::Crossroads;
use std::time::Duration;
use telenotify::statics::BOT_TOKEN;





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

fn run_dbus_server(tele_bot_token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await?;
    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    telenotify::slurm_bot::storage::make_on_fail_teledata_file()?;
    println!("BOT_TOKEN = {}", *BOT_TOKEN);
    //
    // Set dbus listener
    //
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
    telenotify::slurm_bot::bot::start_slurm_bot().await;
    println!("TELEGRAM STARTED");
    Ok(())
}
