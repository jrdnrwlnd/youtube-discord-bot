use serenity::{
    model::channel::Message, 
    prelude::*,
    framework::{
        standard::{
            Args, CommandResult,
            macros::{command, group},
        },
    },
    Result as SerenityResult,
};

use songbird::input::restartable::Restartable;

#[group]
#[commands(leave, play)]
struct General;

// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

async fn is_joinable(ctx: &Context, msg: &Message) -> bool {
    let guild = msg.guild(&ctx.cache).await.expect("Could not find server");

    let channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let _connect_to = match channel_id {
        Some(_channel) => return true,
        None => {
            return false;
        }
    };
}


async fn join(ctx: &Context, msg: &Message) {
    let guild = msg.guild(&ctx.cache).await.expect("Could not find server");

    let channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

        let connect_to = match channel_id {
            Some(channel) => channel,
            None => {
                return;
            }
        };
    
    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    let _handler = manager.join(guild.id, connect_to).await;
}

async fn stop(ctx: &Context, msg: &Message) {
    let guild_id = msg.guild(&ctx.cache).await.expect("Could not find server").id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.stop();
    }
}

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if !is_joinable(&ctx, &msg).await {
        check_msg(msg.reply(ctx, "Must be in a voice channel to use this feature").await);
        return Ok(());
    }
    join(&ctx, &msg).await;
    stop(&ctx, &msg).await;

    let search = args.clone();
    let url = args.single::<String>().expect("URL or Search term not given");
    let guild_id = msg.guild(&ctx.cache).await.expect("Could not find server").id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        if !url.starts_with("http") {
            let source = match songbird::input::ytdl_search(search.message()).await {
                Ok(source) => source,
                Err(why) => {
                    println!("Err starting source: {:?}", why);
    
                    check_msg(msg.channel_id.say(&ctx.http, "Error sourcing file").await);
    
                    return Ok(());
                },
            };

            let source_url = source.metadata.source_url.expect("Could not get source URL");
            let source = match Restartable::ytdl(source_url.clone(), true).await {
                Ok(source) => source,
                Err(why) => {
                    println!("Err starting source: {:?}", why);
    
                    check_msg(msg.reply(&ctx, "Error sourcing file").await);
    
                    return Ok(());
                },
            };

            check_msg(msg.reply(&ctx, format!("Now playing: {}", &source_url)).await);
            println!("{} wants to play this link: {}", msg.author.name, &source_url);

            
            handler.enqueue_source(source.into());

            return Ok(());
        }
        else {
            let source = match Restartable::ytdl(url.clone(), true).await {
                Ok(source) => source,
                Err(why) => {
                    println!("Err starting source: {:?}", why);
    
                    check_msg(msg.reply(&ctx, "Error sourcing file").await);
    
                    return Ok(());
                },
            };
            // CLI log to make sure command isn't abused
            println!("{}+{} wants to play this link: {}", msg.author.name, msg.author.discriminator, &url);
            handler.enqueue_source(source.into());
        }        
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.expect("Could not find server").id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(msg.channel_id.say(&ctx.http, format!("Failed: {:?}", e)).await);
        }
    } 
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    check_msg(msg.reply(ctx, "You can use the `--` prefix to use this bot's commands.\n
    `--play <youtube URL> OR <youtube search query>`\n
    `--leave` when you want the bot to leave.\n")
    .await);
    
    Ok(())
}

