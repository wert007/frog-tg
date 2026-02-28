use std::sync::{Arc, Mutex};

use anyhow::Context;
use chrono::Local;
use teloxide::{
    dispatching::dialogue::{GetChatId, InMemStorage},
    prelude::*,
    types::InputPollOption,
};

use crate::weather::{BotWeatherExt, WeatherStats};

mod weather;
const TOKEN: &'static str = include_str!("../token.txt").trim_ascii();
type DialogueState = Dialogue<State, InMemStorage<State>>;

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

#[derive(Debug, Default, Clone)]
pub enum State {
    #[default]
    Start,
    WalkStarted {
        walk: CompleteWalk,
        id: ChatId,
    },
    End,
}

impl State {
    async fn start(bot: Bot, dialoge: DialogueState, msg: Message) -> anyhow::Result<()> {
        let walk = CompleteWalk::start()
            .await
            .context("Creating walk for new walk created by user")?;
        bot.send_weather_stats(msg.chat.id, walk.weather)
            .await
            .context("Sending the weather via tg to user")?;
        dialoge
            .update(State::WalkStarted {
                walk,
                id: msg.chat.id,
            })
            .await?;
        bot.send_poll(msg.chat.id,
        "Amazing, your walk has been started. When something happens, select one of these options to continue or finish your walk.",
        ["Found Something", "Erdkröte", "Grasfrosch", "Teichmolch", "Bergmolch", "Kammmolch", "End"].map(InputPollOption::new))
        .await
        .context("Sending possible next steps via tg poll to user")?;
        Ok(())
    }

    async fn poll_answer_walk_started(
        bot: Bot,
        (mut walk, id): (CompleteWalk, ChatId),
        dialoge: DialogueState,
    ) -> anyhow::Result<()> {
        let date = Local::now();
        let path = format!("walks/{}.json", date.format("%Y-%m-%d"));

        walk.end = Some(date);
        _ = walk.weather.ending().await;

        serde_json::to_writer(
            std::fs::File::create(path).context("Recreating file for current walk")?,
            &walk,
        )
        .context("Writing new walk to freshly created walk")?;

        bot.send_message(
            id,
            format!(
                "You finished your walk. You've been at it for {}.",
                (date - walk.start)
            ),
        )
        .await?;
        dialoge.update(State::Start).await?;
        Ok(())
    }
}

#[derive(Clone)]
#[allow(unused)]
struct UpdateWithSuppliedChatId(Update, ChatId);

impl GetChatId for UpdateWithSuppliedChatId {
    fn chat_id(&self) -> Option<ChatId> {
        Some(self.1)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bot = Bot::new(TOKEN);
    let schema = dptree::entry()
        .map(|u: Update, m: Arc<Mutex<ChatId>>| {
            let i = u.chat().map(|c| c.id).unwrap_or(*m.lock().unwrap());
            *m.lock().unwrap() = i;
            UpdateWithSuppliedChatId(u, i)
        })
        .enter_dialogue::<UpdateWithSuppliedChatId, InMemStorage<State>, State>()
        .branch(
            Update::filter_message()
                .filter_map(|u: Update| u.from().cloned())
                .branch(dptree::case![State::Start].endpoint(State::start)),
        )
        .branch(
            Update::filter_poll().branch(
                dptree::case![State::WalkStarted { walk, id }]
                    .filter(|p: Poll| {
                        p.options
                            .iter()
                            .find(|o| o.text == "End")
                            .is_some_and(|o| o.voter_count > 0)
                    })
                    .endpoint(State::poll_answer_walk_started),
            ),
        );

    Dispatcher::builder(bot, schema)
        .enable_ctrlc_handler()
        .error_handler(Arc::new(error_handler))
        .dependencies(dptree::deps![
            InMemStorage::<State>::new(),
            Arc::new(Mutex::<ChatId>::new(ChatId(0)))
        ])
        .build()
        .dispatch()
        .await;
    Ok(())
}

async fn error_handler<E: std::fmt::Debug + Send + Sync + 'static>(e: E) {
    eprintln!("[error] {e:?}");
}
