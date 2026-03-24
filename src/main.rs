use parking_lot::Mutex;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use chrono::{DateTime, Local};
use teloxide::{dispatching::dialogue::InMemStorage, dptree::entry, prelude::*};

use crate::{
    notes::Note,
    polls::MainQuestion,
    questionaire::QuestionaireFrogName,
    state::State,
    utils::{
        DialogueState, LastLocation, MessageClassification, Mode, SentMessage, TimedLocation,
        UpdateWithSuppliedChatId, if_is_command,
    },
    weather::WeatherStats,
};

mod callback_answered;
mod commands;
mod counting;
mod end_walk;
mod notes;
mod polls;
mod questionaire;
mod reports;
mod state;
mod utils;
mod weather;

const TOKEN: &'static str = include_str!("../token.txt").trim_ascii();

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Sex {
    Male,
    Female,
    Unknown,
}

impl Sex {
    fn as_emoji(&self) -> char {
        match self {
            Sex::Male => '♂',
            Sex::Female => '♀',
            Sex::Unknown => '?',
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FrogFound {
    name: String,
    sex: Sex,
    location: usize,
    towards: bool,
    time: DateTime<Local>,
    gps_location: Option<TimedLocation>,
}
impl FrogFound {
    fn to_message(&self) -> String {
        format!("{} ({})", self.name, self.sex.as_emoji())
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DeadFrog {
    name: Option<String>,
    sex: Option<Sex>,
    location: usize,
    time: DateTime<Local>,
}

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct CompleteWalk {
    start: chrono::DateTime<Local>,
    end: Option<chrono::DateTime<Local>>,
    weather: WeatherStats,
    frogs: Vec<FrogFound>,
    notes: Vec<Note>,
    dead_frogs: Vec<DeadFrog>,
    repeats: usize,
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
            notes: Vec::new(),
            repeats: 0,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct PartialFrog {
    name: String,
    sex: Option<Sex>,
    location: Option<usize>,
    towards: Option<bool>,
    gps_location: Option<TimedLocation>,
}
impl PartialFrog {
    fn build(self) -> anyhow::Result<FrogFound> {
        Ok(FrogFound {
            name: self.name,
            sex: self.sex.ok_or(anyhow!("Sex is needed"))?,
            location: self.location.ok_or(anyhow!("Location is needed"))?,
            towards: self
                .towards
                .ok_or(anyhow!("Towards or Backwards is needed"))?,
            gps_location: self.gps_location,
            time: Local::now(),
        })
    }

    fn go_back(&self) -> Option<PartialFrog> {
        if self.towards.is_some() {
            Some(Self {
                towards: None,
                ..self.clone()
            })
        } else if self.location.is_some() {
            Some(Self {
                location: None,
                ..self.clone()
            })
        } else if self.sex.is_some() {
            Some(Self {
                sex: None,
                ..self.clone()
            })
        } else {
            None
        }
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let f = std::fs::File::create("lock.txt")?;
    f.lock()?;
    let bot = Bot::new(TOKEN);
    println!("Bot started");
    let text_message_handler = Update::filter_message()
        .filter_map(|u: Update| u.from().cloned())
        .branch(
            dptree::filter_map(|m: Message, s: State| {
                m.text()
                    .is_some_and(|t| t.trim() == "/find")
                    .then(|| s.as_walk())
                    .flatten()
            })
            .endpoint(async |bot, dialoge: DialogueState, sent: SentMessage| {
                sent.clear_history();
                sent.add_to_history(
                    MainQuestion::FoundSomething
                        .ask(bot, dialoge.chat_id())
                        .await?,
                );
                Ok(())
            }),
        )
        .branch(if_is_command("report", commands::report))
        .endpoint(State::text_message);
    let poll_answered_handler = Update::filter_poll().endpoint(polls::poll_answered);
    let schema = dptree::entry()
        .map(UpdateWithSuppliedChatId::ensure_id)
        .enter_dialogue::<UpdateWithSuppliedChatId, InMemStorage<State>, State>()
        .branch(text_message_handler)
        .branch(
            Update::filter_callback_query()
                .filter_map(|s: State| s.as_walk())
                .endpoint(callback_answered::inline_keyboard_button_pressed),
        )
        .branch(poll_answered_handler)
        .branch(LastLocation::update_handler());

    Dispatcher::builder(bot, schema)
        .enable_ctrlc_handler()
        .error_handler(Arc::new(error_handler))
        .dependencies(dptree::deps![
            InMemStorage::<State>::new(),
            Arc::new(Mutex::<ChatId>::new(ChatId(0))),
            Mode::create_debug(),
            LastLocation::default(),
            SentMessage::default()
        ])
        .build()
        .dispatch()
        .await;
    drop(f);
    Ok(())
}

async fn error_handler<E: std::fmt::Debug + Send + Sync + 'static>(e: E) {
    eprintln!("[error] {e:?}");
}
