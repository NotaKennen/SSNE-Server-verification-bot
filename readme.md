# SSNE Server verification bot
A fairly basic Discord bot custom made for the SSNE Wynncraft guild (although it works for all guilds). Its purpose is to function as an easy way to verify people so they can interact in and/or see the member channels. It mostly manages everything automatically, but there's still some need for manual intervention, mainly just running a single command whenever a person joins the Discord. This command then adds them to the member list, and they can then be searched for and managed through the bot.

## Usage
If you want to start using the bot, you can simply add it to your server. Without any need for setup, it can keep track of your verified members. However, to enable some of the other features, you'll need to do some commands.

To enable role management, do __w!verifiedrole (role-id)__, where *(role-id)* is the ID of the role you want to give to verified people. The bot will now give this role to anyone who is verified through it. Note that it'll NOT give the role to people who have been verified before.

To enable automatic member management, do __w!guildname (guild-name)__, where *(guild-name)* is your 4 character Wynncraft guild ID (seen on territory maps etc). Now the bot will automatically unverify anyone who isn't in the guild. It might take up to an hour or so for the unverification to happen, since we don't want to spam the Wynncraft API.

To enable join notifications, do __w!notifchannel (channel-id)__, where *(channel-id)* is the channel where you want your notifications in. The bot will then send a message there when someone joins. You do need to set the guildname (w!guildname) for it to work though.

To enable the "veteran role", or the role given to unverified people, do __w!vetrole (role-id)__, where *(role-id)* is the role given. Then the bot will give a role to whoever happens to be unverified, either through the automatic member management or a manual w!unverify command.

### Self hosting?
If you want to host the bot yourself, you can clone the git repo, make a file called token.txt, put your bot token in there, and then run the Rust project ("cargo run"). Then you should have a local instance of the bot that you can add to your server. 

## Features
As for features, it can:
- (Semi-)Automatically verify people and give them roles
- Keeps track of names and can search people based on their Discord or Minecraft name
- Automatically removes people who leave / are kicked from the Wynncraft guild
- Notifies you of new members that join your guild
- Give managed access to verification to guild staff without the need of extra permissions

Planned features:
- Automatically update members when they change Minecraft usernames
- Fun statistics :D
- More configuration on what the bot should/shouldn't do
- Activity tracking
- (Maybe?) Fully automatic verification (without the need of admins/managers)
- (Backend) A better DB system