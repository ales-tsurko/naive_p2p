use std::net::SocketAddr;

use anyhow::Result;
use clap::{App, Arg};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

use naive_p2p::Peer;

#[tokio::main]
async fn main() -> Result<()> {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    let name = std::env::var("CARGO_PKG_NAME").unwrap();
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();

    let matches = App::new(name)
        .version(version.as_str())
        .author("Ales Tsurko <ales.tsurko@gmail.com>")
        .about("Naive implementation of a peer-to-peer network.")
        .arg(
            Arg::with_name("message")
                .required(true)
                .short("m")
                .long("message")
                .value_name("STRING")
                .help("Sets the message, which will be send by this peer")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("period")
                .required(true)
                .short("t")
                .long("period")
                .value_name("SECONDS")
                .help("Sets the period between sending the message")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .required(true)
                .short("p")
                .long("port")
                .value_name("NUMBER")
                .help("Sets the port number")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("connect")
                .short("c")
                .long("connect")
                .value_name("ADDRESS")
                .help("Address of another peer to connect")
                .takes_value(true),
        )
        .get_matches();

    let message = matches.value_of("message").unwrap();
    let period: u64 = matches.value_of("period").unwrap().parse()?;
    let port: u16 = matches.value_of("port").unwrap().parse()?;
    let connection: Option<SocketAddr> =
        matches.value_of("connect").map(&str::parse).transpose()?;

    let mut server = Peer::new(port, message.to_string(), period, connection).await?;

    server.listen().await?;

    Ok(())
}
