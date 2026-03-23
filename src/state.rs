use crate::{
    CompleteWalk, DeadFrog, PartialFrog, Sex, maybe_end_walk,
    polls::{MainQuestion, QuestionaireQuestion},
    questionaire::{self, QuestionaireFrogName},
    utils::{DialogueState, LastLocation, Mode, PollExt, SentMessage},
    weather::{self, BotWeatherExt},
};
use anyhow::{Context, anyhow, bail};
use chrono::Local;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

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
    pub fn as_walk(&self) -> Option<CompleteWalk> {
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

    pub fn as_walk_mut(&mut self) -> Option<&mut CompleteWalk> {
        match self {
            State::Start => None,
            State::WalkStarted { walk } | State::WaitForModeChange { walk } => Some(walk),
            State::QuestionaireFrogName { walk, .. } => Some(walk),
            State::QuestionaireSex { walk, .. } => Some(walk),
            State::DeadFrog { walk } => Some(walk),
            State::DeadFrogName { walk, .. } => Some(walk),
            State::FrogIdentified { walk, .. } => Some(walk),
            State::ChangePercipation { prev_state }
            | State::EnterTemperature { prev_state, .. } => prev_state.as_walk_mut(),
        }
    }

    pub async fn enter_temperature(
        bot: Bot,
        dialoge: DialogueState,
        (is_start, prev_state): (bool, Box<State>),
        message: Message,
        sent: SentMessage,
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
        let m = bot.send_weather_stats(dialoge.chat_id(), *weather).await?;
        sent.add_weather(m.id);
        dialoge.update(state).await?;
        Ok(())
    }

    pub async fn start(bot: Bot, dialoge: DialogueState, sent: SentMessage) -> anyhow::Result<()> {
        sent.clear();
        let walk = CompleteWalk::start()
            .await
            .context("Creating walk for new walk created by user")?;
        bot.send_weather_stats(dialoge.chat_id(), walk.weather)
            .await
            .context("Sending the weather via tg to user")?;
        dialoge.update(State::WalkStarted { walk }).await?;
        Ok(())
    }

    pub async fn change_percipation(
        bot: Bot,
        prev_state: Box<State>,
        dialoge: DialogueState,
        poll: Poll,
        sent: SentMessage,
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
        let w = bot
            .send_weather_stats(dialoge.chat_id(), *weather)
            .await?
            .id;
        sent.add_weather(w);
        dialoge.update(state).await?;
        Ok(())
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
                    .update(State::FrogIdentified {
                        frog: PartialFrog {
                            name: name.into(),
                            gps_location: last_location.as_location(),
                            ..Default::default()
                        },
                        walk,
                    })
                    .await?;
            }
            "Found Something" => {
                let last_message_id = QuestionaireQuestion::IsItAFrogToadOrMolch
                    .ask(bot.clone(), dialoge.chat_id())
                    .await?;
                sent.add_frog(last_message_id, walk.frogs.len());
                dialoge
                    .update(State::QuestionaireFrogName {
                        walk,
                        questionaire: QuestionaireFrogName::new(),
                    })
                    .await?;
            }
            "Dead Frog :(" => {
                let id = MainQuestion::FoundDeadFrog
                    .ask(bot, dialoge.chat_id())
                    .await?;
                sent.add_dead_frog(id, walk.dead_frogs.len());
                dialoge.update(State::DeadFrog { walk }).await?;
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
        dialoge.update(State::DeadFrogName { walk, name }).await?;
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
        dialoge.update(State::WalkStarted { walk }).await?;
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
                        .update(State::QuestionaireSex {
                            walk,
                            questionaire: questionaire::QuestionaireSex::new(frog.clone()),
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
            sent.add_frog(last_message_id, walk.frogs.len());
            dialoge.update(State::FrogIdentified { frog, walk }).await?;
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
        dialoge.update(State::FrogIdentified { frog, walk }).await?;
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
        dialoge.update(State::WalkStarted { walk }).await?;
        Ok(())
    }

    pub fn go_back(&self) -> Self {
        match self {
            State::WalkStarted { .. } | State::Start => self.clone(),
            State::DeadFrog { walk } | State::WaitForModeChange { walk } => {
                Self::WalkStarted { walk: walk.clone() }
            }
            State::QuestionaireFrogName { walk, questionaire } => {
                if let Some(questionaire) = questionaire.go_back() {
                    Self::QuestionaireFrogName {
                        walk: walk.clone(),
                        questionaire,
                    }
                } else {
                    Self::WalkStarted { walk: walk.clone() }
                }
            }
            State::QuestionaireSex { walk, questionaire } => {
                if let Some(questionaire) = questionaire.go_back() {
                    Self::QuestionaireSex {
                        walk: walk.clone(),
                        questionaire,
                    }
                } else {
                    Self::FrogIdentified {
                        frog: questionaire.frog.clone(),
                        walk: walk.clone(),
                    }
                }
            }
            State::DeadFrogName { walk, name } => {
                if name.is_some() {
                    Self::DeadFrogName {
                        walk: walk.clone(),
                        name: None,
                    }
                } else {
                    Self::WalkStarted { walk: walk.clone() }
                }
            }
            State::FrogIdentified { frog, walk } => {
                if let Some(frog) = frog.go_back() {
                    Self::FrogIdentified {
                        frog,
                        walk: walk.clone(),
                    }
                } else {
                    // TODO: Check that using questionaire does not break
                    // anything here!
                    Self::WalkStarted { walk: walk.clone() }
                }
            }
            State::ChangePercipation { prev_state }
            | State::EnterTemperature { prev_state, .. } => *prev_state.clone(),
        }
    }
}
