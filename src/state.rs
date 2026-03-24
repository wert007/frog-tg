use std::sync::Arc;

use crate::{
    CompleteWalk, DeadFrog, PartialFrog, Sex,
    end_walk::maybe_end_walk,
    notes,
    polls::{MainQuestion, QuestionaireQuestion},
    questionaire::{self, QuestionaireFrogName, QuestionaireSex},
    utils::*,
    weather::{self, BotWeatherExt, WeatherStats},
};
use anyhow::{Context, anyhow, bail};
use chrono::Local;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId},
};

#[derive(Debug, Default, Clone)]
pub struct State {
    walk: Option<Arc<Mutex<CompleteWalk>>>,
    state: Arc<Mutex<StateState>>,
}

#[derive(Debug, Default, Clone)]
pub enum StateState {
    #[default]
    Start,
    WalkStarted,
    WaitForModeChange,
    QuestionaireFrogName(questionaire::QuestionaireFrogName),
    QuestionaireSex(questionaire::QuestionaireSex),
    DeadFrog,
    DeadFrogName(Option<String>),
    FrogIdentified(PartialFrog),
    Temperature {
        is_start: bool,
        prev: Box<Arc<Mutex<StateState>>>,
    },
    ChangePercipation(Box<Arc<Mutex<StateState>>>),
}

const fn is_send_and_sync<T: Send + Sync>() {}

const _: () = const {
    is_send_and_sync::<StateState>();
    is_send_and_sync::<State>();
};

impl StateState {
    fn go_back(&self) -> Arc<Mutex<StateState>> {
        Arc::new(Mutex::new(match self {
            Self::WalkStarted | Self::Start => self.clone(),
            Self::DeadFrog | Self::WaitForModeChange => Self::WalkStarted,
            Self::QuestionaireFrogName(questionaire) => {
                if let Some(questionaire) = questionaire.go_back() {
                    Self::QuestionaireFrogName(questionaire)
                } else {
                    Self::WalkStarted
                }
            }
            Self::QuestionaireSex(questionaire) => {
                if let Some(questionaire) = questionaire.go_back() {
                    Self::QuestionaireSex(questionaire)
                } else {
                    Self::FrogIdentified(questionaire.frog.clone())
                }
            }
            Self::DeadFrogName(name) => {
                if name.is_some() {
                    Self::DeadFrogName(None)
                } else {
                    Self::WalkStarted
                }
            }
            Self::FrogIdentified(frog) => {
                if let Some(frog) = frog.go_back() {
                    Self::FrogIdentified(frog)
                } else {
                    // TODO: Check that using questionaire does not break
                    // anything here!
                    // Or better put, we need to remove previously added frogs
                    // at some point, don't we?
                    Self::WalkStarted
                }
            }
            Self::ChangePercipation(prev) | Self::Temperature { prev, .. } => return *prev.clone(),
        }))
    }
}

// #[derive(Debug, Default, Clone)]
// pub enum State {
//     #[default]
//     Start,
//     WalkStarted {
//         walk: CompleteWalk,
//     },
//     WaitForModeChange {
//         walk: CompleteWalk,
//     },
//     QuestionaireFrogName {
//         walk: CompleteWalk,
//         questionaire: questionaire::QuestionaireFrogName,
//     },
//     QuestionaireSex {
//         walk: CompleteWalk,
//         questionaire: questionaire::QuestionaireSex,
//     },
//     DeadFrog {
//         walk: CompleteWalk,
//     },
//     DeadFrogName {
//         walk: CompleteWalk,
//         name: Option<String>,
//     },
//     FrogIdentified {
//         frog: PartialFrog,
//         walk: CompleteWalk,
//     },
//     EnterTemperature {
//         is_start: bool,
//         prev_state: Box<State>,
//     },
//     ChangePercipation {
//         prev_state: Box<State>,
//     },
// }

