use std::borrow::Cow;

use teloxide::{
    prelude::*,
    types::{InputPollOption, MessageId},
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
            .await?
            .id)
    }

    pub fn find_original(msg: &str) -> Option<MainQuestion> {
        if msg == Self::FoundSomething.question() {
            Some(Self::FoundSomething)
        } else if msg == Self::FoundDeadFrog.question() {
            Some(Self::FoundDeadFrog)
        } else if msg == Self::WhereIsFrogHeaded.question() {
            Some(Self::WhereIsFrogHeaded)
        } else if msg == Self::WhereAreYou.question() {
            Some(Self::WhereAreYou)
        } else if let Some(name) = msg.strip_prefix("Enter the sex of your ") {
            Some(Self::AskForSex(name.split(' ').next()?.into()))
        } else {
            None
        }
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
