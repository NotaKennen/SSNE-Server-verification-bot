use std::collections::HashMap;

// Not actually Wynn's api but idc really I don't want to make a new file for this
// https://api.mojang.com/users/profiles/minecraft/{} where {} is username
/// Gets the user's minecraft UUID from mojang api
pub async fn get_mc_uuid(username: &str) -> Option<String> {
    let resp: HashMap<String, serde_json::Value> = reqwest::get(format!("https://api.mojang.com/users/profiles/minecraft/{}", username))
        .await.unwrap()
        .json::<HashMap<String, serde_json::Value>>()
        .await.unwrap();
    if resp.contains_key("id") == false {
        return None
    }
    return Some(resp["id"].as_str().unwrap().to_string()) // force convert to string just to be safe
}

/// Checks whether or not a guild exists, returns true/false
pub async fn is_real_guild(guild_identification: &str) -> bool {
    let req = reqwest::get(format!("https://api.wynncraft.com/v3/guild/prefix/{}", guild_identification)).await.unwrap();
    return req.status() == 200;
}

/// Get the members of a guild, returns in Vec<[String;2]>, where the box is [username, uuid]. Panics on incorrect guild prefix
pub async fn get_guild_members(guild_identification: &str) -> Vec<[String; 2]> {
    // Get response from API
    let resp: HashMap<String, serde_json::Value> = reqwest::get(format!("https://api.wynncraft.com/v3/guild/prefix/{}", guild_identification))
        .await.unwrap()
        .json::<HashMap<String, serde_json::Value>>()
        .await.unwrap();

    // UUID in RET.members.(rank).(name).uuid
    // Put the members in a returnable vec
    let mut memberlist: Vec<[String; 2]> = vec![];
    let guildranks = &resp["members"].as_object().unwrap().clone();
    for rank in guildranks {
        if rank.0 == "total" {continue}
        for member in rank.1.as_object().unwrap().clone() {
            let uuid = member.1["uuid"].as_str().unwrap().to_string();
            memberlist.push([
                member.0.to_string(),   // name
                uuid                    // uuid
            ])
    }}
    return memberlist
}