impl State {
    pub fn text_message(
        s: State,
        message: Message,
        bot: Bot,
        dialoge: DialogueState,
        location: LastLocation,
        sent: SentMessage,
    ) -> impl Future<Output = R> + Send {
        async move {
            let text = message.text().unwrap_or_default().to_string();
            let state: StateState = s.state.lock().clone();
            match state {
                StateState::Start if text.to_lowercase().trim() == "/start" => {
                    State::start(bot, dialoge, sent).await
                    // Ok(())
                }
                StateState::Temperature { is_start, prev } => {
                    let temp = message
                        .text()
                        .ok_or(anyhow!("There should be a text message???"))?;
                    let temp: f64 = temp.parse()?;
                    if is_start {
                        s.weather_mut().temperature_start = temp;
                    } else {
                        s.weather_mut().temperature_end = Some(temp);
                    }
                    let m = bot
                        .send_weather_stats(dialoge.chat_id(), s.as_walk().unwrap().weather)
                        .await?;
                    sent.add_weather(m.id);
                    let statestate = prev.lock().clone();
                    s.change_to(statestate);
                    Ok(())
                }
                // _ => Ok(()),
                _ => notes::add_note(bot, dialoge, message, text, location, sent).await,
            }
        }
    }
    pub fn is_start(&self) -> bool {
        matches!(*self.state.lock(), StateState::Start)
    }
    // pub fn as_walk(&self) -> Option<CompleteWalk> {
    //     match self {
    //         State::Start => None,
    //         State::WalkStarted | State::WaitForModeChange { walk } => Some(walk.clone()),
    //         State::QuestionaireFrogName { walk, .. } => Some(walk.clone()),
    //         State::QuestionaireSex { walk, .. } => Some(walk.clone()),
    //         State::DeadFrog { walk } => Some(walk.clone()),
    //         State::DeadFrogName { walk, .. } => Some(walk.clone()),
    //         State::FrogIdentified { walk, .. } => Some(walk.clone()),
    //         State::ChangePercipation { .. } | State::EnterTemperature { .. } => todo!(),
    //     }
    // }

    // pub fn as_walk_mut(&mut self) -> Option<&mut CompleteWalk> {
    //     match self {
    //         State::Start => None,
    //         State::WalkStarted { walk } | State::WaitForModeChange { walk } => Some(walk),
    //         State::QuestionaireFrogName { walk, .. } => Some(walk),
    //         State::QuestionaireSex { walk, .. } => Some(walk),
    //         State::DeadFrog { walk } => Some(walk),
    //         State::DeadFrogName { walk, .. } => Some(walk),
    //         State::FrogIdentified { walk, .. } => Some(walk),
    //         State::ChangePercipation { prev_state }
    //         | State::EnterTemperature { prev_state, .. } => prev_state.as_walk_mut(),
    //     }
    // }

    // pub fn enter_temperature(
    //     bot: Bot,
    //     dialoge: DialogueState,
    //     (is_start, prev_state, mut weather): (
    //         bool,
    //         Box<Arc<Mutex<StateState>>>,
    //         MappedMutexGuard<'_, WeatherStats>,
    //     ),
    //     message: Message,
    //     sent: SentMessage,
    // ) -> impl Future<Output = anyhow::Result<()>> + Send + Sync {
    //     async move {
    //         let temp = message
    //             .text()
    //             .ok_or(anyhow!("There should be a text message???"))?;
    //         let temp: f64 = temp.parse()?;
    //         if is_start {
    //             weather.temperature_start = temp;
    //         } else {
    //             weather.temperature_end = Some(temp);
    //         }
    //         let m = bot.send_weather_stats(dialoge.chat_id(), *weather).await?;
    //         sent.add_weather(m.id);
    //         dialoge
    //             .get_or_default()
    //             .await?
    //             .change_to(prev_state.lock().clone());
    //         Ok(())
    //     }
    // }

    pub async fn start(bot: Bot, dialoge: DialogueState, sent: SentMessage) -> anyhow::Result<()> {
        sent.clear();
        let walk = CompleteWalk::start()
            .await
            .context("Creating walk for new walk created by user")?;
        bot.send_weather_stats(dialoge.chat_id(), walk.weather)
            .await
            .context("Sending the weather via tg to user")?;
        dialoge
            .update(State {
                walk: Some(Arc::new(Mutex::new(walk))),
                ..Default::default()
            })
            .await?;
        Ok(())
    }

    pub fn change_percipation(
        bot: Bot,
        prev_state: StateState,
        state: State,
        dialoge: DialogueState,
        poll: Poll,
        sent: SentMessage,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        use weather::Percipation::*;
        async move {
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
            let weather = {
                let mut weather_mutex = state.weather_mut();
                weather_mutex.percipation = percipation;
                weather_mutex.clone()
            };
            let w = bot.send_weather_stats(dialoge.chat_id(), weather).await?.id;
            state.change_to(prev_state);
            sent.add_weather(w);
            Ok(())
        }
    }

