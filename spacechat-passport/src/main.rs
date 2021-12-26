use clap::{App, AppSettings, Arg};
use spacechat_passport::{command, PrivateKey, PublicKey};
use std::env;
use std::io::Read;
use std::process;
use std::str::FromStr;
use std::{fs::File, io::BufReader};
use tracing::Level;

#[actix_web::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_max_level(
            Level::from_str(&env::var("RUST_LOG").unwrap_or_else(|_| String::from("error")))
                .unwrap_or(Level::ERROR),
        )
        .init();

    let matches = App::new("spacechat_passport")
        .version("0.1.0")
        .author("kyfk <fukushi098@gmail.com>")
        .about("The Id Allocator of SpaceChat")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            App::new("run_server")
                .long_flag("run-server")
                .about("Run the passport server")
                .arg(
                    Arg::new("host")
                        .help("a valid string consisting of a hostname or IP followed by an optional port number")
                        .long("host")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::new("public_key")
                        .help("a path that stores a public key")
                        .long("pub-key")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::new("private_key")
                        .help("a path that stores a private key")
                        .long("priv-key")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::new("passphrase")
                        .help("a key used to decrypt the file that contains the RSA key")
                        .short('p')
                        .long("passphrase")
                        .takes_value(true)
                        .required(true)
                )
        )
        .subcommand(
            App::new("key_gen")
                .long_flag("key-gen")
                .about("Generate a private key and a public key of RSA key")
                .arg(
                    Arg::new("dist")
                        .help("the diatination to store keys")
                        .short('d')
                        .long("dist")
                        .takes_value(true)
                        .required(true)
                )
                .arg(
                    Arg::new("passphrase")
                        .help("a key used to encrypt the file that contains the RSA key")
                        .short('p')
                        .long("passphrase")
                        .takes_value(true)
                        .required(true)
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("run-server", matches)) => {
            let host = matches.value_of("host").unwrap();
            let public_key_path = matches.value_of("public_key").unwrap();
            let private_key_path = matches.value_of("private_key").unwrap();
            let passphrase = matches.value_of("passphrase").unwrap();

            let public_key: PublicKey = read_as_bytes(public_key_path).map_err(handle_err).unwrap();
            let private_key: PrivateKey =
                read_as_bytes(private_key_path).map_err(handle_err).unwrap();

            command::run_server(
                host,
                public_key,
                private_key,
                passphrase.as_bytes().to_vec(),
            )
            .await
            .map_err(|e| handle_err(Box::new(e)));
        }
        Some(("key-gen", matches)) => {
            let dist = matches.value_of("dist").unwrap();
            let passphrase = matches.value_of("passphrase").unwrap();

            command::key_gen(dist, passphrase).map_err(handle_err);
        }
        _ => unreachable!(),
    };
    process::exit(0);
}

fn handle_err(e: Box<(dyn std::error::Error + 'static)>) -> ! {
    eprintln!("command returned an error: {}", e);
    process::exit(1)
}

fn read_as_bytes(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut bytes = Vec::new();
    buf_reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}
