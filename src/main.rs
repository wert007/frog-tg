use parking_lot::Mutex;
use std::sync::{Arc, atomic::AtomicBool};

use anyhow::{Context, anyhow, bail};
use chrono::{DateTime, Local};
use teloxide::{
    dispatching::dialogue::{GetChatId, InMemStorage},
    prelude::*,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup, InputFile, InputPollOption, Location,
        MessageId, UpdateKind,
    },
};

use crate::{
    polls::{MainQuestion, QuestionaireQuestion},
    questionaire::QuestionaireFrogName,
    weather::{BotWeatherExt, WeatherStats},
};

mod counting;
mod polls;
mod questionaire;
mod reports;
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
            repeats: 0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Mode(Arc<AtomicBool>);

impl Mode {
    pub fn change_to_debug(&self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn change_to_release(&self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn as_path(&self) -> &'static str {
        if self.is_debug() {
            "debug-walks"
        } else {
            "walks"
        }
    }

    pub fn create_debug() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }
    pub fn create_release() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn is_debug(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
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
}

#[derive(Debug, Default, Clone)]
pub enum State {
    #[default]
    Start,
    WalkStarted {
        walk: CompleteWalk,
    },
    WaitForModeChange {
        walk: CompleteWalk,
    },
    QuestionaireFrogName {
        walk: CompleteWalk,
        questionaire: questionaire::QuestionaireFrogName,
    },
    QuestionaireSex {
        walk: CompleteWalk,
        questionaire: questionaire::QuestionaireSex,
    },
    DeadFrog {
        walk: CompleteWalk,
    },
    DeadFrogName {
        walk: CompleteWalk,
        name: Option<String>,
    },
    FrogIdentified {
        frog: PartialFrog,
        walk: CompleteWalk,
        last_message_id: MessageId,
    },
    EnterTemperature {
        is_start: bool,
        prev_state: Box<State>,
    },
    ChangePercipation {
        prev_state: Box<State>,
    },
}

impl State {
    fn as_walk(&self) -> Option<CompleteWalk> {
        match self {
            State::Start => None,
            State::WalkStarted { walk } | State::WaitForModeChange { walk } => Some(walk.clone()),
            State::QuestionaireFrogName { walk, .. } => Some(walk.clone()),
            State::QuestionaireSex { walk, .. } => Some(walk.clone()),
            State::DeadFrog { walk } => Some(walk.clone()),
            State::DeadFrogName { walk, .. } => Some(walk.clone()),
            State::FrogIdentified { walk, .. } => Some(walk.clone()),
            State::ChangePercipation { .. } | State::EnterTemperature { .. } => todo!(),
        }
    }

    fn as_walk_mut(&mut self) -> Option<&mut CompleteWalk> {
        match self {
            State::Start => None,
            State::WalkStarted { walk } | State::WaitForModeChange { walk } => Some(walk),
            State::QuestionaireFrogName { walk, .. } => Some(walk),
            State::QuestionaireSex { walk, .. } => Some(walk),
            State::DeadFrog { walk } => Some(walk),
            State::DeadFrogName { walk, .. } => Some(walk),
            State::FrogIdentified { walk, .. } => Some(walk),
            State::ChangePercipation { .. } | State::EnterTemperature { .. } => todo!(),
        }
    }

    async fn enter_temperature(
        bot: Bot,
        dialoge: DialogueState,
        (is_start, prev_state): (bool, Box<State>),
        message: Message,
    ) -> anyhow::Result<()> {
        let temp = message
            .text()
            .ok_or(anyhow!("There should be a text message???"))?;
        let temp: f64 = temp.parse()?;
        let mut state = *prev_state;
        let weather = &mut state
            .as_walk_mut()
            .expect("Should be set at this point?")
            .weather;
        if is_start {
            weather.temperature_start = temp;
        } else {
            weather.temperature_end = Some(temp);
        }
        bot.send_weather_stats(dialoge.chat_id(), *weather).await?;
        dialoge.update(state).await?;
        Ok(())
    }

    async fn start(bot: Bot, dialoge: DialogueState) -> anyhow::Result<()> {
        let walk = CompleteWalk::start()
            .await
            .context("Creating walk for new walk created by user")?;
        bot.send_weather_stats(dialoge.chat_id(), walk.weather)
            .await
            .context("Sending the weather via tg to user")?;
        dialoge.update(State::WalkStarted { walk }).await?;
        Ok(())
    }

    async fn change_percipation(
        bot: Bot,
        prev_state: Box<State>,
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        use weather::Percipation::*;
        let percipation = match poll.selected_index() {
            0 => None,
            1 => Fog,
            2 => Drizzle,
            3 => ModerateRain,
            4 => StrongRain,
            5 => Graupel,
            6 => Snow,
            -1 => bail!("TODO, no unselecting allowed!"),
            _ => unreachable!(),
        };
        let mut state = *prev_state;
        let weather = &mut state.as_walk_mut().expect("Should be unreachable").weather;
        weather.percipation = percipation;
        bot.send_weather_stats(dialoge.chat_id(), *weather).await?;
        dialoge.update(state).await?;
        Ok(())
    }

