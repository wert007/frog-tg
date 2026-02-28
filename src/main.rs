use std::sync::Arc;

use anyhow::Context;
use chrono::Local;
use teloxide::{
    prelude::*,
    types::{InputPollOption, User},
};

use crate::weather::{BotWeatherExt, WeatherStats};

mod weather;
const TOKEN: &'static str = include_str!("../token.txt").trim_ascii();

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum FrogFound {
    Partial,
    Complete,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CompleteWalk {
    start: chrono::DateTime<Local>,
    end: Option<chrono::DateTime<Local>>,
    weather: WeatherStats,
    frogs: Vec<FrogFound>,
}
impl CompleteWalk {
    async fn start() -> anyhow::Result<Self> {
        Ok(Self {
            start: Local::now(),
            end: None,
            weather: WeatherStats::current()
                .await
                .context("Starting a new walk")?,
            frogs: Vec::new(),
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bot = Bot::new(TOKEN);
    let schema = Update::filter_message()
        .filter_map(|u: Update| u.from().cloned())
        .branch(Message::filter_text().endpoint(process_text_message));

    Dispatcher::builder(bot, schema)
        .enable_ctrlc_handler()
        .error_handler(Arc::new(error_handler))
        .build()
        .dispatch()
        .await;
    Ok(())
}

async fn error_handler<E: std::fmt::Debug + Send + Sync + 'static>(e: E) {
    eprintln!("[error] {e:?}");
}

async fn process_text_message(bot: Bot, user: User, message_text: String) -> anyhow::Result<()> {
    if message_text.starts_with("/start") {
        start_new_walk(bot, user).await
    } else {
        let date = chrono::Local::now();
        let path = format!("walks/{}.json", date.format("%Y-%m-%d"));
        let file_content = match std::fs::read_to_string(&path) {
            Ok(it) => it,
            Err(err) => {
                bot.send_message(user.id, format!("If you want to start a new walk type /start. Otherwise something went wrong and I have more details here: ```\nCould not open file {path}.\n\nError: {err}```")).await?;
                return Ok(());
            }
        };
        let mut current_walk: CompleteWalk = match serde_json::from_str(&file_content) {
            Ok(it) => it,
            Err(err) => {
                bot.send_message(user.id, format!("Oops. There are some errors in your current walk. Something went wrong on my end. Here are more details: ```\nError: {err}```")).await?;
                return Ok(());
            }
        };
        handle_active_walk(bot, user, message_text, &mut current_walk).await
    }
}

async fn handle_active_walk(
    bot: Bot,
    user: User,
    message_text: String,
    current_walk: &mut CompleteWalk,
) -> Result<(), anyhow::Error> {
    Ok(())
}

async fn start_new_walk(bot: Bot, user: User) -> anyhow::Result<()> {
    let walk = CompleteWalk::start()
        .await
        .context("Creating walk for new walk created by user")?;
    let path = format!("walks/{}.json", walk.start.format("%Y-%m-%d"));
    serde_json::to_writer(
        std::fs::File::create(path).context("Creating file for new walk")?,
        &walk,
    )
    .context("Writing new walk to freshly created walk")?;
    bot.send_weather_stats(user.id, walk.weather)
        .await
        .context("Sending the weather via tg to user")?;
    // Send weather stats, once we have those!
    bot.send_poll(user.id,
        "Amazing, your walk has been started. When something happens, select one of these options to continue or finish your walk.",
        ["Found Something", "Erdkröte", "Grasfrosch", "Teichmolch", "Bergmolch", "Kammmolch", "End"].map(InputPollOption::new))
        .await
        .context("Sending possible next steps via tg poll to user")?;
    Ok(())
}
