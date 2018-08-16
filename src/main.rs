extern crate env_logger;
extern crate failure;
extern crate irc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate tokio_timer;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use failure::Error;
use irc::client::prelude::*;
use irc::error::IrcError;
use irc::proto::caps::Capability;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod syntax;
use syntax::LunchCommand;

mod state;
use state::{update_state, LunchBotState, StateUpdateCallbacks, User};

mod storage;

impl<'a> StateUpdateCallbacks for &'a IrcClient {
    fn get_list_of_users(&self, channel: &str) -> Vec<User> {
        if let Some(list) = self.list_users(channel) {
            list.into_iter()
                .map(|u| u.get_nickname().to_string())
                .collect()
        } else {
            error!("The user list cannot be acquired");
            vec![]
        }
    }
}

fn run() -> Result<(), Error> {
    let nick: String = std::env::var("LUNCHBOT_NICK")?;
    let server: String = std::env::var("LUNCHBOT_SERVER")?;
    let channel: String = std::env::var("LUNCHBOT_CHANNEL")?;
    let port: u16 = std::env::var("LUNCHBOT_PORT")?.parse()?;
    let backup_file = std::env::var("LUNCHBOT_BACKUP_FILE");

    let mut state = LunchBotState::new(&channel);

    if let Ok(file_name) = &backup_file {
        if let Err(e) = storage::recover_state(&mut state, Path::new(&file_name)) {
            error!("Failed to recover state: {}", e);
        }
    }

    let config = Config {
        nickname: Some(nick),
        server: Some(server),
        channels: Some(vec![channel]),
        port: Some(port),
        ..Default::default()
    };

    let mut reactor = IrcReactor::new()?;
    let client = match reactor.prepare_client_and_connect(&config) {
        Ok(c) => c,
        Err(_e) => {
            error!(
                "Could not connect to the server: {}",
                &config.server.unwrap()
            );
            panic!("Don't know how to handle this error yet")
        }
    };
    client.send_cap_req(&[Capability::MultiPrefix])?;
    client.identify()?;

    let state = Arc::new(Mutex::new(state));

    let send_interval = tokio_timer::wheel()
        .tick_duration(Duration::from_secs(1))
        .num_slots(256)
        .build()
        .interval(Duration::from_secs(60));

    let sc = state.clone();

    reactor.register_future(send_interval.map_err(IrcError::Timer).for_each(move |_| {
        // Anything in here will happen every 60 seconds!
        let state = &mut sc.lock().unwrap();
        let num_before = state.num_of_proposals();
        state.remove_old_proposals();
        let num_after = state.num_of_proposals();
        let removed = num_before - num_after;
        if removed > 0 {
            info!("Removing {} old proposals", removed);
        }
        //send_client.send_privmsg("#rust-spam", "AWOOOOOOOOOO")
        Ok(())
    }));

    if let Ok(v) = backup_file {
        let backup_interval = tokio_timer::wheel()
            .tick_duration(Duration::from_secs(1))
            .num_slots(256)
            .build()
            .interval(Duration::from_secs(300));

        let sc = state.clone();

        reactor.register_future(backup_interval.map_err(IrcError::Timer).for_each(move |_| {
            if let Err(e) = storage::backup_state(&sc.lock().unwrap(), Path::new(&v)) {
                error!("Failed to backup the state: {}", e);
            }
            Ok(())
        }));
    }

    reactor.register_client_with_handler(client, move |irc_client, message| {
        match message.command {
            Command::PRIVMSG(ref target, ref line) => {
                if line.starts_with("lb ") {
                    // Update state and store the response
                    let response = update_state(line, state.clone(), &irc_client);
                    if let Some(t) = message.response_target() {
                        if let Err(e) = irc_client.send_privmsg(t, &response) {
                            error!("send_privmsg: {:?}", e);
                        }
                    } else {
                        error!("response_target is None; fallback to PRIVMSG::target");
                        if let Err(e) = irc_client.send_privmsg(target, &response) {
                            error!("send_privmsg: {:?}", e);
                        }
                    }
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
    builder
        .target(Target::Stdout)
        .filter_level(LevelFilter::Info);
    builder.init();

    info!("Starting up");

    if let Err(e) = run() {
        error!("{}", e);
    }
}
