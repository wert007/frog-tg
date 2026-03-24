use crate::{CompleteWalk, utils::*};
use anyhow::anyhow;
use chrono::{DateTime, Local};
use teloxide::{prelude::*, types::ReactionType};

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct Note {
    pub text: String,
    pub location: usize,
    pub source: MessageClassification,
    pub time: DateTime<Local>,
    gps_location: Option<TimedLocation>,
}

impl CompleteWalk {
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

pub async fn add_note(
    bot: Bot,
    dialoge: DialogueState,
    message: Message,
    text: String,
    location: LastLocation,
    sent: SentMessage,
) -> R {
    let s = dialoge.get().await?.ok_or(anyhow!(
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
