use dbus::arg::{ArgAll, PropMap, RefArg, Variant};
use dbus::blocking::Connection;
use dbus_crossroads::{Crossroads, Context};
use std::time::Duration;

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
    //let (message_nr,): (u32,) =
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::new_session()?;

    let notifications_proxy = conn.with_proxy(
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        Duration::from_secs(5),
    );

    let notifications_properties = PropMap::from([(
        String::from("urgency"),
        Variant::<Box<dyn RefArg>>(Box::new(1u32)),
    )]);
    let (message_nr,): (u32,) = notifications_proxy.method_call(
        "org.freedesktop.Notifications",
        "Notify",
        (
            "",
            0u32,
            "",
            "Hello world!",
            "This is an example notification.",
            vec![""],
            notifications_properties,
            5000i32,
        ),
    )?;
    println!("{}", message_nr);

    local_simple_notification("Hola", "Este es un mensaje de ejemplo", Duration::from_secs(5))?;
    let r = conn.request_name("com.example.dbustest", false, false, true)?;
    println!("{:?}", r);

    let test_proxy = conn.with_proxy(
        "com.example.dbustest",
        "/com/example/dbustest",
        Duration::from_secs(5),
    );

    let mut cr = Crossroads::new();
    let token = cr.register("com.example.dbustest", |b| {
        b.method("Hello", ("name",), ("reply",), |_, _, (name,): (String,)| {
            // Ok((format!("Hello {}!", name),))
            local_simple_notification("rebound", &name , Duration::from_secs(5));
            Ok((format!("Hello {}!", name),))
        });
    });
    cr.insert("/hello", &[token], ());
    // local_simple_notification("test", &format!("{:?}",token) , Duration::from_secs(5))?;
    cr.serve(&conn)?;
    Ok(())
}
