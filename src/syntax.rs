use regex::{Captures, Match, Regex};
use std::str::FromStr;

lazy_static! {
        static ref ADD_CMD_REGEX: Regex = Regex::new(r"lb add (\d+)").unwrap();
        static ref ADD_USER_CMD_REGEX: Regex = Regex::new(r"lb add (\w+) to (\w+)").unwrap();
        static ref GROUP_CMD_REGEX: Regex = Regex::new(r"lb group (?:(add) (\w+) ([\w,]+)|(remove) (\w+))").unwrap();
        static ref PROPOSE_CMD_REGEX: Regex = Regex::new(r"lb propose (\w+)(?: at)? ([\w:]+)(?: to (\w+))?").unwrap();
        static ref LIST_CMD_REGEX: Regex = Regex::new(r"lb list(?: (groups|proposals))?").unwrap();
}

#[derive(Debug, Eq, PartialEq)]
pub enum ListOptions {
    Groups,
    Proposals,
}

#[derive(Debug, Eq, PartialEq)]
pub enum LunchCommand<'a>{
    Add(u32),
    AddUser(&'a str, &'a str),
    GroupAdd(&'a str, Vec<&'a str>),
    GroupRemove(&'a str),
    List(ListOptions),
    Propose(&'a str, &'a str, Option<&'a str>), //(place, time, group)
}

fn add(caps: Captures) -> Option<LunchCommand> {
    let arg = caps.get(1)?.as_str();
    Some(LunchCommand::Add(u32::from_str(arg).ok()?))
}

fn add_user(caps: Captures) -> Option<LunchCommand> {
    let user = caps.get(1)?.as_str();
    let group = caps.get(2)?.as_str();
    Some(LunchCommand::AddUser(user, group))
}

fn group(caps: Captures) -> Option<LunchCommand> {
    if let Some(_) = caps.get(1) {
        let name = caps.get(2)?.as_str();
        let users = caps.get(3)?.as_str();
        let users: Vec<&str> = users.split(',').collect();
        Some(LunchCommand::GroupAdd(name, users))
    } else if let Some(_) = caps.get(4) {
        let name = caps.get(5)?.as_str();
        Some(LunchCommand::GroupRemove(name))
    } else {
        None
    }
}

fn propose(caps: Captures) -> Option<LunchCommand> {
    let place = caps.get(1)?.as_str();
    let time = caps.get(2)?.as_str();
    let group = caps.get(3).map(|g| g.as_str());
    Some(LunchCommand::Propose(place, time, group))
}

fn list(caps: Captures) -> Option<LunchCommand> {
    if let Some(option) = caps.get(1) {
        match option.as_str() {
            "groups" => Some(LunchCommand::List(ListOptions::Groups)),
            "proposals" => Some(LunchCommand::List(ListOptions::Proposals)),
            _ => None
        }
    } else {
        Some(LunchCommand::List(ListOptions::Proposals))
    }
}

pub fn parse_command(line: &str) -> Option<LunchCommand> {
    if let Some(caps) = ADD_CMD_REGEX.captures(line) {
        add(caps)
    } else if let Some(caps) = ADD_USER_CMD_REGEX.captures(line) {
        add_user(caps)
    } else if let Some(caps) = GROUP_CMD_REGEX.captures(line) {
        group(caps)
    } else if let Some(caps) = PROPOSE_CMD_REGEX.captures(line) {
        propose(caps)
    } else if let Some(caps) = LIST_CMD_REGEX.captures(line) {
        list(caps)
    } else {
        None
    }
}

#[test]
fn test_add_cmd() {
    assert_eq!(Some(LunchCommand::Add(5)),
               parse_command("lb add 5"))
}

#[test]
fn test_add_user_cmd() {
    assert_eq!(Some(LunchCommand::AddUser("honza", "coreserv1")),
               parse_command("lb add honza to coreserv1"))
}

#[test]
fn test_group_add_cmd() {
    assert_eq!(Some(LunchCommand::GroupAdd("coreserv1", vec!["jan", "ondra", "tester"])),
               parse_command("lb group add coreserv1 jan,ondra,tester"))
}

#[test]
fn test_group_remove_cmd() {
    assert_eq!(Some(LunchCommand::GroupRemove("coreserv1")),
               parse_command("lb group remove coreserv1"))
}

#[test]
fn test_list_cmd() {
    assert_eq!(Some(LunchCommand::List(ListOptions::Proposals)),
               parse_command("lb list"))
}

#[test]
fn test_list_groups_cmd() {
    assert_eq!(Some(LunchCommand::List(ListOptions::Groups)),
               parse_command("lb list groups"))
}

#[test]
fn test_list_proposals_cmd() {
    assert_eq!(Some(LunchCommand::List(ListOptions::Proposals)),
               parse_command("lb list proposals"))
}

#[test]
fn test_propose_cmd() {
    assert_eq!(Some(LunchCommand::Propose("winston", "10:55", None)),
               parse_command("lb propose winston 10:55"))
}

#[test]
fn test_propose_to_group_cmd() {
    assert_eq!(Some(LunchCommand::Propose("winston", "10:55", Some("corserv1"))),
               parse_command("lb propose winston 10:55 to corserv1"))
}
