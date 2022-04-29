use dbus::arg::{ArgAll, PropMap, RefArg, Variant};
use dbus::blocking::Connection;
use dbus_crossroads::{Context, Crossroads};
use std::env::args;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let empty_string = String::new();
    let arguments: Vec<_> = args().collect();
    let user = arguments.get(1).unwrap_or(&empty_string);
    let message = arguments.get(2).unwrap_or(&empty_string);

    println!("{} / {}", user, message);
    let conn = Connection::new_session()?;
    let totify_proxy = conn.with_proxy(
        "ar.uba.fcen",
        "/ar/uba/fcen",
        Duration::from_secs(5),
    );

    let (message_nr,): (u32,) = totify_proxy.method_call(
        "ar.uba.fcen",
        "TeleNotify",
        (user, message)
    )?;

    println!("---{}", message_nr);

    Ok(())
}
