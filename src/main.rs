use std::sync::{Arc, Mutex};

use anyhow::{Context, bail};
use chrono::{DateTime, Local};
use teloxide::{
    dispatching::dialogue::{GetChatId, InMemStorage},
    prelude::*,
    types::InputPollOption,
};

use crate::{
    questionaire::QuestionaireFrogName,
    weather::{BotWeatherExt, WeatherStats},
};

mod questionaire;
mod weather;

const TOKEN: &'static str = include_str!("../token.txt").trim_ascii();
type DialogueState = Dialogue<State, InMemStorage<State>>;

trait PollExt {
    fn selected(&self) -> &str;
    fn selected_index(&self) -> isize;
}

impl PollExt for Poll {
    fn selected(&self) -> &str {
        self.options
            .iter()
            .filter(|o| o.voter_count > 0)
            .map(|o| o.text.as_str())
            .next()
            .unwrap_or_default()
    }

    fn selected_index(&self) -> isize {
        self.options
            .iter()
            .enumerate()
            .filter(|(_, o)| o.voter_count > 0)
            .map(|(i, _)| i as isize)
            .next()
            .unwrap_or(-1)
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Sex {
    Male,
    Female,
    Unknown,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FrogFound {
    name: String,
    sex: Sex,
    location: usize,
    towards: bool,
    time: DateTime<Local>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DeadFrog {
    name: Option<String>,
    sex: Option<Sex>,
    location: usize,
    time: DateTime<Local>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CompleteWalk {
    start: chrono::DateTime<Local>,
    end: Option<chrono::DateTime<Local>>,
    weather: WeatherStats,
    frogs: Vec<FrogFound>,
    dead_frogs: Vec<DeadFrog>,
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
            dead_frogs: Vec::new(),
        })
    }
}

#[derive(Debug, Default, Clone)]
pub enum State {
    #[default]
    Start,
    WalkStarted {
        walk: CompleteWalk,
    },
    QuestionaireFrogName {
        walk: CompleteWalk,
        questionaire: questionaire::QuestionaireFrogName,
    },
    DeadFrog {
        walk: CompleteWalk,
    },
    DeadFrogName {
        walk: CompleteWalk,
        name: Option<String>,
    },
    FrogIdentified {
        name: String,
        walk: CompleteWalk,
    },
    FrogIdentifiedSex {
        name: String,
        walk: CompleteWalk,
        sex: Sex,
    },
    FrogIdentifiedSexLocation {
        name: String,
        walk: CompleteWalk,
        sex: Sex,
        location: usize,
    },
    End,
}

impl State {
    async fn start(bot: Bot, dialoge: DialogueState) -> anyhow::Result<()> {
        let walk = CompleteWalk::start()
            .await
            .context("Creating walk for new walk created by user")?;
        bot.send_weather_stats(dialoge.chat_id(), walk.weather)
            .await
            .context("Sending the weather via tg to user")?;
        dialoge.update(State::WalkStarted { walk }).await?;
        found_something(bot, dialoge).await?;
        Ok(())
    }

    async fn poll_answer_walk_started(
        bot: Bot,
        walk: CompleteWalk,
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        match poll.selected() {
            "End" => {
                end_walk(bot, walk, dialoge).await?;
            }
            name @ ("Erdkröte" | "Grasfrosch" | "Teichmolch" | "Bergmolch" | "Kammmolch") => {
                dialoge
                    .update(State::FrogIdentified {
                        name: name.into(),
                        walk,
                    })
                    .await?;
                ask_sex(bot, name, dialoge.chat_id()).await?;
            }
            "Found Something" => {
                dialoge
                    .update(State::QuestionaireFrogName {
                        walk,
                        questionaire: QuestionaireFrogName::default(),
                    })
                    .await?;
                questionaire::start(bot, dialoge).await?;
            }
            "Dead Frog :(" => {
                bot.send_poll(
                    dialoge.chat_id(),
                    "Can you still recognize what it was?",
                    [
                        "No",
                        "Erdkröte",
                        "Grasfrosch",
                        "Teichmolch",
                        "Bergmolch",
                        "Kammmolch",
                    ]
                    .map(InputPollOption::new),
                )
                .await?;
                dialoge.update(State::DeadFrog { walk }).await?;
            }
            _ => bail!("TODO"),
        }
        Ok(())
    }

    async fn dead_frog_answered(
        bot: Bot,
        walk: CompleteWalk,
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        let name = match poll.selected() {
            "No" => None,
            name => Some(name.to_string()),
        };
        dialoge.update(State::DeadFrogName { walk, name }).await?;
        let locations: Vec<InputPollOption> = include_str!("../locations.txt")
            .lines()
            .filter(|l| !l.is_empty())
            .map(InputPollOption::new)
            .collect();
        // TODO: This is probably only changing slowly and not all the time. Can
        // we easily remember the last choice?
        bot.send_poll(dialoge.chat_id(), "Where are you right now?", locations)
            .await?;
        Ok(())
    }

    async fn dead_frog_location_answered(
        bot: Bot,
        (mut walk, name): (CompleteWalk, Option<String>),
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        let location = poll.selected_index();
        if location < 0 {
            bail!("We do not really handle you unselecting something. sorry.");
        }
        let location = location as usize;
        walk.dead_frogs.push(DeadFrog {
            name,
            sex: None,
            location,
            time: Local::now(),
        });
        dialoge.update(State::WalkStarted { walk }).await?;
        found_something(bot, dialoge).await?;
        Ok(())
    }

    async fn frog_identified(
        bot: Bot,
        (name, walk): (String, CompleteWalk),
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        let sex = match poll.selected() {
            "Male" => Sex::Male,
            "Female" => Sex::Female,
            "Unknown" => Sex::Unknown,
            "Use Questionaire" => bail!("TODO"),
            _ => unreachable!(),
        };
        dialoge
            .update(State::FrogIdentifiedSex { name, walk, sex })
            .await?;
        let locations: Vec<InputPollOption> = include_str!("../locations.txt")
            .lines()
            .filter(|l| !l.is_empty())
            .map(InputPollOption::new)
            .collect();
        // TODO: This is probably only changing slowly and not all the time. Can
        // we easily remember the last choice?
        bot.send_poll(dialoge.chat_id(), "Where are you right now?", locations)
            .await?;
        Ok(())
    }
    async fn frog_identified_sex(
        bot: Bot,
        (name, walk, sex): (String, CompleteWalk, Sex),
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        let location = poll.selected_index();
        if location < 0 {
            bail!("We do not really handle you unselecting something. sorry.");
        }
        let location = location as usize;
        dialoge
            .update(State::FrogIdentifiedSexLocation {
                name,
                walk,
                sex,
                location,
            })
            .await?;
        bot.send_poll(
            dialoge.chat_id(),
            "Is the frog going towards water or away from it?",
            ["Towards water", "Back from water"].map(InputPollOption::new),
        )
        .await?;
        Ok(())
    }

    async fn frog_identified_sex_location(
        bot: Bot,
        (name, mut walk, sex, location): (String, CompleteWalk, Sex, usize),
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        let towards = poll.selected_index() == 0;
        if poll.selected_index() < 0 {
            bail!("We do not really handle you unselecting something. sorry.");
        }
        let frog = FrogFound {
            name,
            sex,
            location,
            towards,
            time: Local::now(),
        };
        walk.frogs.push(frog);
        dialoge.update(State::WalkStarted { walk }).await?;
        found_something(bot, dialoge).await?;
        Ok(())
    }
}

async fn found_something(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(dialoge.chat_id(),
    "Amazing, your walk has been started. When something happens, select one of these options to continue or finish your walk.",
    ["Found Something", "Dead Frog :(", "Erdkröte", "Grasfrosch", "Teichmolch", "Bergmolch", "Kammmolch", "End"].map(InputPollOption::new))
    .await
    .context("Sending possible next steps via tg poll to user")?;
    Ok(())
}

async fn ask_sex(bot: Bot, name: &str, chat_id: ChatId) -> anyhow::Result<()> {
    bot.send_poll(
        chat_id,
        format!("Enter the sex of your {name} now:"),
        ["Male", "Female", "Unknown", "Use Questionaire"].map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

async fn end_walk(
    bot: Bot,
    mut walk: CompleteWalk,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    let date = Local::now();
    let existing = glob::glob(&format!("walks/{}*.json", date.format("%Y-%m-%d")))?;
    let index = existing.count();
    let path = format!("walks/{}({}).json", date.format("%Y-%m-%d"), index + 1);
    walk.end = Some(date);
    _ = walk.weather.ending().await;
    serde_json::to_writer(
        std::fs::File::create_new(path).context("Recreating file for current walk")?,
        &walk,
    )
    .context("Writing new walk to freshly created walk")?;
    let duration = date - walk.start;
    bot.send_message(
        dialoge.chat_id(),
        format!(
            "You finished your walk. You've been at it for {}:{:02} h.\n\nWhenever you want to /start a new walk, I'm ready.",
            duration.num_hours(),
            duration.num_minutes(),
        ),
    )
    .await?;
    dialoge.update(State::Start).await?;
    Ok(())
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
            Update::filter_poll()
                .branch(
                    dptree::case![State::WalkStarted { walk }]
                        .endpoint(State::poll_answer_walk_started),
                )
                .branch(dptree::case![State::DeadFrog { walk }].endpoint(State::dead_frog_answered))
                .branch(
                    dptree::case![State::DeadFrogName { walk, name }]
                        .endpoint(State::dead_frog_location_answered),
                )
                .branch(
                    dptree::case![State::QuestionaireFrogName { walk, questionaire }]
                        .filter(|(_, q): (CompleteWalk, QuestionaireFrogName)| q.species.is_none())
                        .endpoint(questionaire::found_species),
                )
                .branch(
                    dptree::case![State::QuestionaireFrogName { walk, questionaire }]
                        .filter(|(_, q): (CompleteWalk, QuestionaireFrogName)| q.species.is_some())
                        .endpoint(questionaire::found_frog_name),
                )
                .branch(
                    dptree::case![State::FrogIdentified { name, walk }]
                        .endpoint(State::frog_identified),
                )
                .branch(
                    dptree::case![State::FrogIdentifiedSex { name, walk, sex }]
                        .endpoint(State::frog_identified_sex),
                )
                .branch(
                    dptree::case![State::FrogIdentifiedSexLocation {
                        name,
                        walk,
                        sex,
                        location
                    }]
                    .endpoint(State::frog_identified_sex_location),
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
