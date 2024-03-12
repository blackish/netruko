extern crate netruko;
use clap;
use tokio;
use netruko::Netruko;

#[tokio::main]
async fn main() {
    let args = clap::command!()
        .arg(
            clap::arg!(host: -H --host <HOST> "host to connect")
                .required(true)
        )
        .arg(
            clap::arg!(username: -u --user <USERNAME> "username")
                .required(true)
        )
        .get_matches();
    let host = args.get_one::<String>("host").unwrap();
    let user = args.get_one::<String>("username").unwrap();
    let password = rpassword::prompt_password("Pwd?:").unwrap();
    let mut conn = Netruko::new(host, "linux", user, password.as_str(), Some(&password)).expect("Cannot create conn");
    conn.connect().await.expect("Cannot connect");
    let mut res = conn.command("whoami".to_string(), false).await.expect("Cannot send command");
    conn.do_become().await.expect("Cannot become");
    res.extend_from_slice(&conn.command("whoami".to_string(), false).await.expect("Cannot send command"));
    let _ = conn.command("exit".into(), false).await.expect("Cannot send command");
    res.extend_from_slice(&conn.command("whoami".to_string(), false).await.expect("Cannot send command"));
    let _ = conn.disconnect().await;
    for line in res {
        println!("Hello, world! {} {:?}", host, line);
    }
}
