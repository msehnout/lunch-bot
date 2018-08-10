extern crate irc;
extern crate env_logger;
extern crate failure;
#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
extern crate regex;
extern crate tokio_timer;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use failure::Error;
use irc::client::prelude::*;
use irc::error::IrcError;
use regex::{Regex};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

mod syntax;
use syntax::{LunchCommand, ListOptions};

mod state;
use state::{User, Group, LunchBotState, Proposal, StateUpdateCallbacks, update_state};

mod storage;

impl<'a> StateUpdateCallbacks for &'a IrcClient {
    fn get_list_of_users(&self, channel: &str) -> Vec<User> {
        if let Some(list) = self.list_users(channel) {
            list.into_iter()
                .map(|u| u.get_nickname().to_string())
                .collect()
        } else {
            vec![]
        }
    }
}

fn run() -> Result<(), Error> {
    let server: String = std::env::var("LUNCHBOT_SERVER")?;
    let channel: String = std::env::var("LUNCHBOT_CHANNEL")?;
    let port: u16 = std::env::var("LUNCHBOT_PORT")?.parse()?;

    let config = Config {
        nickname: Some("lunchbot".to_owned()),
        server: Some(server),
        channels: Some(vec![channel]),
        port: Some(port),
        ..Default::default()
    };

    let mut reactor = IrcReactor::new()?;
    let client = match reactor.prepare_client_and_connect(&config) {
        Ok(c) => c,
        Err(_e) => {
            error!("Could not connect to the server: {}", &config.server.unwrap());
            panic!("Don't know how to handle this error yet")
        },
    };
    client.identify()?;

    let state = Arc::new(Mutex::new(LunchBotState::new("#rust-spam")));

    let send_interval = tokio_timer::wheel()
        .tick_duration(Duration::from_secs(1))
        .num_slots(256)
        .build()
        .interval(Duration::from_secs(60));

    let sc = state.clone();

    reactor.register_future(send_interval
        .map_err(IrcError::Timer)
        .for_each(move |_| {
            // Anything in here will happen every 60 seconds!
            let state = &mut sc.lock().unwrap();
            let num_before = state.num_of_proposals();
            state.remove_old_proposals();
            let num_after = state.num_of_proposals();
            info!("Removing {} old proposals", num_before - num_after);
            //send_client.send_privmsg("#rust-spam", "AWOOOOOOOOOO")
            Ok(())
        })
    );

    if let Ok(v) = std::env::var("LUNCHBOT_BACKUP_FILE") {
        let backup_interval = tokio_timer::wheel()
            .tick_duration(Duration::from_secs(1))
            .num_slots(256)
            .build()
            .interval(Duration::from_secs(300));

        let sc = state.clone();

        reactor.register_future(
            backup_interval.map_err(IrcError::Timer)
                .for_each(move|_| {
                    if let Err(e) = storage::backup_state(&sc.lock().unwrap(), Path::new(&v)) {
                        error!("Failed to backup the state: {}", e);
                    }
                    Ok(())
                })
        );

    }

    reactor.register_client_with_handler(client,move|irc_client, message| {
        print!("{}", message);
        match message.command {
            Command::PRIVMSG(ref target, ref line) => {
                if line.starts_with("lb ") {
                    let message = update_state(line, state.clone(), &irc_client);
                    let _ = irc_client.send_privmsg(
                        target,
                        &message
                    );
                }
            }
            _ => (),
        }
        Ok(())
    });
    reactor.run()?;

    Ok(())
}

fn main() {
    // Set up logging
    //env_logger::init();
    use env_logger::{Builder, Target};
    use log::LevelFilter;

    let mut builder = Builder::new();
    builder.target(Target::Stdout).filter_level(LevelFilter::Info);
    builder.init();

    info!("Starting up");

    if let Err(e) = run() {
        error!("{}", e);
    }
}
