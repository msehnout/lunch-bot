use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde_json;

use super::syntax::{parse_command, ListOptions};

pub type User = String;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Group {
    name: String,
    users: Vec<User>,
}

impl Group {
    pub fn new<T>(name: T, users: Vec<T>) -> Group
    where
        T: Into<String>,
    {
        Group {
            name: name.into(),
            users: users.into_iter().map(Into::into).collect(),
        }
    }

    pub fn push_user<T>(&mut self, user: T)
    where
        T: Into<String>,
    {
        self.users.push(user.into());
    }

    /// When using IRC, we usually set names with some appendix such as
    /// |mtg or |lunch, so we need to update basic names with these
    pub fn update_names(&self, users: Vec<User>) -> Group {
        Group {
            name: String::new(),
            users: self
                .users
                .iter()
                .filter_map(|base_user| {
                    users
                        .iter()
                        .find(|current_user| current_user.starts_with(base_user))
                        .map(|u| u.to_string())
                })
                .collect(),
        }
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.users.join(","))
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct Proposal {
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
    where
        T: Into<String>,
    {
        Proposal {
            place: place.into(),
            time: time.into(),
            group: None,
            created: SystemTime::now(),
        }
    }

    pub fn new_with_group<T>(place: T, time: T, group: T) -> Proposal
    where
        T: Into<String>,
    {
        Proposal {
            place: place.into(),
            time: time.into(),
            group: Some(group.into()),
            created: SystemTime::now(),
        }
    }
}

pub trait StateUpdateCallbacks {
    fn get_list_of_users(&self, channel: &str) -> Vec<User>;
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct LunchBotState {
    groups: Vec<Group>,
    proposals: Vec<Proposal>,
    store: u32,
    channel: String,
}

impl LunchBotState {
    pub fn new(channel: &str) -> Self {
        LunchBotState {
            groups: vec![],
            proposals: vec![],
            store: 0,
            channel: channel.to_owned(),
        }
    }

    fn get_group<'a>(&'a mut self, name: &str) -> Option<&'a mut Group> {
        self.groups.iter_mut().find(|g| g.name == name)
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

    pub fn list_of_groups(&self) -> String {
        self.groups
            .iter()
            .map(|g| g.name.clone())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn remove_old_proposals(&mut self) {
        let dur = Duration::from_secs(60 * 60 * 2);
        self.proposals.retain(|p| {
            if let Ok(d) = p.created.elapsed() {
                d < dur
            } else {
                true
            }
        });
    }

    pub fn num_of_proposals(&self) -> usize {
        self.proposals.len()
    }
}

pub fn update_state<T>(line: &str, state: Arc<Mutex<LunchBotState>>, cb: &T) -> String
where
    T: StateUpdateCallbacks,
{
    use LunchCommand::*;

    let cmd = parse_command(line);
    if let Some(ref cmd) = cmd {
        info!("Incoming command: {:?}", cmd);
    }
    match cmd {
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
                        let users = cb.get_list_of_users(&channel);
                        let updated_names = g.update_names(users);
                        info!(
                            "Proposal {:?}, group {:?}, names {:?}",
                            proposal, g, updated_names
                        );
                        ret = format!("{} go to {} at {}", updated_names, place, time);
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
        Some(List(opt)) => match opt {
            ListOptions::Proposals => {
                let proposals = &state.lock().unwrap().proposals;
                format!("All proposals: {:?}", proposals)
            }
            ListOptions::Groups => {
                let groups = state.lock().unwrap().list_of_groups();
                format!("Groups: {}", groups)
            }
        },
        Some(DumpState) => {
            let state: &LunchBotState = &state.lock().unwrap();
            serde_json::to_string(state).unwrap_or("failed to dump state".to_string())
        }
        Some(RestoreState(input_state_string)) => {
            if let Ok(new_state) = serde_json::from_str(input_state_string) {
                let state: &mut LunchBotState = &mut state.lock().unwrap();
                *state = new_state;
                format!("Success")
            } else {
                format!("Fail")
            }
        }
        _ => include_str!("../usage").to_string(),
    }
}
