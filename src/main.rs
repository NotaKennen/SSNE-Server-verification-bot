mod io;
use io::*;

use serenity::all::{CreateEmbed, CreateMessage, EditMember, RoleId, UserId};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

static BOT_VERSION: &str = "v1.0.0";

struct Handler; 

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.content.starts_with("w!") {return} // Ignore a message if it doesn't start with the prefix so we dont waste time going through it

        if msg.content.starts_with("w!help") {
            msg.reply(&ctx, format!(
"Running bot version **{}**
Help with commands:

__w!verifiedrole [role-id]__
*Use this command to specify the role used for verified members*

__w!verify [discord-ping] [minecraft-user]__
*Verifies a person and links their MC and DC names together, also gives them the verified role. Make sure to ping the member so it surely gets the right person*

__w!whois [dc | mc] [name]__
*Gives you the mc name of someone based on their dc name, or the other way around*

__w!list__
*List all verified members*

__w!unverify [discord-ping]__
*Unverifies a person, also removes their role*

The command to verify people is only available to people with the Manage Roles permission
Commands to manage the verification system are only available to people with Administrator",
BOT_VERSION
            )).await.unwrap();
            return;
        }

        else if msg.content.starts_with("w!list") {
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please. (Can't fetch server's verified members)").await.unwrap(); return;}
            let names = get_names(guild_id);

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
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please. (Can't fetch server's verified members)").await.unwrap(); return;}
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

        // Get permission level (all that just for permissions dawg) (I am NOT solving these unwraps fuck you)
        let author_perms = msg.guild_id.unwrap().to_partial_guild(&ctx).await.unwrap().user_permissions_in(&msg.channel_id.to_channel(&ctx).await.unwrap().guild().unwrap(), &msg.member(&ctx).await.unwrap());
        if !author_perms.contains(serenity::model::Permissions::MANAGE_ROLES) {msg.reply(&ctx, "You don't have the permission to do that").await.unwrap(); return}
        // - - - MANAGE ROLES PERMISSION LEVEL - - - 

        if msg.content.starts_with("w!verify") {
            // Get and check arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 3 {msg.reply(&ctx, "Command requires 2 arguments\nUsage: w!verify [discord-ping] [minecraft-user]").await.unwrap(); return;}

            // TODO: Get an API to get minecraft UUIDs (see the note left in a certain dm)
            // Or alternatively trust names and get borked over someone changing theirs

            let dc_username = args[1];
            let mc_username = args[2];

            // Validate dc username
            if !dc_username.starts_with("<@") || !dc_username.ends_with(">") {
                msg.reply(&ctx, "Invalid Discord username, please ping the user or format the name as <@(userid)>").await.unwrap();
                return;
            }

            // Reply and commit
            let guild_id: i64 = match msg.guild_id {Some(guildid) => {guildid.into()}, None => {0}};
            if guild_id == 0 {msg.reply(&ctx, "ERROR: Guild ID wasn't found! Either not a guild, or an error happened somewhere, report to Memarios please. (Member was not verified)").await.unwrap(); return;}
            msg.reply(&ctx, format!("Verifying dc-user \"{}\" as mc-user \"{}\"", dc_username, mc_username)).await.unwrap();
            insert_name_to_db(guild_id, dc_username, mc_username);

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
            target_mem_clone.edit(&ctx, builder).await.unwrap();
        }

        else if msg.content.starts_with("w!unverify") {
            // get arguments
            let args: Vec<&str> = msg.content.split(" ").collect();
            if args.len() < 2 {msg.reply(&ctx, "Command requires 1 argument\nUsage: w!unverify [discord-ping]").await.unwrap(); return;}

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
            remove_name_from_db(guild_id, args[1]);
            msg.reply(&ctx, "User has been unverified").await.unwrap();
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
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Load token from file
    let token = std::fs::read_to_string("token.txt").expect("Unable to read token file");

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
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
