use teloxide::{Bot, prelude::Requester};

use crate::{
    reports,
    utils::{DialogueState, R},
};

pub async fn report(bot: Bot, dialogue: DialogueState) -> R {
    let text = match dialogue.get_or_default().await?.as_walk() {
        Some(w) if w.frogs.is_empty() => "Nothing has been found yet.".into(),
        Some(w) => reports::create_inline_end_walk_report(&w),
        _ => "Use command /start to start a new walk first.".into(),
    };

    bot.send_message(dialogue.chat_id(), text).await?;
    anyhow::Ok(())
}
