use std::process;
use std::env;
use std::str::FromStr;
use tracing::Level;
use clap::{App, AppSettings, Arg};
use spacechat_agent::server;

/// The `tokio::main` attribute sets up a tokio runtime.
#[actix_web::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_max_level(
            Level::from_str(&env::var("RUST_LOG").unwrap_or_else(|_| String::from("error")))
                .unwrap_or(Level::ERROR),
        )
        .init();

    let matches = App::new("spacechat_agent")
        .version("0.1.0")
        .author("kyfk <fukushi098@gmail.com>")
        .about("The SpaceChat Agent to interact with other agents.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            App::new("run")
                .long_flag("run")
                .about("Run the SpaceChat Agent")
                .arg(
                    Arg::new("host")
                        .help("the host listen to")
                        .long("host")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::new("port")
                        .help("the port listen to")
                        .short('p')
                        .long("port")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::new("listen_multiaddr")
                        .help("the address to expose and accept messages.")
                        .short('m')
                        .long("listen-multiaddr")
                        .takes_value(true)
                        .required(true)
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("run", matches)) => {
            let host = matches.value_of("host").unwrap();
            let port = matches.value_of("port").unwrap();
            let listen_multiaddr = matches.value_of("listen_multiaddr").unwrap();

            server::run(host, port, listen_multiaddr)
                .await
                .map_err(|e| handle_err(Box::new(e)));
        }
        _ => unreachable!(),
    };

    process::exit(0)
}

fn handle_err(e: Box<(dyn std::error::Error + 'static)>) -> ! {
    eprintln!("command returned an error: {}", e);
    process::exit(1)
}
