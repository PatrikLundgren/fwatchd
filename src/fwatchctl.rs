mod socket;
use anyhow::{anyhow, Context, Result};
use clap::{Arg, ArgMatches, Command};
use socket::*;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;

fn build() -> clap::Command<'static> {
    let mut app = Command::new("fwatchd - A file watching daemon")
        .version("2021")
        .author("Patrik Lundgren <patrik.lundgren.95@gmail.com>")
        .about("fwatch utilizes the inotity API to watch files \n");

    app = app.subcommand(
        Command::new("track")
            .about("track file, take snapshot when changed.")
            .arg(Arg::new("file").required(true).takes_value(true))
            .arg(
                Arg::new("script")
                    .long("script")
                    .value_name("SCRIPT")
                    .takes_value(true)
                    .help("command which is executed on events"),
            )
            .arg(
                Arg::new("alias")
                    .long("alias")
                    .value_name("ALIAS")
                    .takes_value(true)
                    .help("Script called to determine an alias for a file given an event"),
            ),
    );

    app = app.subcommand(
        Command::new("list")
            .about("list available file snapshots")
            .arg(Arg::new("file").takes_value(true)),
    );

    app = app.subcommand(
        Command::new("select")
            .about("list available file snapshots")
            .arg(Arg::new("file").takes_value(true))
            .arg(Arg::new("hash").takes_value(true)),
    );

    app = app.subcommand(
        Command::new("echo")
            .about("Test unix socket for daemon")
            .arg(Arg::new("message").takes_value(true)),
    );

    app = app.subcommand(
        Command::new("echoerr")
            .about("Test unix socket for daemon")
            .arg(Arg::new("message").takes_value(true)),
    );

    app
}

fn track(args: &ArgMatches) -> Result<()> {
    let fpath: String = args.value_of_t("file").context("No pattern provided")?;
    let action = match args.value_of_t::<String>("script") {
        Ok(path) => Action::Script(path),
        _ => Action::Save,
    };
    let alias: Alias = match args.value_of_t::<String>("alias") {
        Ok(spath) => Alias::Script(spath),
        _ => Alias::Basename,
    };
    let track = Track {
        fpath,
        alias,
        action,
    };
    let payload = bincode::serialize(&track).context("Failed to serialize payload")?;
    let mut stream = UnixStream::connect(SOCK_PATH)?;
    let mut response = String::new();

    let pkt = Packet {
        command: socket::Command::Track,
        payload,
    };
    stream.write_all(&bincode::serialize(&pkt)?)?;
    stream.read_to_string(&mut response)?;
    println!("{}", response);
    Ok(())
}

fn list(args: &ArgMatches) -> Result<()> {
    let payload: String = args
        .value_of_t("file")
        .ok()
        .or_else(|| Some("*".to_string()))
        .context("No pattern provided")?;

    let mut stream = UnixStream::connect(SOCK_PATH)?;
    let mut response = String::new();
    let payload = bincode::serialize(&payload).context("Failed to serialize payload")?;
    let pkt = Packet {
        command: socket::Command::List,
        payload,
    };

    stream.write_all(&bincode::serialize(&pkt)?)?;
    stream.read_to_string(&mut response)?;
    println!("{}", response);
    Ok(())
}

fn select(args: &ArgMatches) -> Result<()> {
    let sel: (String, String) = (
        args.value_of_t("file").context("No pattern provided")?,
        args.value_of_t("hash").context("No pattern provided")?,
    );
    let mut stream = UnixStream::connect(SOCK_PATH).context("Failed to open socket")?;
    let mut response = String::new();
    let payload = bincode::serialize(&sel).context("Failed to serialize payload")?;
    let pkt = Packet {
        command: socket::Command::Select,
        payload,
    };

    stream
        .write_all(&bincode::serialize(&pkt)?)
        .context("Failed to write to socket")?;
    stream
        .read_to_string(&mut response)
        .context("Failed to read from socket")?;

    println!("{}", response);
    Ok(())
}

fn echo(args: &ArgMatches, is_err: bool) -> Result<()> {
    let msg: String = args
        .value_of_t("message")
        .ok()
        .context("No message provided")?;
    let mut stream = UnixStream::connect(SOCK_PATH)?;
    let mut response = String::new();
    let payload = bincode::serialize(&msg).context("Failed to serialize payload")?;
    let pkt = Packet {
        command: if is_err {
            socket::Command::Echoerr
        } else {
            socket::Command::Echo
        },
        payload,
    };

    stream.write_all(&bincode::serialize(&pkt)?)?;
    stream.read_to_string(&mut response)?;
    println!("{}", response);
    Ok(())
}

fn dispatch(app: &mut Command, matches: &ArgMatches) {
    if let Err(msg) = match matches.subcommand() {
        Some(("track", sargs)) => track(sargs),
        Some(("list", sargs)) => list(sargs),
        Some(("select", sargs)) => select(sargs),
        Some(("echo", sargs)) => echo(sargs, false),
        Some(("echoerr", sargs)) => echo(sargs, true),
        None => {
            println!("{}", app.render_usage());
            Ok(())
        }
        _ => Err(anyhow!("Unrecognized command")),
    } {
        println!("{}", msg);
    }
}

fn main() {
    let mut app = build();
    let matches = app.clone().try_get_matches();
    match matches {
        Ok(m) => dispatch(&mut app, &m),
        Err(msg) => println!("{}", msg),
    };
}
