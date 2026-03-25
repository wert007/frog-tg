use std::borrow::Cow;

use teloxide::{
    prelude::*,
    types::{InputPollOption, MessageId},
};

use crate::{
    questionaire,
    state::{State, StateState},
    utils::*,
};

pub enum QuestionaireQuestion {
    IsItAFrogToadOrMolch,
    ItIsAMolch,
    ItIsAFrog,
    ItIsAToad,
}

impl QuestionaireQuestion {
    pub async fn ask(&self, bot: Bot, chat_id: ChatId) -> anyhow::Result<MessageId> {
        Ok(bot
            .send_poll(
                chat_id,
                self.question(),
                self.options().into_iter().map(InputPollOption::new),
            )
            .is_anonymous(false)
            .await?
            .id)
    }

    fn question(&self) -> Cow<'static, str> {
        match self {
            QuestionaireQuestion::IsItAFrogToadOrMolch => "Is it a frog, toad or a Molch?".into(),
            QuestionaireQuestion::ItIsAMolch => "Check its skin now!".into(),
            QuestionaireQuestion::ItIsAFrog => "Check its Skin and its Nose!".into(),
            QuestionaireQuestion::ItIsAToad => "Check its Skin now!".into(),
        }
    }
    fn options(&self) -> Vec<&'static str> {
        match self {
            QuestionaireQuestion::IsItAFrogToadOrMolch => vec![
                "Molch (Has Tail)",
                "Toad (Has Wards)",
                "Frog (No Wards)",
                "Unsure",
            ],
            QuestionaireQuestion::ItIsAMolch => vec![
                "There are white dots",
                "No dark markings on the bottom side",
                "Otherwise it is a Teichmolch",
                "Unsure",
            ],
            QuestionaireQuestion::ItIsAFrog => vec![
                "It has markings on its back",
                "Its nose is pointy",
                "Its nose is more stump",
                "Unsure",
            ],
            QuestionaireQuestion::ItIsAToad => vec![
                "Has markings on its back",
                "Red marks",
                "Has dark markings",
                "Has a lot of wards",
                "Unsure",
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MainQuestion {
    FoundSomething,
    FoundDeadFrog,
    WhereIsFrogHeaded,
    WhereAreYou,
    AskForSex(String),
}

impl MainQuestion {
    pub async fn ask(&self, bot: Bot, chat_id: ChatId) -> anyhow::Result<MessageId> {
        Ok(bot
            .send_poll(
                chat_id,
                self.question(),
                self.options().into_iter().map(InputPollOption::new),
            )
            .is_anonymous(false)
            .await?
            .id)
    }

    fn question(&self) -> Cow<'static, str> {
        match self {
            MainQuestion::FoundSomething => "What did you find?".into(),
            MainQuestion::FoundDeadFrog => "Can you still recognize what it was?".into(),
            MainQuestion::WhereIsFrogHeaded => "Is it going towards water".into(),
            MainQuestion::WhereAreYou => "Where are you right now?".into(),
            MainQuestion::AskForSex(name) => format!("Enter the sex of your {name} now:").into(),
        }
    }

    fn options(&self) -> Vec<&'static str> {
        match self {
            MainQuestion::FoundSomething => [
                "Found Something",
                "Dead Frog :(",
                "Erdkröte",
                "Grasfrosch",
                "Teichmolch",
                "Bergmolch",
                "Kammmolch",
                "End",
            ]
            .into(),
            MainQuestion::FoundDeadFrog => [
                "No",
                "Erdkröte",
                "Grasfrosch",
                "Teichmolch",
                "Bergmolch",
                "Kammmolch",
            ]
            .into(),
            MainQuestion::WhereIsFrogHeaded => ["Towards water", "Back from water"].into(),
            MainQuestion::WhereAreYou => include_str!("../locations.txt")
                .lines()
                .filter(|l| !l.is_empty())
                .collect(),
            MainQuestion::AskForSex(_) => ["Male", "Female", "Unknown", "Use Questionaire"].into(),
        }
    }
}

pub fn poll_answered(
    bot: Bot,
    dialoge: DialogueState,
    state: State,
    last_location: LastLocation,
    poll: PollAnswer,
    mode: Mode,
    sent: SentMessage,
) -> impl Future<Output = R> + Send {
    async move {
        let statestate = state.get_state();
        match statestate {
            crate::state::StateState::WaitForModeChange | crate::state::StateState::WalkStarted => {
                let walk = state.as_walk().unwrap();
                State::poll_answer_walk_started(bot, last_location, walk, dialoge, poll, mode, sent)
                    .await
            }
            crate::state::StateState::QuestionaireFrogName(questionaire_frog_name) => {
                let walk = state.as_walk().unwrap();

                if questionaire_frog_name.species.is_none() {
                    questionaire::found_species(
                        bot,
                        dialoge,
                        (walk, questionaire_frog_name),
                        poll,
                        sent,
                    )
                    .await
                } else {
                    questionaire::found_frog_name(
                        bot,
                        dialoge,
                        last_location,
                        (walk, questionaire_frog_name),
                        poll,
                        sent,
                    )
                    .await
                }
            }
            crate::state::StateState::QuestionaireSex(questionaire_sex) => {
                questionaire::found_sex(
                    bot,
                    dialoge,
                    (state.as_walk().unwrap(), questionaire_sex),
                    poll,
                    sent,
                )
                .await
            }
            crate::state::StateState::DeadFrog => {
                State::dead_frog_answered(bot, state.as_walk().unwrap(), dialoge, poll, sent).await
            }
            crate::state::StateState::DeadFrogName(name) => {
                State::dead_frog_location_answered(
                    bot,
                    (state.as_walk().unwrap(), name),
                    dialoge,
                    poll,
                    sent,
                )
                .await
            }
            crate::state::StateState::FrogIdentified(partial_frog) => {
                State::frog_identified(
                    bot,
                    (partial_frog, state.as_walk().unwrap()),
                    dialoge,
                    poll,
                    sent,
                    state,
                )
                .await
            }
            crate::state::StateState::ChangePercipation(prev) => {
                let prev_state: StateState = *prev;
                State::change_percipation(bot, prev_state, state, dialoge, poll, sent).await
            }
            _ => {
                bot.send_message(
                    dialoge.chat_id(),
                    "Seems like you just changed a poll answer. Sadly I cannot react to that anymore.",
                )
                .await?;
                Ok(())
            }
        }
    }
}