    async fn poll_answer_walk_started(
        bot: Bot,
        last_location: LastLocation,
        walk: CompleteWalk,
        dialoge: DialogueState,
        poll: Poll,
        mode: Mode,
    ) -> anyhow::Result<()> {
        match poll.selected() {
            "End" => {
                maybe_end_walk(bot, walk, dialoge, mode).await?;
            }
            name @ ("Erdkröte" | "Grasfrosch" | "Teichmolch" | "Bergmolch" | "Kammmolch") => {
                let last_message_id = MainQuestion::AskForSex(name.into())
                    .ask(bot, dialoge.chat_id())
                    .await?;
                dialoge
                    .update(State::FrogIdentified {
                        frog: PartialFrog {
                            name: name.into(),
                            gps_location: if_is_relevant(last_location),
                            ..Default::default()
                        },
                        walk,
                        last_message_id,
                    })
                    .await?;
            }
            "Found Something" => {
                let last_message_id = QuestionaireQuestion::IsItAFrogToadOrMolch
                    .ask(bot.clone(), dialoge.chat_id())
                    .await?;
                dialoge
                    .update(State::QuestionaireFrogName {
                        walk,
                        questionaire: QuestionaireFrogName::new(last_message_id),
                    })
                    .await?;
            }
            "Dead Frog :(" => {
                MainQuestion::FoundDeadFrog
                    .ask(bot, dialoge.chat_id())
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
        MainQuestion::WhereAreYou
            .ask(bot, dialoge.chat_id())
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
        MainQuestion::FoundSomething
            .ask(bot, dialoge.chat_id())
            .await?;
        Ok(())
    }

    async fn frog_identified(
        bot: Bot,
        (mut frog, walk, last_message_id): (PartialFrog, CompleteWalk, MessageId),
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        if poll.selected_index() < 0
            && let Some(q) = MainQuestion::find_original(&poll.question)
        {
            match q {
                MainQuestion::FoundSomething => {
                    dialoge.update(State::WalkStarted { walk }).await?;
                }
                MainQuestion::FoundDeadFrog => unreachable!("I think this is unreachable"),
                MainQuestion::WhereIsFrogHeaded => frog.towards = None,
                MainQuestion::WhereAreYou => frog.location = None,
                MainQuestion::AskForSex(_) => frog.sex = None,
            }
            bot.delete_message(dialoge.chat_id(), last_message_id)
                .await?;
            return Ok(());
        }
        if frog.location.is_some() {
            State::frog_identified_sex_location(bot, (frog, walk), dialoge, poll).await?;
        } else if frog.sex.is_some() {
            State::frog_identified_sex(bot, (frog, walk), dialoge, poll).await?;
        } else {
            let sex = match poll.selected() {
                "Male" => Sex::Male,
                "Female" => Sex::Female,
                "Unknown" => Sex::Unknown,
                "Use Questionaire" => {
                    let last_message_id =
                        questionaire::start_sex(bot, dialoge.chat_id(), &frog.name).await?;
                    dialoge
                        .update(State::QuestionaireSex {
                            walk,
                            questionaire: questionaire::QuestionaireSex::new(
                                frog.clone(),
                                last_message_id,
                            ),
                        })
                        .await?;
                    return Ok(());
                }
                _ => unreachable!(),
            };
            frog.sex = Some(sex);
            let last_message_id = MainQuestion::WhereAreYou
                .ask(bot, dialoge.chat_id())
                .await?;
            dialoge
                .update(State::FrogIdentified {
                    frog,
                    walk,
                    last_message_id,
                })
                .await?;
        }
        Ok(())
    }
    async fn frog_identified_sex(
        bot: Bot,
        (mut frog, walk): (PartialFrog, CompleteWalk),
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        let location = poll.selected_index();
        if location < 0 {
            bail!("We do not really handle you unselecting something. sorry.");
        }
        let location = location as usize;
        frog.location = Some(location);
        let last_message_id = MainQuestion::WhereIsFrogHeaded
            .ask(bot, dialoge.chat_id())
            .await?;
        dialoge
            .update(State::FrogIdentified {
                frog,
                walk,
                last_message_id,
            })
            .await?;
        Ok(())
    }

    async fn frog_identified_sex_location(
        bot: Bot,
        (mut frog, mut walk): (PartialFrog, CompleteWalk),
        dialoge: DialogueState,
        poll: Poll,
    ) -> anyhow::Result<()> {
        let towards = poll.selected_index() == 0;
        if poll.selected_index() < 0 {
            bail!("We do not really handle you unselecting something. sorry.");
        }
        frog.towards = Some(towards);
        let frog = frog.build()?;
        bot.send_message(dialoge.chat_id(), format!("Found {}", frog.to_message()))
            .reply_markup(InlineKeyboardMarkup::new([
                [InlineKeyboardButton::callback("Repeat", "found:repeat")],
                [InlineKeyboardButton::callback("Find", "found:next")],
                [InlineKeyboardButton::callback("End", "found:end")],
            ]))
            .await?;
        walk.frogs.push(frog);
        walk.repeats = 1;
        dialoge.update(State::WalkStarted { walk }).await?;
        Ok(())
    }
}

fn if_is_relevant(last_location: LastLocation) -> Option<TimedLocation> {
    let last_location = last_location.lock();
    if last_location.latitude.is_nan() || last_location.longitude.is_nan() {
        None
    } else if (Local::now() - last_location.time).num_minutes() > 5 {
        None
    } else {
        Some(last_location.clone())
    }
}

async fn inline_keyboard_button_pressed(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
    cb: CallbackQuery,
    last_location: LastLocation,
    mode: Mode,
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
            frog.gps_location = if_is_relevant(last_location);
            walk.frogs.push(frog);
            is_repeat = true;
        }
        Some("found:next") => {
            bot.answer_callback_query(cb.id).await?;
            MainQuestion::FoundSomething
                .ask(bot, dialoge.chat_id())
                .await?;
            return Ok(());
        }
        Some("found:end") => {
            bot.answer_callback_query(cb.id).await?;
            let walk = std::mem::take(walk);
            maybe_end_walk(bot, walk, dialoge, mode).await?;
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
            bot.send_message(dialoge.chat_id(), "Enter now your starting temperature:")
                .await?;
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
            bot.send_message(dialoge.chat_id(), "Enter now your ending temperature:")
                .await?;
            bot.answer_callback_query(cb.id).await?;
            return Ok(());
        }
        Some("weather:percipation-change") => {
            dialoge
                .update(State::ChangePercipation {
                    prev_state: Box::new(state),
                })
                .await?;
            bot.send_poll(
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
            .await?;
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
    dialoge: Dialogue<State, InMemStorage<State>>,
    mode: Mode,
) -> Result<(), anyhow::Error> {
    if mode.is_debug() {
        bot.send_message(
            dialoge.chat_id(),
            "You are in debug mode. If this is a real walk switch now.",
        )
        .reply_markup(
            InlineKeyboardMarkup::default()
                .append_row([InlineKeyboardButton::callback("Switch", "end:switch")])
                .append_row([InlineKeyboardButton::callback("Debug", "end:debug")]),
        )
        .await?;
        dialoge.update(State::WaitForModeChange { walk }).await?;
        Ok(())
    } else {
        end_walk(bot, walk, dialoge, mode).await
    }
}

async fn end_walk(
    bot: Bot,
    mut walk: CompleteWalk,
    dialoge: Dialogue<State, InMemStorage<State>>,
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

type LastLocation = Arc<Mutex<TimedLocation>>;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TimedLocation {
    latitude: f64,
    longitude: f64,
    time: DateTime<Local>,
}

impl TimedLocation {
    pub fn error() -> Self {
        Self {
            latitude: f64::NAN,
            longitude: f64::NAN,
            time: DateTime::UNIX_EPOCH.with_timezone(&Local),
        }
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
                    .endpoint(async |bot, dialoge: DialogueState| {
                        MainQuestion::FoundSomething
                            .ask(bot, dialoge.chat_id())
                            .await
                            .map(|_| ())
                    }),
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
                .branch(dptree::case![State::Start].endpoint(State::start))
                .branch(
                    dptree::case![State::EnterTemperature {
                        is_start,
                        prev_state
                    }]
                    .endpoint(State::enter_temperature),
                ),
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
                    dptree::case![State::FrogIdentified {
                        frog,
                        walk,
                        last_message_id
                    }]
                    .endpoint(State::frog_identified),
                ),
        )
        .branch(
            Update::filter_edited_message()
                .filter_map(|u: Update| {
                    if let UpdateKind::EditedMessage(m) = u.kind {
                        m.location().cloned()
                    } else {
                        None
                    }
                })
                .endpoint(async |l: Location, tl: LastLocation| {
                    let mut tl = tl.lock();
                    tl.latitude = l.latitude;
                    tl.longitude = l.longitude;
                    tl.time = Local::now();
                    Ok(())
                }),
        );

    Dispatcher::builder(bot, schema)
        .enable_ctrlc_handler()
        .error_handler(Arc::new(error_handler))
        .dependencies(dptree::deps![
            InMemStorage::<State>::new(),
            Arc::new(Mutex::<ChatId>::new(ChatId(0))),
            Mode::create_debug(),
            Arc::new(Mutex::new(TimedLocation::error()))
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
