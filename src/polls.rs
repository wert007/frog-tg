use std::borrow::Cow;

use teloxide::{
    prelude::*,
    types::{InputPollOption, MessageId},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Question {
    FoundSomething,
    FoundDeadFrog,
    WhereIsFrogHeaded,
    WhereAreYou,
    AskForSex(String),
}

impl Question {
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

    pub fn find_original(msg: &str) -> Option<Question> {
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
            Question::FoundSomething => "What did you find?".into(),
            Question::FoundDeadFrog => "Can you still recognize what it was?".into(),
            Question::WhereIsFrogHeaded => "Is it going towards water".into(),
            Question::WhereAreYou => "Where are you right now?".into(),
            Question::AskForSex(name) => format!("Enter the sex of your {name} now:").into(),
        }
    }

    fn options(&self) -> Vec<&'static str> {
        match self {
            Question::FoundSomething => [
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
            Question::FoundDeadFrog => [
                "No",
                "Erdkröte",
                "Grasfrosch",
                "Teichmolch",
                "Bergmolch",
                "Kammmolch",
            ]
            .into(),
            Question::WhereIsFrogHeaded => ["Towards water", "Back from water"].into(),
            Question::WhereAreYou => include_str!("../locations.txt")
                .lines()
                .filter(|l| !l.is_empty())
                .collect(),
            Question::AskForSex(_) => ["Male", "Female", "Unknown", "Use Questionaire"].into(),
        }
    }
}
