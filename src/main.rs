use serenity::all::{ChannelId, CreateEmbed, CreateMessage, GuildId, Http, RoleId, UserId};
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

static BOT_VERSION: &str = "v1.5.1";

/// Verify a member
async fn verify(cache: impl AsRef<Http> + serenity::prelude::CacheHttp, guild_id: i64, target_dc_user: DcUsername, target_mc_user: McUsername) -> Result<String, String> {
    // Make objects to be used later
    let guild_obj = GuildId::new(guild_id as u64);
    let target_userid = UserId::new(target_dc_user.rawid as u64);
    let mut target_member = match guild_obj.member(&cache, target_userid).await {
        Err(_) => {return Err("Couldn't find target member".to_string())},
        Ok(member) => member
    };
    
    // Add verified role
    match get_guild_config(guild_id, "verified-role-id") {
        None => {},
        Some(str_role_id) => {
            let verified_role_id = match str_role_id.parse::<u64>() {Ok(id) => {id}, Err(_) => {1} };
            let verified_role = RoleId::new(verified_role_id);
            let _ = target_member.add_role(&cache, verified_role).await;
        }
    }

    // Remove vet role
    match get_guild_config(guild_id, "veteran-role-id") {
        None => {},
        Some(str_role_id) => {
            let veteran_role_id = match str_role_id.parse::<u64>() {Ok(id) => {id}, Err(_) => {1} };
            let veteran_role = RoleId::new(veteran_role_id);
            let _ = target_member.remove_role(&cache, veteran_role).await;
        }
    }
    
    // Add nickname
    let nick_builder = EditMember::new().nickname(&target_mc_user.name);
    let _ = target_member.edit(&cache, nick_builder).await;

    // Commit to DB
    insert_name_to_db(guild_id, &target_dc_user, &target_mc_user);

    // Respond
    return Ok(format!("Discord user \"{}\" has been verified as \"{}\"", target_dc_user.pingid, target_mc_user.name))
}

/// Unverify a member
async fn unverify(cache: impl AsRef<Http> + serenity::prelude::CacheHttp, guild_id: i64, target_dc_user: DcUsername) -> Result<String, String> {
    // Make objects to be used later
    let guild_obj = GuildId::new(guild_id as u64);
    let target_userid = UserId::new(target_dc_user.rawid as u64);
    let target_member = match guild_obj.member(&cache, target_userid).await {
        Err(_) => {return Err("Couldn't find target member".to_string())},
        Ok(member) => member
    };
    
    // Remove verified role
    match get_guild_config(guild_id, "verified-role-id") {
        None => {},
        Some(str_role_id) => {
            let verified_role_id = match str_role_id.parse::<u64>() {Ok(id) => {id}, Err(_) => {1} };
            let verified_role = RoleId::new(verified_role_id);
            let _ = target_member.remove_role(&cache, verified_role).await;
        }
    }

    // Remove all removable roles
    match get_guild_config(guild_id, "removed-roles") {
        None => {},
        Some(str_role_ids) => {
            let vec_role_ids: Vec<&str> = str_role_ids.split(" ").collect();

            // Run through them and try to remove
            for str_role_id in vec_role_ids {
                let removable_role_id = match str_role_id.parse::<u64>() {Ok(id) => {id}, Err(_) => {1} };
                let _ = target_member.remove_role(&cache, removable_role_id).await;
            }
        }
    }

    // Add vet role
    match get_guild_config(guild_id, "veteran-role-id") {
        None => {},
        Some(str_role_id) => {
            let veteran_role_id = match str_role_id.parse::<u64>() {Ok(id) => {id}, Err(_) => {1} };
            let veteran_role = RoleId::new(veteran_role_id);
            let _ = target_member.add_role(&cache, veteran_role).await;
        }
    }

    // Commit and reply
    remove_name_from_db(guild_id, target_dc_user);
    return Ok("User has been unverified".to_string())
}

