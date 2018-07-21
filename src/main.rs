extern crate irc;
extern crate env_logger;
extern crate failure;
#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
extern crate regex;
extern crate tokio_timer;

use failure::Error;
use irc::client::prelude::*;
use irc::error::IrcError;
use regex::{Regex};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

mod syntax;
use syntax::{LunchCommand, ListOptions};

type User = String;

#[derive(Debug)]
struct Group {
    name: String,
    users: Vec<User>,
}

impl Group {
    pub fn new<T>(name: T, users: Vec<T>) -> Group
        where T: Into<String> {
        Group {
            name: name.into(),
            users: users.into_iter().map(Into::into).collect(),
        }
    }

    pub fn push_user<T>(&mut self, user: T)
        where T: Into<String> {
        self.users.push(user.into());
    }

    /// When using IRC, we usually set names with some appendix such as
    /// |mtg or |lunch, so we need to update basic names with these
    pub fn update_names(&self, users: Vec<User>) -> Group {
        Group {
            name: String::new(),
            users: self.users.iter()
                .filter_map(|base_user| {
                    users.iter()
                        .find(|current_user| {
                            current_user.starts_with(base_user)
                        })
                        .map(|u| u.to_string())
                })
                .collect()
        }
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.users.join(","))
    }
}

struct Proposal {
    place: String,
    time: String,
    group: Option<String>,
    created: SystemTime,
}

impl fmt::Debug for Proposal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} at {}", self.place, self.time)
    }
}

impl Proposal {
    pub fn new<T>(place: T, time: T) -> Proposal
        where T: Into<String> {
        Proposal {
            place: place.into(),
            time: time.into(),
            group: None,
            created: SystemTime::now(),
        }
    }

    pub fn new_with_group<T>(place: T, time: T, group: T) -> Proposal
        where T: Into<String> {
        Proposal {
            place: place.into(),
            time: time.into(),
            group: Some(group.into()),
            created: SystemTime::now(),
        }
    }
}

trait StateUpdateCallbacks {
    fn get_list_of_users(&self, channel: &str) -> Vec<User>;
}

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

struct LunchBotState {
    groups: Vec<Group>,
    proposals: Vec<Proposal>,
    store: u32,
    channel: String,
}

impl LunchBotState {
    fn new(channel: &str) -> Self {
        LunchBotState {
            groups: vec![],
            proposals: vec![],
            store: 0,
            channel: channel.to_owned(),
        }
    }

    fn get_group<'a>(&'a mut self, name: &str) -> Option<&'a mut Group> {
        self.groups.iter_mut()
            .find(|g| g.name == name)
    }

    fn remove_group(&mut self, name: &str) -> bool {
        let length = self.groups.len();
        self.groups.retain(|g| g.name != name);
        if self.groups.len() < length {
            true
        } else {
            false
        }
    }

    fn list_of_groups(&self) -> String {
        self.groups.iter()
            .map(|g| g.name.clone())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn remove_old_proposals(&mut self) {
        let dur = Duration::from_secs(60*60*2);
        self.proposals.retain(|p| {
            if let Ok(d) = p.created.elapsed() {
                d < dur
            } else {
                true
            }
        });
    }

    fn num_of_proposals(&self) -> usize {
        self.proposals.len()
    }
}

fn update_state<T>(line: &str, state: Arc<Mutex<LunchBotState>>, cb: &T) -> String
  where T: StateUpdateCallbacks {
    use LunchCommand::*;

    match syntax::parse_command(line) {
        Some(Add(n)) => {
            let store = &mut state.lock().unwrap().store;
            *store += n;
            format!("Store: {}", *store)
        }
        Some(AddUser(user, group)) => {
            let state = &mut state.lock().unwrap();
            if let Some(g) = state.get_group(group) {
                g.push_user(user);
                format!("Group {} updated: {}", g.name, g)
            } else {
                format!("No group named {}", group)
            }
        }
        Some(GroupAdd(name, users)) => {
            let group = Group::new(name, users);
            let ret = format!("New group: {} - {}", name, group);
            {
                let groups = &mut state.lock().unwrap().groups;
                groups.push(group);
            }
            ret
        }
        Some(GroupRemove(name)) => {
            let state = &mut state.lock().unwrap();
            if state.remove_group(name) {
                format!("Group {} has been removed", name)
            } else {
                format!("No such group: {}", name)
            }
        }
        Some(Propose(place, time, group)) => {
            if let Some(group) = group {
                let proposal = Proposal::new_with_group(place, time, group);
                let ret;
                {
                    let state = &mut state.lock().unwrap();
                    // Unfortunately I need to borrow in advance in order to prevent lifetime
                    // collisions.
                    let channel = state.channel.clone();
                    if let Some(g) = state.get_group(group) {
                        ret = format!("{} go to {} at {}",
                                      g.update_names(cb.get_list_of_users(&channel)),
                                      place, time);
                    } else {
                        ret = format!("-No such group- go to {} at {}", place, time);
                    }
                    state.proposals.push(proposal);
                }
                ret
            } else {
                {
                    let proposals = &mut state.lock().unwrap().proposals;
                    proposals.push(Proposal::new(place, time));
                }
                format!("New proposal: go to {} at {}", place, time)
            }
        }
        Some(List(opt)) => {
            match opt {
                ListOptions::Proposals => {
                    let proposals = &state.lock().unwrap().proposals;
                    format!("All proposals: {:?}", proposals)
                }
                ListOptions::Groups => {
                    let groups = state.lock().unwrap().list_of_groups();
                    format!("Groups: {}", groups)
                }
            }
        }
        _ => {
            "Hi!".to_string()
        }
    }
}

fn run() -> Result<(), Error> {
    let config = Config {
        nickname: Some("lunchbot".to_owned()),
        server: Some("54.85.60.193".to_owned()),
        channels: Some(vec!["#rust-spam".to_owned()]),
        ..Default::default()
    };

    let mut reactor = IrcReactor::new()?;
    let client = reactor.prepare_client_and_connect(&config).unwrap();
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
            // Anything in here will happen every 10 seconds!
            let state = &mut sc.lock().unwrap();
            let num_before = state.num_of_proposals();
            state.remove_old_proposals();
            let num_after = state.num_of_proposals();
            info!("Removing {} old proposals", num_before - num_after);
            //send_client.send_privmsg("#rust-spam", "AWOOOOOOOOOO")
            Ok(())
        })
    );

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
        println!("{}", e);
    }
}
