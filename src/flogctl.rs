use anyhow::{anyhow, Context, Result};
use clap::{Arg, ArgMatches, Command};
use flib::*;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;

pub fn build() -> clap::Command<'static> {
    let mut app = Command::new("flog - the forgetful file log.")
        .version("2021")
        .author("Patrik Lundgren <patrik.lundgren.95@gmail.com>")
        .about("flog has a short but excellent memory, it remembers file(s) by name and \n");

    app = app.subcommand(
        Command::new("track")
            .about("track file, take snapshot when changed.")
            .arg(Arg::new("file").required(true).takes_value(true)),
    );

    app = app.subcommand(
        Command::new("list")
            .about("list available file snapshots")
            .arg(Arg::new("file").takes_value(true)),
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
    let payload: String = args.value_of_t("file").context("No pattern provided")?;
    let payload = bincode::serialize(&payload).context("Failed to serialize payload")?;
    let mut stream = UnixStream::connect(SOCK_PATH)?;
    let mut response = String::new();

    let pkt = Packet {
        command: flib::Command::TRACK,
        payload,
    };
    stream.write_all(&bincode::serialize(&pkt)?)?;
    stream.read_to_string(&mut response)?;
    println!("{}", response);
    Ok(())
}

fn list(args: &ArgMatches) -> Result<()> {
    let payload: String = args
        .value_of_t("pattern")
        .ok()
        .or_else(|| Some("*".to_string()))
        .context("No pattern provided")?;

    let mut stream = UnixStream::connect(SOCK_PATH)?;
    let mut response = String::new();
    let payload = bincode::serialize(&payload).context("Failed to serialize payload")?;
    let pkt = Packet {
        command: flib::Command::LIST,
        payload,
    };

    stream.write_all(&bincode::serialize(&pkt)?)?;
    stream.read_to_string(&mut response)?;
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
            flib::Command::ECHOERR
        } else {
            flib::Command::ECHO
        },
        payload,
    };

    stream.write_all(&bincode::serialize(&pkt)?)?;
    stream.read_to_string(&mut response)?;
    println!("{}", response);
    Ok(())
}

fn dispatch(app: &mut Command, matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("track", sargs)) => track(sargs),
        Some(("list", sargs)) => list(sargs),
        Some(("echo", sargs)) => echo(sargs, false),
        Some(("echoerr", sargs)) => echo(sargs, true),
        None => {
            println!("{}", app.render_usage());
            Ok(())
        }
        _ => Err(anyhow!("Unrecognized command")),
    }
    .unwrap();
}

fn main() {
    let mut app = build();
    let matches = app.clone().try_get_matches();
    match matches {
        Ok(m) => dispatch(&mut app, &m),
        Err(msg) => println!("{}", msg),
    };
}
