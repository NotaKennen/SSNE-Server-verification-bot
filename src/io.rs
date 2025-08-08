use std::fs::{read_to_string, write, create_dir_all};

/*
The database is formatted like this:
(dc-ping) (mc-name) (mc-uuid)

The database file is in the path
{DB_PATH} / (server-id) / {DB_FILE}

The database *folder* (not file!) also stores the server's config.
*/

static DB_PATH: &str = "DBs";           // Name of the folder where DBs are stored
static DB_FILE: &str = "names.txt";     // Name of the DB file (.txt my favorite db format)

// TODO: Database code rewrite (start using a better format or something)

/// Insert a name to the DB
pub fn insert_name_to_db(guild_id: i64, dc_user: &str, mc_user: &str) {
    let previous_text: String = match read_to_string(format!("{}/{}/{}", DB_PATH, guild_id, DB_FILE)) {
        Ok(text) => {text},
        Err(_) => {"".to_string()}
    };
    let _ = create_dir_all(format!("{}/{}", DB_PATH, guild_id.to_string()));
    write(format!("{}/{}/{}", DB_PATH, guild_id, DB_FILE), format!("{}{} {}\n", previous_text, dc_user, mc_user)).unwrap();
}

/// Remove a name from the DB (automatically runs for both mc and dc username)
pub fn remove_name_from_db(guild_id: i64, username: &str) {
    // get names and a new namevec
    let names = get_names(guild_id);
    let mut purgednames: Vec<[String; 2]> = vec![];
     
    // Find and remove the extra name
    for name in names {
        if !(name[0] == username) && !(name[1] == username) {
            purgednames.push(name);
        }
    }

    // Push back the old names
    clear_name_db(guild_id);
    for name in purgednames {
        insert_name_to_db(guild_id, &name[0], &name[1]);
    }
}

/// Delete the entire name DB (don't use unless necessary)
fn clear_name_db(guild_id: i64) {
    write(format!("{}/{}/{}", DB_PATH, guild_id, DB_FILE), "").unwrap();
}

/// Gets all the names in the name DB, in format {dc, mc}
pub fn get_names(guild_id: i64) -> Vec<[String; 2]> {
    // get names or ""
    let names = match read_to_string(format!("{}/{}/{}", DB_PATH, guild_id, DB_FILE)) {
        Ok(text) => {text}
        Err(_) => {"".to_string()}
    };

    // construct a Vec<[String; 2]> (the return value)
    let name_vec: Vec<&str> = names.split("\n").collect();
    let mut return_vec: Vec<[String; 2]> = vec![];
    for name in name_vec {
        if name == "" {continue} // Check for the \n at the end
        let names: Vec<&str> = name.split(" ").collect();
        let dc_name = names[0];
        let mc_name = names[1];

        return_vec.push([dc_name.to_string(), mc_name.to_string()]);
    };
    
    return_vec
}

/// Get a config value from a guild
pub fn get_guild_config(guild_id: i64, config_name: &str) -> Option<String> {
    match read_to_string(format!("{}/{}/{}", DB_PATH, guild_id, config_name)) {
        Ok(text) => {Some(text)},
        Err(_) => {None}
    }
}

/// Push a config value into a guild DB
pub fn push_guild_config(guild_id: i64, config_name: &str, value: String) {
    let _ = create_dir_all(format!("{}/{}", DB_PATH, guild_id.to_string())); // ensure everything exists
    write(format!("{}/{}/{}", DB_PATH, guild_id, config_name), value).unwrap();
}