// TODO: Eventual rewrite for this as well
/// Run the guild wynn-dc syncing
async fn run_wynndc_update(cache: impl AsRef<Http> + serenity::prelude::CacheHttp) -> () {
    let guild_ids: Vec<i64> = {
        let mut guild_ids: Vec<i64> = vec![];
        let db_dir = &get_db_statics()[0];
        let dir = read_dir(db_dir).unwrap();
        for entry in dir { // MIGHT cause crashes if there ever is a file in the DB directory (shouldn't happen anyway)
            let path = entry.unwrap().path();
            let name = path.to_str().unwrap();
            let db_prefix = format!("{}/", &get_db_statics()[0]);
            let win_db_prefix = format!("{}\\", &get_db_statics()[0]);
            // windows uses \ instead of / (fuckin rats)
            let path_name = match name.strip_prefix(&db_prefix) {
                Some(noprefix_name) => {noprefix_name},
                None => {
                    match name.strip_prefix(&win_db_prefix) {
                        Some(noprefix_name) => {noprefix_name},
                        None => {name}
                    }
                }
            }; 
            let int_id = match path_name.parse::<i64>() {Ok(id) => {id}, Err(_) => {0}};
            
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

    // If notificationchannel is set, check for new members
    match get_guild_config(guild_id, "notif-channel-id") {
        None => {break}
        Some(str_channelid) => {
            let members = match get_guild_config(guild_id, "wynn-guild-members") {
                Some(r) => {r}
                None => {String::new()}
            };
            let wg_members = wynn_guildmembers.clone(); // List of names
            for wg_name in &wg_members {
                let mut namestatus = false;
                for old_name in members.split("\n") {
                    if old_name == wg_name {namestatus = true}
                }
                if !namestatus {
                    let int_channelid = match str_channelid.parse::<u64>() {Ok(id) => {id}, Err(_) => {0}};
                    let notifchannel = ChannelId::new(int_channelid); // ^ should never be 0
                    let wg_username = match get_name_from_uuid(&wg_name).await { // wg_name is UUID
                        Some(name) => {name} // You could get it from the box_wynnguildmembers but I'm lazy
                        None => {continue;}          // Maybe I'll optimize it later if I care
                    }; 
                    let msg_builder = CreateMessage::new().content(format!("**{}** has joined the guild!", wg_username));
                    notifchannel.send_message(&cache, msg_builder).await.unwrap();
                };
            }
            let mut formatted_members = String::new();
            for name in &wg_members {formatted_members.push_str(&format!("{}\n", name));}
            push_guild_config(guild_id, "wynn-guild-members", formatted_members);
        }
    }

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
            let _ = unverify(&cache, guild_id, dc_user).await;
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
Command permissions, arguments and descriptions:

\\- - - Everyone - - -

__w!help__
*Take a guess*

__w!whois ['dc' | 'mc'] [name]__
*Gives you the mc name of someone based on their dc name, or the other way around*

__w!list__
*List all verified members*

\\- - - Manage roles permissions - - -

__w!verify [discord-ping] [minecraft-user]__
*Verifies a person and links their MC and DC names together, also gives them the verified role. Make sure to ping the member so it surely gets the right person*

__w!unverify [discord-ping]__
*Unverifies a person, also removes their role, ping the person so it gets the right one*

\\- - - Administrator - - -

__w!verifiedrole [role-id]__
*Use this command to specify the role used for verified members*

__w!guildname [guild-name]__
*Use this command to specify what wynncraft guild should the bot compare to when doing automatic member management* 

__w!notifchannel [channel-id]__
*Use this command to specify where should join notifications go to*

__w!vetrole [role-id]__
*Use this command to specify the role given to people who've been unverified*

__w!removedroles [role-id1] <role-id2> <role-id3>...__
*Use this command to specify what roles should be removed when a person is unverified (e.g. ranks), you can list multiple, add a space between the IDs.*

See the github repo for more accurate information, source code, or if you have any issues with the bot: 
<https://github.com/NotaKennen/SSNE-Server-verification-bot>",
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
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 3 {msg.reply(&ctx, "Command requires 2 arguments\nUsage: w!verify [discord-ping] [minecraft-user]").await.unwrap(); return;}
            let dc_user = match DcUsername::try_from_pingid(args[1]) {
                Some(obj) => obj,
                None => {let _ = msg.reply(&ctx, "Please ping the user or format as <@id> to ensure it gets the right person.").await; return;}
            };
            let mc_user = match McUsername::try_new_from_name(args[2]).await {
                Some(obj) => obj,
                None => {let _ = msg.reply(&ctx, "Couldn't find the Minecraft user from Mojang's API, did you write it correctly?").await; return;}
            };
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            let resp = verify(&ctx, guild_id, dc_user, mc_user).await; // Commit
            match resp {
                Ok(rsp) => {let _ = msg.reply(&ctx, rsp).await; return},
                Err(rsp) => {let _ = msg.reply(&ctx, rsp).await; return},
            }
        }

        else if msg.content.starts_with("w!unverify") {
            // get arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 2 {msg.reply(&ctx, "Command requires 1 argument\nUsage: w!unverify [discord-ping]").await.unwrap(); return;}
            let dc_user = match DcUsername::try_from_pingid(args[1]) {
                Some(obj) => obj,
                None => {let _ = msg.reply(&ctx, "Please ping the user or format as <@id> to ensure it gets the right person.").await; return;}
            };
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            let resp = unverify(&ctx, guild_id, dc_user).await; // Commit
            match resp {
                Ok(rsp) => {let _ = msg.reply(&ctx, rsp).await; return},
                Err(rsp) => {let _ = msg.reply(&ctx, rsp).await; return},
            }
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
            if args.len() < 2 {msg.reply(&ctx, "Command requires 1 argument\nUsage: w!guildname [guild-name]").await.unwrap(); return;}

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

        else if msg.content.starts_with("w!notifchannel") {
            // Get and check arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 2 {msg.reply(&ctx, "Command requires 1 argument\nUsage: w!notifchannel [channel-id]").await.unwrap(); return;}

            // Check if it's a proper ID
            let notif_channelid = match args[1].parse::<i64>() {Ok(id) => {id}, Err(_) => {0}};
            if notif_channelid == 0 {msg.reply(&ctx, "Invalid channel ID").await.unwrap(); return}

            // Get guild id
            let guild_id: i64 = match msg.guild_id {
                Some(id) => {id.into()},
                None => {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            };

            // Push current members so it doesn't spam on the first check (directly stolen from the check lol)
            let guildname = match get_guild_config(guild_id, "wynn-guild-name") { Some(name) => {name}, None => {msg.reply(&ctx, "Your guild name hasn't been set or is incorrect").await.unwrap(); return}}; 
            if !is_real_guild(&guildname).await {return}
            let box_wynnguildmembers = get_guild_members(&guildname).await;
            let mut wg_members: Vec<String> = vec![];
            for name in box_wynnguildmembers.clone() {wg_members.push(name[1].to_string())}
            let mut formatted_members = String::new();
            for name in &wg_members {formatted_members.push_str(&format!("{}\n", name));}
            push_guild_config(guild_id, "wynn-guild-members", formatted_members);

            // Commit and reply
            push_guild_config(guild_id, "notif-channel-id", notif_channelid.to_string());
            msg.reply(&ctx, "Channel has been saved").await.unwrap();
        }

        else if msg.content.starts_with("w!vetrole") {
            // Get and check arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 2 {msg.reply(&ctx, "Command requires 1 argument\nUsage: w!vetrole [role-id]").await.unwrap(); return;}

            // Format the provided (str) id to a proper integer
            let role_id = match args[1].parse::<u64>() {Ok(id) => {id}, Err(_) => {0}};
            if role_id == 0 {msg.reply(&ctx, "The ID you provided couldn't be parsed (probably incorrect)").await.unwrap(); return;}

            // Push the role id to the config file 
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            push_guild_config(guild_id, "veteran-role-id", role_id.to_string());

            // Answer
            msg.reply(&ctx, "Veteran role has been saved").await.unwrap();
        }

        else if msg.content.starts_with("w!removedroles") {
            // TODO: let people clear configs by not passing args
            // Get args and misc (different from usual since we don't know how many there are)
            let args = match msg.content.strip_prefix("w!removedroles ") {None => {"".to_string()}, Some(args) => {args.to_string()}};
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please.").await.unwrap(); return;}
            
            // We won't check them here, just do it in the removing section
            // (commit and push)
            push_guild_config(guild_id, "removed-roles", args);
            msg.reply(&ctx, "Roles have been saved").await.unwrap();
        }

        /* if msg.content.starts_with("w!debug_runupdate") {
            println!("DEBUG: Running forced update");
            run_wynndc_update().await
        } */ // Command to force run update (disabled obv)
    }

    async fn ready(&self, _: Context, ready: Ready) {
        // Inform readiness
        println!("{} is connected!", ready.user.name);

        // Start scheduler for updates (do an update every hour) 
        // Tbh I barely understand what's going on here
        let mut interval_timer = tokio::time::interval(chrono::Duration::hours(1).to_std().unwrap());
        tokio::spawn( async move { loop {
            interval_timer.tick().await;
            tokio::spawn( async move {
                println!("STATUS: Running updater");
                let cache = Http::new(&std::fs::read_to_string("token.txt").unwrap());
                run_wynndc_update(cache).await; // ^ // TODO: Figure out a solution that doesn't re-read token 
                println!("STATUS: Hourly update done!") // ^ Shouldn't be too hard tbh
                }
            );
        }}
        );
    }
}

#[tokio::main]
async fn main() {
    // Load token from file
    let token = std::fs::read_to_string("token.txt").expect("Unable to read token file");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client =
        Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