    pub async fn poll_answer_walk_started(
        bot: Bot,
        last_location: LastLocation,
        walk: CompleteWalk,
        dialoge: DialogueState,
        poll: Poll,
        mode: Mode,
        sent: SentMessage,
    ) -> anyhow::Result<()> {
        if poll.selected_index() < 0 {
            sent.go_back(bot, dialoge).await?;
            return Ok(());
        }
        match poll.selected() {
            "End" => {
                maybe_end_walk(bot, walk, dialoge, mode, sent).await?;
            }
            name @ ("Erdkröte" | "Grasfrosch" | "Teichmolch" | "Bergmolch" | "Kammmolch") => {
                let last_message_id = MainQuestion::AskForSex(name.into())
                    .ask(bot, dialoge.chat_id())
                    .await?;
                sent.add_frog(last_message_id, walk.frogs.len());
                dialoge
                    .get_or_default()
                    .await?
                    .change_to_frog_identified(PartialFrog {
                        name: name.into(),
                        gps_location: last_location.as_location(),
                        ..Default::default()
                    });
            }
            "Found Something" => {
                let last_message_id = QuestionaireQuestion::IsItAFrogToadOrMolch
                    .ask(bot.clone(), dialoge.chat_id())
                    .await?;
                sent.add_frog(last_message_id, walk.frogs.len());
                dialoge
                    .get_or_default()
                    .await?
                    .change_to_questionaire_frog_name(QuestionaireFrogName::new());
            }
            "Dead Frog :(" => {
                let id = MainQuestion::FoundDeadFrog
                    .ask(bot, dialoge.chat_id())
                    .await?;
                sent.add_dead_frog(id, walk.dead_frogs.len());
                dialoge.get_or_default().await?.change_to_dead_frog();
            }
            _ => bail!("TODO"),
        }
        Ok(())
    }

    pub async fn dead_frog_answered(
        bot: Bot,
        walk: CompleteWalk,
        dialoge: DialogueState,
        poll: Poll,
        sent: SentMessage,
    ) -> anyhow::Result<()> {
        if poll.selected_index() < 0 {
            sent.go_back(bot, dialoge).await?;
            return Ok(());
        }
        let name = match poll.selected() {
            "No" => None,
            name => Some(name.to_string()),
        };
        let id = MainQuestion::WhereAreYou
            .ask(bot, dialoge.chat_id())
            .await?;
        sent.add_dead_frog(id, walk.dead_frogs.len());
        dialoge
            .get_or_default()
            .await?
            .change_to_dead_frog_name(name);
        Ok(())
    }

    pub async fn dead_frog_location_answered(
        bot: Bot,
        (mut walk, name): (CompleteWalk, Option<String>),
        dialoge: DialogueState,
        poll: Poll,
        sent: SentMessage,
    ) -> anyhow::Result<()> {
        let location = poll.selected_index();
        if location < 0 {
            sent.go_back(bot, dialoge).await?;
            return Ok(());
        }
        let location = location as usize;
        walk.dead_frogs.push(DeadFrog {
            name,
            sex: None,
            location,
            time: Local::now(),
        });
        dialoge.get_or_default().await?.change_to_default();
        sent.clear_history();
        sent.add_to_history(
            MainQuestion::FoundSomething
                .ask(bot, dialoge.chat_id())
                .await?,
        );
        Ok(())
    }

    pub async fn frog_identified(
        bot: Bot,
        (mut frog, walk): (PartialFrog, CompleteWalk),
        dialoge: DialogueState,
        poll: Poll,
        sent: SentMessage,
    ) -> anyhow::Result<()> {
        if poll.selected_index() < 0
        // && let Some(q) = MainQuestion::find_original(&poll.question)
        {
            sent.go_back(bot, dialoge).await?;
            // match q {
            //     MainQuestion::FoundSomething => {
            //         dialoge.update(State::WalkStarted { walk }).await?;
            //     }
            //     MainQuestion::FoundDeadFrog => unreachable!("I think this is unreachable"),
            //     MainQuestion::WhereIsFrogHeaded => frog.towards = None,
            //     MainQuestion::WhereAreYou => frog.location = None,
            //     MainQuestion::AskForSex(_) => frog.sex = None,
            // }
            // bot.delete_message(dialoge.chat_id(), last_message_id)
            //     .await?;
            return Ok(());
        }
        if frog.location.is_some() {
            State::frog_identified_sex_location(bot, (frog, walk), dialoge, poll, sent).await?;
        } else if frog.sex.is_some() {
            State::frog_identified_sex(bot, (frog, walk), dialoge, poll, sent).await?;
        } else {
            let sex = match poll.selected() {
                "Male" => Sex::Male,
                "Female" => Sex::Female,
                "Unknown" => Sex::Unknown,
                "Use Questionaire" => {
                    let last_message_id =
                        questionaire::start_sex(bot, dialoge.chat_id(), &frog.name).await?;
                    sent.add_frog(last_message_id, walk.frogs.len());
                    dialoge
                        .get_or_default()
                        .await?
                        .change_to_questionaire_sex(frog);
                    return Ok(());
                }
                _ => unreachable!(),
            };
            frog.sex = Some(sex);
            let last_message_id = MainQuestion::WhereAreYou
                .ask(bot, dialoge.chat_id())
                .await?;
            sent.add_frog(last_message_id, walk.frogs.len());
            dialoge
                .get_or_default()
                .await?
                .change_to_frog_identified(frog);
        }
        Ok(())
    }

