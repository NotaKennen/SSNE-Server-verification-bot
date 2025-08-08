use serenity::all::{CreateEmbed, CreateMessage, GuildId, Http, RoleId, UserId};
use serenity::builder::EditMember;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use std::fs::read_dir;

mod io;
use io::*;

mod structs;
use structs::*;

mod wynnapi;
use wynnapi::*;

static BOT_VERSION: &str = "DEVELOPMENT v1.1.0";

// TODO: make verify and unverify functions use better args
/// Verify a member
async fn verify(ctx: &Context, msg: &Message, args: Vec<&str>) {
    // Make user objects
    let dc_username = args[1];
    let dc_user = match DcUsername::try_from_pingid(dc_username) {
        Some(dcuser) => dcuser,
        None => {msg.reply(&ctx, "Please ping the user or format as <@(userid)>").await.unwrap(); return}
    };
    let mc_username = args[2];
    let mc_user = match McUsername::try_new_from_name(mc_username).await {
        Some(user) => {user},
        None => {msg.reply(&ctx, "Username isn't valid or isn't found through the Mojang API. Are you sure you wrote it correctly?").await.unwrap(); return;}
    };

    // Commit to DB
    let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
    if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please. (Member was not verified)").await.unwrap(); return;}
    insert_name_to_db(guild_id, dc_user, mc_user);

    // Add role (messy af ik)
    let str_role_id = match get_guild_config(guild_id, "verified-role-id") {
        Some(id) => {id},
        None => {"0".to_string()} };
    let role_id = match str_role_id.parse::<u64>() {Ok(id) => {id}, Err(_) => {0}};
    let mod_dc_username = dc_username.strip_prefix("<@").unwrap().strip_suffix(">").unwrap();
    let target_id_int = match mod_dc_username.parse::<u64>() {Ok(id) => {id}, Err(_) => {0} };
    let target_id = UserId::new(target_id_int);
    let target_member = msg.guild_id.unwrap().member(&ctx, target_id).await.unwrap();
    let mut target_mem_clone = target_member.clone(); // Make a clone for later (nick change)
    if role_id != 0 { target_member.add_role(&ctx, RoleId::new(role_id)).await.unwrap(); }

    // Add nickname
    let builder = EditMember::new().nickname(mc_username);
    let _ = target_mem_clone.edit(&ctx, builder).await; // don't unwrap this because perms might sometimes block nick changes and stop code here

    // Respond
    msg.reply(&ctx, format!("User \"{}\" has been verified as \"{}\".", dc_username, mc_username)).await.unwrap();
}

