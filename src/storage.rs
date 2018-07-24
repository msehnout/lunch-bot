use failure::Error;
use serde_json;
use state::LunchBotState;
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

fn backup_state(state: &LunchBotState, file_name: &Path) -> Result<(), Error> {
    let mut f = File::create(file_name)?;
    f.write_all(serde_json::to_string(&state)?.as_bytes())?;
    Ok(())
}

fn recover_state(state: &mut LunchBotState, file_name: &Path) -> Result<(), Error> {
    let mut f = File::open(file_name)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    *state = serde_json::from_str(&contents)?;
    Ok(())
}

#[test]
fn backup_and_recover() {
    use state::LunchBotState;

    let state = LunchBotState::new("#ahoj");
    let file_name = Path::new("/tmp/lunch-bot-test-file");
    let _ = backup_state(&state, &file_name);

    let mut state2 = LunchBotState::new("");
    let _ = recover_state(&mut state2, &file_name);

    assert_eq!(state, state2);
}