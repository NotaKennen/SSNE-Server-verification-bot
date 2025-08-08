use std::fs::{create_dir_all, read_to_string, write, OpenOptions};
use std::io::Write;
use crate::structs::*;

/*
The database is formatted like this:
(dc-ping) (mc-name) (mc-uuid)

The database file is in the path
{DB_PATH} / (server-id) / {DB_FILE}

The database *folder* (not file!) also stores the server's config.
*/

static DB_PATH: &str = "DBs";           // Name of the folder where DBs are stored
static DB_FILE: &str = "names.txt";     // Name of the DB file (.txt my favorite db format)

/// Insert a name to the DB
pub fn insert_name_to_db(guild_id: i64, dc_user: &DcUsername, mc_user: &McUsername) {
    let database_path = format!("{}/{}/{}", DB_PATH, guild_id, DB_FILE);
    create_dir_all(format!("{}/{}", DB_PATH, guild_id)).ok();

    let mut file: std::fs::File = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&database_path)
        .unwrap();

    writeln!(file, "{},{},{}", dc_user.pingid, mc_user.name, mc_user.uuid).ok();
}

/// Remove a name from the DB
pub fn remove_name_from_db(guild_id: i64, username: DcUsername) {
    let database_path = format!("{}/{}/{}", DB_PATH, guild_id, DB_FILE);

    // Load file contents (if error, just return since nothing to remove)
    let db_contents = match read_to_string(&database_path) {
        Err(_) => return,
        Ok(values) => values,
    };

    // Keep only the lines that don't match the username
    let new_contents: String = db_contents
        .lines()
        .filter(|line| {
            // Line format: dc-ping mc-name mc-uuid
            let parts: Vec<&str> = line.split(",").collect();
            if parts.len() < 1 {
                return true; // keep malformed lines
            }
            parts[0] != username.pingid
        })
        .map(|s| s.to_string() + "\n")
        .collect();

    // Overwrite the file with filtered contents
    let _ = write(database_path, new_contents);
}

/// Gets all the names in the server's DB, formatted as Vec<[discord-ping, mc-user, mc-uuid]>
pub fn get_names(guild_id: i64) -> Vec<[String; 3]> {
    let database_path = format!("{}/{}/{}", DB_PATH, guild_id, DB_FILE);
    let db_contents = match read_to_string(&database_path) {
        Err(_) => {return vec![]}
        Ok(values) => {values}
    };

    let lines: Vec<&str> = db_contents.split("\n").collect();
    let mut returnvec: Vec<[String; 3]> = vec![];
    for line in lines {
        if line == "" {continue}
        let values: Vec<&str> = line.split(",").collect();
        returnvec.push([
            values[0].to_string(),
            values[1].to_string(),
            values[2].to_string()
        ])
    }
    return returnvec
}

/// Get a config value from a guild
pub fn get_guild_config(guild_id: i64, config_name: &str) -> Option<String> {
    match read_to_string(format!("{}/{}/{}", DB_PATH, guild_id, config_name)) {
        Ok(text) => {
            if text == "" {return None};
            Some(text)
        },
        Err(_) => {None}
    }
}

/// Push a config value into a guild DB
pub fn push_guild_config(guild_id: i64, config_name: &str, value: String) {
    let _ = create_dir_all(format!("{}/{}", DB_PATH, guild_id.to_string())); // ensure everything exists
    write(format!("{}/{}/{}", DB_PATH, guild_id, config_name), value).unwrap();
}

/// Get the statics DB_PATH and DB_FILE (in that order)
pub fn get_db_statics() -> [String; 2] {
    return [DB_PATH.to_string(), DB_FILE.to_string()]
}