/// Unverify a member
async fn unverify(ctx: &Context, msg: &Message, args: Vec<&str>) {
    // check that it's a discord username
    if !(args[1].starts_with("<@") && args[1].ends_with(">")) {
        msg.reply(&ctx, "Please ping the user or format as <@(userid)>").await.unwrap(); return;
    }

    // Get verified-role-id 
    let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
    let str_role_id = match get_guild_config(guild_id, "verified-role-id") {Some(roleid) => {roleid}, None => {"0".to_string()}};
    let role_id = match str_role_id.parse::<u64>() {Ok(id) => {id}, Err(_) => {0}};
    if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please. (User wasn't unverified)").await.unwrap(); return;}
    if role_id != 0 {

    // Remove role from user
    let mod_dc_username = args[1].strip_prefix("<@").unwrap().strip_suffix(">").unwrap();
    let target_id_int = match mod_dc_username.parse::<u64>() {Ok(id) => {id}, Err(_) => {0} };
    let target_id = UserId::new(target_id_int);
    let target_member = msg.guild_id.unwrap().member(&ctx, target_id).await.unwrap();
    target_member.remove_role(&ctx, role_id).await.unwrap();
    } // Note that this (^) only runs if role id != 0 (if in previous chunk (might be hard to see))
    
    // Commit and reply
    remove_name_from_db(guild_id, DcUsername::new_from_pingid(args[1]));
    msg.reply(&ctx, "User has been unverified").await.unwrap();
}

// TODO: start using the above unverify() function instead of a helper
// needs the better arguments, so can't do that yet
/// (Helper, don't use) Unverifies a member with better args
async fn update_unverify_helper(guild_id: i64, dc_user: DcUsername) {
    // Get verified-role-id
    let str_role_id = match get_guild_config(guild_id, "verified-role-id") {Some(roleid) => {roleid}, None => {"0".to_string()}};
    let role_id = match str_role_id.parse::<u64>() {Ok(id) => {id}, Err(_) => {0}};
    if guild_id == 0 {return;}
    if role_id != 0 {

    // Remove role from user
    let target_id = UserId::new(dc_user.rawid as u64);
    let guild: GuildId = GuildId::new(guild_id as u64);
    let cache = Http::new(&std::fs::read_to_string("token.txt").unwrap()); // TODO: maybe don't re-read the token? (move to scheduler?)
    let target_member = guild.member(&cache, target_id).await.unwrap();
    target_member.remove_role(cache, role_id).await.unwrap();
    } // Note that this (^) only runs if role id != 0 (if in previous chunk (might be hard to see))
    
    // Commit and reply
    remove_name_from_db(guild_id, dc_user);
}

/// Run the guild wynn-dc syncing
async fn run_wynndc_update() -> () {
    let guild_ids: Vec<i64> = {
        let mut guild_ids: Vec<i64> = vec![];
        let db_dir = &get_db_statics()[0];
        let dir = read_dir(db_dir).unwrap();
        for entry in dir { // MIGHT cause crashes if there ever is a file in the DB directory (shouldn't happen anyway)
            let path = entry.unwrap().path();
            let name = path.to_str().unwrap();
            let db_prefix = format!("{}/", &get_db_statics()[0]);
            let int_id = match name.strip_prefix(&db_prefix).unwrap().parse::<i64>() {Ok(id) => {id}, Err(_) => {0}};
            if int_id == 0 {continue} // in case of some weird bug
            guild_ids.push(int_id);
        } guild_ids
    };
    println!("STATUS: Search got {} guilds to sync", guild_ids.len());
    let mut member_removed_counter = 0;
    for guild_id in guild_ids { // Not indented cause it's so big

    // get necessary info
    if guild_id == 0 {return} 
    let guildname = match get_guild_config(guild_id, "wynn-guild-name") {
        Some(name) => {name},
        None => {return} // normally just return, debug for now
    }; 
    
    // Get members from dc and wynn
    if !is_real_guild(&guildname).await {return} // return if not real guild
    let box_wynnguildmembers = get_guild_members(&guildname).await;
    let box_dcguildmembers = get_names(guild_id);

    // Take the boxes out and leave them as Vec<String> ( Vec<mc_uuid> )
    let mut wynn_guildmembers: Vec<String> = vec![];
    let mut dc_guildmembers: Vec<String> = vec![];
    for name in box_wynnguildmembers.clone() {wynn_guildmembers.push(name[1].to_string())}
    for name in box_dcguildmembers.clone() {dc_guildmembers.push(name[2].to_string())}

    // Compare names
    let mut index = 0;
    for dc_mc_uuid in dc_guildmembers { 
        let mut memberstatus = false; // default not found
        for d_wynn_uuid in &wynn_guildmembers {
            let wynn_uuid = &d_wynn_uuid.replace("-", ""); // Wynncraft adds dashed to mc UUIDs
            if wynn_uuid == &dc_mc_uuid { 
                memberstatus = true;
            } // Found from guild, safe
        }
        if memberstatus == false { // NOT found from guild, unverify
            let dc_user: DcUsername = DcUsername::new_from_pingid(
                // get pingid by using the previous database variables
                &box_dcguildmembers[index][0]
            );
            member_removed_counter += 1;
            update_unverify_helper(guild_id, dc_user).await;
        }
        index += 1;
    }}
    println!("STATUS: Removed {} members", member_removed_counter);
}

struct Handler; 

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.content.starts_with("w!") {return} // Ignore a message if it doesn't start with the prefix so we dont waste time going through it

        if msg.content.starts_with("w!help") {
            msg.reply(&ctx, format!(
"Running bot version **{}**
Help with commands:

__w!verify [discord-ping] [minecraft-user]__
*Verifies a person and links their MC and DC names together, also gives them the verified role. Make sure to ping the member so it surely gets the right person*

__w!unverify [discord-ping]__
*Unverifies a person, also removes their role*

__w!whois [dc | mc] [name]__
*Gives you the mc name of someone based on their dc name, or the other way around*

__w!list__
*List all verified members*

__w!verifiedrole [role-id]__
*Use this command to specify the role used for verified members*

The command to verify people is only available to people with the Manage Roles permission
Commands to manage the verification system are only available to people with Administrator",
BOT_VERSION
            )).await.unwrap();
            return;
        }

        else if msg.content.starts_with("w!list") {
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            let names = get_names(guild_id);

            // In case no one is verified
            if names.len() == 0 {msg.reply(&ctx, "This server has no verified members").await.unwrap(); return;}

            // Make embed for the thing
            let mut embed = CreateEmbed::new();
            embed = embed.title("Verified members");
            for name in names {embed = embed.field(&name[1], &name[0], false);}
            let builder = CreateMessage::new().embed(embed);

            msg.channel_id.send_message(&ctx, builder).await.unwrap();
        }

        else if msg.content.starts_with("w!whois") {
            // Get and check arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 3 {msg.reply(&ctx, "Command requires 2 arguments\nUsage: w!whois [dc | mc] [discord-name]").await.unwrap(); return;}

            // Run the set mode and check for names (I'm aware this code is ass)
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            let namelist = get_names(guild_id);
            if args[1].to_lowercase() == "dc" {
                for name in namelist {
                    if name[0] == args[2] { // Name matches
                        msg.reply(&ctx, format!("User is verified as \"{}\" in Minecraft", name[1])).await.unwrap();
                    }
                }
            } else if args[1].to_lowercase() == "mc" {
                for name in namelist {
                    if name[1] == args[2] { // Name matches
                        msg.reply(&ctx, format!("User is verified as \"{}\" in Discord", name[0])).await.unwrap();
                    }
                }
            } else {
                msg.reply(&ctx, format!("\"{}\" is not a valid mode, use DC or MC", args[1])).await.unwrap();
            }
        }

        // TODO: Feature: Statistics page 
        // Could list kicked members (amount), % of members (verified) on dc, total members ever

        // Get permission level (all that just for permissions dawg) (I am NOT solving these unwraps fuck you)
        let author_perms = msg.guild_id.unwrap().to_partial_guild(&ctx).await.unwrap().user_permissions_in(&msg.channel_id.to_channel(&ctx).await.unwrap().guild().unwrap(), &msg.member(&ctx).await.unwrap());
        if !author_perms.contains(serenity::model::Permissions::MANAGE_ROLES) {msg.reply(&ctx, "You don't have the permission to do that").await.unwrap(); return}
        // - - - MANAGE ROLES PERMISSION LEVEL - - - 

        if msg.content.starts_with("w!verify") {
            // Get and check arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 3 {msg.reply(&ctx, "Command requires 2 arguments\nUsage: w!verify [discord-ping] [minecraft-user]").await.unwrap(); return;}
            verify(&ctx, &msg, args).await; // Commit
        }

        else if msg.content.starts_with("w!unverify") {
            // get arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 2 {msg.reply(&ctx, "Command requires 1 argument\nUsage: w!unverify [discord-ping]").await.unwrap(); return;}
            unverify(&ctx, &msg, args).await; // Commit
        } 

        // Next perms check level
        if !author_perms.contains(serenity::model::Permissions::ADMINISTRATOR) {msg.reply(&ctx, "You don't have the permission to do that").await.unwrap(); return}
        // - - - ADMINISTRATOR PERMISSION LEVEL - - - 

        if msg.content.starts_with("w!verifiedrole") {
            // Get and check arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 2 {msg.reply(&ctx, "Command requires 1 argument\nUsage: w!verifiedrole [role-id]").await.unwrap(); return;}

            // Format the provided (str) id to a proper integer
            let role_id = match args[1].parse::<u64>() {Ok(id) => {id}, Err(_) => {0}};
            if role_id == 0 {msg.reply(&ctx, "The ID you provided couldn't be parsed (probably incorrect)").await.unwrap(); return;}

            // Push the role id to the config file 
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            push_guild_config(guild_id, "verified-role-id", role_id.to_string());

            // Answer
            msg.reply(&ctx, "Verified role has been saved").await.unwrap();
        }
    
        else if msg.content.starts_with("w!guildname") {
            // Get and check arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 2 {msg.reply(&ctx, "Command requires 1 argument\nUsage: w!guildname [guild-prefix]").await.unwrap(); return;}

            // Check if it's real
            if !is_real_guild(args[1]).await {
                msg.reply(&ctx, "Could not find guild from Wynn's API. Did you write the name correctly? Use the 4 letter identification name instead of the full one.").await.unwrap();
                return;
            }

            // Commit
            let guild_id: i64 = match msg.guild_id {
                Some(id) => {id.into()},
                None => {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            };
            push_guild_config(guild_id, "wynn-guild-name", args[1].to_string());
            msg.reply(&ctx, "Guild name has been saved").await.unwrap();
        }

        /* if msg.content.starts_with("w!debug_runupdate") {
            println!("DEBUG: Ran forced update");
            run_wynndc_update().await
        } */ // Command to force run update (disabled obv)
    }

    async fn ready(&self, _: Context, ready: Ready) {
        // Inform readiness
        println!("{} is connected!", ready.user.name);

        // Start scheduler for updates (do an update every hour) 
        // Tbh I barely understand what's going on here
        if true { // nice toggle
            let mut interval_timer = tokio::time::interval(chrono::Duration::hours(1).to_std().unwrap());
            tokio::spawn( async move { loop {
                interval_timer.tick().await;
                tokio::spawn( async move {
                    println!("STATUS: Running updater");
                    run_wynndc_update().await;
                    println!("STATUS: Hourly update done!")
                    }
                );
            }}
            );
        }
    }
}

#[tokio::main]
async fn main() {
    // Load token from file
    let token = std::fs::read_to_string("token.txt").expect("Unable to read token file");

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client =
        Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