    async fn frog_identified_sex(
        bot: Bot,
        (mut frog, walk): (PartialFrog, CompleteWalk),
        dialoge: DialogueState,
        poll: Poll,
        sent: SentMessage,
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
        sent.add_frog(last_message_id, walk.frogs.len());
        dialoge
            .get_or_default()
            .await?
            .change_to_frog_identified(frog);
        Ok(())
    }

    async fn frog_identified_sex_location(
        bot: Bot,
        (mut frog, mut walk): (PartialFrog, CompleteWalk),
        dialoge: DialogueState,
        poll: Poll,
        sent: SentMessage,
    ) -> anyhow::Result<()> {
        let towards = poll.selected_index() == 0;
        if poll.selected_index() < 0 {
            bail!("We do not really handle you unselecting something. sorry.");
        }
        frog.towards = Some(towards);
        let frog = frog.build()?;
        let id = bot
            .send_message(dialoge.chat_id(), format!("Found {}", frog.to_message()))
            .reply_markup(InlineKeyboardMarkup::new([
                [InlineKeyboardButton::callback("Repeat", "found:repeat")],
                [InlineKeyboardButton::callback("Find", "found:next")],
                [InlineKeyboardButton::callback("End", "found:end")],
            ]))
            .await?
            .id;
        sent.add_frog(id, walk.frogs.len());
        walk.frogs.push(frog);
        walk.repeats = 1;
        dialoge.get_or_default().await?.change_to_default();
        Ok(())
    }

    pub fn go_back(&self) -> Self {
        Self {
            state: self.state.lock().go_back(),
            walk: self.walk.clone(),
        }
    }

    pub fn change_to_frog_identified(&self, frog: PartialFrog) {
        self.change_to(StateState::FrogIdentified(frog))
    }

    fn weather_mut<'a>(&'a self) -> parking_lot::MappedMutexGuard<'a, WeatherStats> {
        MutexGuard::map(self.walk.as_ref().expect("Should be set!").lock(), |w| {
            &mut w.weather
        })
    }

    pub fn change_to_questionaire_frog_name(&self, questionaire: QuestionaireFrogName) {
        self.change_to(StateState::QuestionaireFrogName(questionaire));
    }

    fn change_to_dead_frog(&self) {
        self.change_to(StateState::DeadFrog);
    }

    fn change_to_dead_frog_name(&self, name: Option<String>) {
        self.change_to(StateState::DeadFrogName(name));
    }

    fn change_to_default(&self) {
        self.change_to(StateState::WalkStarted);
    }

    fn change_to_questionaire_sex(&self, frog: PartialFrog) {
        self.change_to(StateState::QuestionaireSex(QuestionaireSex::new(frog)));
    }

    fn change_to(&self, state: StateState) {
        *self.state.lock() = state;
    }

    pub(crate) fn as_walk(&self) -> Option<CompleteWalk> {
        self.walk.as_ref().map(|w| w.lock().clone())
    }

    pub(crate) fn as_walk_mut<'a>(&'a self) -> Option<MutexGuard<'a, CompleteWalk>> {
        self.walk.as_ref().map(|w| w.lock())
    }

    pub(crate) fn change_to_enter_temperature(&self, is_start: bool) {
        self.change_to(StateState::Temperature {
            is_start,
            prev: Box::new(self.state.clone()),
        });
    }

    pub(crate) fn change_to_percipation(&self) {
        self.change_to(StateState::ChangePercipation(Box::new(self.state.clone())));
    }

    pub(crate) fn change_to_start(&self) {
        self.change_to(StateState::Start);
    }

    pub(crate) fn change_to_wait_for_mode_change(&self) {
        self.change_to(StateState::WaitForModeChange);
    }

    pub(crate) fn get_state(&self) -> StateState {
        self.state.lock().clone()
    }
}
