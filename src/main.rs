use parking_lot::Mutex;
use std::sync::Arc;

use anyhow::{Context, anyhow, bail};
use chrono::{DateTime, Local};
use teloxide::{
    dispatching::dialogue::{GetChatId, InMemStorage},
    payloads::SetMessageReactionSetters,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile, InputPollOption, ReactionType},
};

use crate::{
    polls::MainQuestion,
    questionaire::QuestionaireFrogName,
    state::State,
    utils::{DialogueState, LastLocation, MessageClassification, Mode, SentMessage, TimedLocation},
    weather::WeatherStats,
};

mod counting;
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
pub struct Note {
    text: String,
    location: usize,
    source: MessageClassification,
    time: DateTime<Local>,
    gps_location: Option<TimedLocation>,
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

    fn create_note(
        &mut self,
        text: String,
        gps_location: LastLocation,
        source: MessageClassification,
    ) {
        let location = match source {
            MessageClassification::None => 2,
            MessageClassification::Weather => 2,
            MessageClassification::Frog(i) => self.frogs[i].location,
            MessageClassification::DeadFrog(i) => self.dead_frogs[i].location,
        };
        self.notes.push(Note {
            text,
            location,
            source,
            time: Local::now(),
            gps_location: gps_location.as_location(),
        });
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

async fn inline_keyboard_button_pressed(
    bot: Bot,
    dialoge: DialogueState,
    cb: CallbackQuery,
    last_location: LastLocation,
    mode: Mode,
    sent: SentMessage,
) -> anyhow::Result<()> {
    let mut state = dialoge.get_or_default().await?;
    let walk = state.as_walk_mut().unwrap();
    let weather = &mut walk.weather;
    let before = *weather;
    let mut is_repeat = false;
    match cb.data.as_ref().map(|s| s.as_str()) {
        Some("end:debug") => {
            bot.answer_callback_query(cb.id).await?;
            let walk = std::mem::take(walk);
            end_walk(bot, walk, dialoge, Mode::create_debug()).await?;
            return Ok(());
        }
        Some("end:switch") => {
            bot.answer_callback_query(cb.id).await?;
            let walk = std::mem::take(walk);
            end_walk(bot, walk, dialoge, Mode::create_release()).await?;
            return Ok(());
        }
        Some("found:repeat") => {
            let mut frog = walk.frogs.last().unwrap().clone();
            frog.time = Local::now();
            frog.gps_location = last_location.as_location();
            walk.frogs.push(frog);
            is_repeat = true;
        }
        Some("found:next") => {
            bot.answer_callback_query(cb.id).await?;
            sent.clear_history();
            sent.add_to_history(
                MainQuestion::FoundSomething
                    .ask(bot, dialoge.chat_id())
                    .await?,
            );
            return Ok(());
        }
        Some("found:end") => {
            bot.answer_callback_query(cb.id).await?;
            let walk = std::mem::take(walk);
            maybe_end_walk(bot, walk, dialoge, mode, sent).await?;
            return Ok(());
        }
        Some("weather:wind-0") => {
            weather.wind_beaufort = weather::Beaufort::Zero;
        }
        Some("weather:wind-minus") => {
            weather.wind_beaufort = weather.wind_beaufort.decrease();
        }
        Some("weather:wind-plus") => {
            weather.wind_beaufort = weather.wind_beaufort.increase();
        }
        Some("weather:wind-6") => {
            weather.wind_beaufort = weather::Beaufort::Six;
        }
        Some("weather:clouds-0") => {
            weather.cloudiness = weather::Cloudiness::Clear;
        }
        Some("weather:clouds-less") => {
            weather.cloudiness = weather.cloudiness.decrease();
        }
        Some("weather:clouds-more") => {
            weather.cloudiness = weather.cloudiness.increase();
        }
        Some("weather:clouds-100") => {
            weather.cloudiness = weather::Cloudiness::AllClouds;
        }
        Some("weather:ground-wet") => {
            weather.ground_humidity = weather::GroundHumidity::Wet;
        }
        Some("weather:ground-humid") => {
            weather.ground_humidity = weather::GroundHumidity::Humid;
        }
        Some("weather:ground-dry") => {
            weather.ground_humidity = weather::GroundHumidity::Dry;
        }
        Some("weather:ground-very-dry") => {
            weather.ground_humidity = weather::GroundHumidity::VeryDry;
        }
        Some("weather:temperature-start-change") => {
            dialoge
                .update(State::EnterTemperature {
                    is_start: true,
                    prev_state: Box::new(state),
                })
                .await?;
            let id = bot
                .send_message(dialoge.chat_id(), "Enter now your starting temperature:")
                .await?
                .id;
            sent.add_weather(id);
            bot.answer_callback_query(cb.id).await?;
            return Ok(());
        }
        Some("weather:temperature-end-change") => {
            dialoge
                .update(State::EnterTemperature {
                    is_start: false,
                    prev_state: Box::new(state),
                })
                .await?;
            let id = bot
                .send_message(dialoge.chat_id(), "Enter now your ending temperature:")
                .await?
                .id;
            sent.add_weather(id);
            bot.answer_callback_query(cb.id).await?;
            return Ok(());
        }
        Some("weather:percipation-change") => {
            dialoge
                .update(State::ChangePercipation {
                    prev_state: Box::new(state),
                })
                .await?;
            let id = bot
                .send_poll(
                    dialoge.chat_id(),
                    "Select the current percipation:",
                    [
                        weather::Percipation::None,
                        weather::Percipation::Fog,
                        weather::Percipation::Drizzle,
                        weather::Percipation::ModerateRain,
                        weather::Percipation::StrongRain,
                        weather::Percipation::Graupel,
                        weather::Percipation::Snow,
                    ]
                    .map(|e| InputPollOption::new(e.to_string())),
                )
                .await?
                .id;
            sent.add_weather(id);
            bot.answer_callback_query(cb.id).await?;
            return Ok(());
        }
        None => todo!(),
        _ => bail!("TODO"),
    }
    let message_id = cb.message.unwrap().id();
    bot.answer_callback_query(cb.id).await?;

    if before != *weather {
        let m = bot.edit_message_text(dialoge.chat_id(), message_id, weather.as_message());
        m.reply_markup(WeatherStats::default_weather_keyboard_markup())
            .await?;
        dialoge.update(state).await?;
    } else if is_repeat {
        walk.repeats += 1;
        bot.edit_message_text(
            dialoge.chat_id(),
            message_id,
            format!(
                "Found {} x {} ",
                walk.repeats,
                walk.frogs.last().unwrap().to_message(),
            ),
        )
        .reply_markup(InlineKeyboardMarkup::new([
            [InlineKeyboardButton::callback("Repeat", "found:repeat")],
            [InlineKeyboardButton::callback("Find", "found:next")],
            [InlineKeyboardButton::callback("End", "found:end")],
        ]))
        .await?;
        dialoge.update(state).await?;
    }
    Ok(())
}

async fn maybe_end_walk(
    bot: Bot,
    walk: CompleteWalk,
    dialoge: DialogueState,
    mode: Mode,
    sent: SentMessage,
) -> Result<(), anyhow::Error> {
    if mode.is_debug() {
        let m = bot
            .send_message(
                dialoge.chat_id(),
                "You are in debug mode. If this is a real walk switch now.",
            )
            .reply_markup(
                InlineKeyboardMarkup::default()
                    .append_row([InlineKeyboardButton::callback("Switch", "end:switch")])
                    .append_row([InlineKeyboardButton::callback("Debug", "end:debug")]),
            )
            .await?;
        sent.add_to_history(m.id);
        dialoge.update(State::WaitForModeChange { walk }).await?;
        Ok(())
    } else {
        end_walk(bot, walk, dialoge, mode).await
    }
}

async fn end_walk(
    bot: Bot,
    mut walk: CompleteWalk,
    dialoge: DialogueState,
    mode: Mode,
) -> Result<(), anyhow::Error> {
    let date = Local::now();
    let existing = glob::glob(&format!(
        "{}/{}*.json",
        mode.as_path(),
        date.format("%Y-%m-%d")
    ))?;
    let index = existing.count();
    let path = format!(
        "{}/{}({}).json",
        mode.as_path(),
        date.format("%Y-%m-%d"),
        index + 1
    );
    walk.end = Some(date);
    _ = walk.weather.ending(mode.is_debug()).await;
    serde_json::to_writer(
        std::fs::File::create_new(&path).context("Recreating file for current walk")?,
        &walk,
    )
    .context("Writing new walk to freshly created walk")?;
    let inline_report = reports::create_inline_end_walk_report(&walk);
    let duration = date - walk.start;
    bot.send_message(
        dialoge.chat_id(),
        format!(
            "You finished your walk. You've been at it for {}:{:02} h. {inline_report}\n\nWhenever you want to /start a new walk, I'm ready.",
            duration.num_hours(),
            duration.num_minutes() % 60,
        ),
    )
    .await?;

    send_pdf_report_to_bot(bot, dialoge.chat_id(), &path, &walk).await?;

    dialoge.update(State::Start).await?;
    Ok(())
}

async fn send_pdf_report_to_bot(
    bot: Bot,
    chat_id: ChatId,
    path: &str,
    walk: &CompleteWalk,
) -> anyhow::Result<()> {
    let img = reports::create_image_report(walk)?;
    std::fs::write(std::path::Path::new(path).with_extension("png"), &img)?;
    let f =
        InputFile::memory(img).file_name(format!("report-{}.png", walk.start.format("%d.%m.%Y")));
    bot.send_photo(chat_id, f).await?;
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

fn add_note(
    bot: Bot,
    dialoge: DialogueState,
    message: Message,
    text: String,
    location: LastLocation,
    sent: SentMessage,
) -> impl Future<Output = anyhow::Result<()>> {
    async move {
        let mut s = dialoge.get().await?.ok_or(anyhow!(
            "This can only be executed if a walk has been started???"
        ))?;
        let classification = message
            .reply_to_message()
            .map(|m| sent.get(m.id))
            .flatten()
            .unwrap_or_default();
        s.as_walk_mut()
            .ok_or(anyhow!("There should be a walk at this point!"))?
            .create_note(text, location, classification);
        dialoge.update(s).await?;
        bot.set_message_reaction(dialoge.chat_id(), message.id)
            .reaction(vec![ReactionType::Emoji {
                emoji: "✍".into()
            }])
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let f = std::fs::File::create("lock.txt")?;
    f.lock()?;
    let bot = Bot::new(TOKEN);
    println!("Bot started");
    let schema = dptree::entry()
        .map(|u: Update, m: Arc<Mutex<ChatId>>| {
            let i = u.chat().map(|c| c.id).unwrap_or(*m.lock());
            *m.lock() = i;
            UpdateWithSuppliedChatId(u, i)
        })
        .enter_dialogue::<UpdateWithSuppliedChatId, InMemStorage<State>, State>()
        .branch(
            Update::filter_message()
                .filter_map(|u: Update| u.from().cloned())
                .branch(
                    dptree::filter_map(|m: Message, s: State| {
                        m.text()
                            .is_some_and(|t| t.trim() == "/find")
                            .then(|| s.as_walk())
                            .flatten()
                    })
                    .endpoint(
                        async |bot, dialoge: DialogueState, sent: SentMessage| {
                            sent.clear_history();
                            sent.add_to_history(
                                MainQuestion::FoundSomething
                                    .ask(bot, dialoge.chat_id())
                                    .await?,
                            );
                            Ok(())
                        },
                    ),
                )
                .branch(
                    dptree::filter(|m: Message| m.text().is_some_and(|t| t.trim() == "/realfrogs"))
                        .endpoint(async |m: Mode| {
                            m.change_to_release();
                            anyhow::Ok(())
                        }),
                )
                .branch(
                    dptree::filter(|m: Message| m.text().is_some_and(|t| t.trim() == "/debug"))
                        .endpoint(async |m: Mode| {
                            m.change_to_debug();
                            anyhow::Ok(())
                        }),
                )
                .branch(
                    dptree::filter(|m: Message| m.text().is_some_and(|t| t.trim() == "/report"))
                        .endpoint(async |bot: Bot, dialogue: DialogueState| {
                            let text = match dialogue.get_or_default().await?.as_walk() {
                                Some(w) if w.frogs.is_empty() => {
                                    "Nothing has been found yet.".into()
                                }
                                Some(w) => reports::create_inline_end_walk_report(&w),
                                _ => "Use command /start to start a new walk first.".into(),
                            };

                            bot.send_message(dialogue.chat_id(), text).await?;
                            anyhow::Ok(())
                        }),
                )
                .branch(
                    dptree::case![State::Start]
                        .filter(|m: Message| m.text().is_some_and(|t| t.trim() == "/start"))
                        .endpoint(State::start),
                )
                .branch(
                    dptree::case![State::EnterTemperature {
                        is_start,
                        prev_state
                    }]
                    .endpoint(State::enter_temperature),
                )
                .filter_map(|m: Message| m.text().map(ToString::to_string))
                .endpoint(add_note),
        )
        .branch(
            Update::filter_callback_query()
                .filter_map(|s: State| s.as_walk())
                .endpoint(inline_keyboard_button_pressed),
        )
        .branch(
            Update::filter_poll()
                .branch(
                    dptree::case![State::ChangePercipation { prev_state }]
                        .endpoint(State::change_percipation),
                )
                .branch(
                    dptree::case![State::WalkStarted { walk }]
                        .endpoint(State::poll_answer_walk_started),
                )
                .branch(
                    dptree::case![State::WaitForModeChange { walk }]
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
                    dptree::case![State::QuestionaireSex { walk, questionaire }]
                        .endpoint(questionaire::found_sex),
                )
                .branch(
                    dptree::case![State::FrogIdentified { frog, walk }]
                        .endpoint(State::frog_identified),
                ),
        )
        .branch(LastLocation::update_handler());

    Dispatcher::builder(bot, schema)
        .enable_ctrlc_handler()
        .error_handler(Arc::new(error_handler))
        .dependencies(dptree::deps![
            InMemStorage::<State>::new(),
            Arc::new(Mutex::<ChatId>::new(ChatId(0))),
            Mode::create_debug(),
            Arc::new(Mutex::new(TimedLocation::error())),
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
