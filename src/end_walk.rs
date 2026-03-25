use anyhow::Context;
use chrono::Local;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile},
};

use crate::{CompleteWalk, reports, utils::*};

pub async fn maybe_end_walk(
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
        dialoge
            .get_or_default()
            .await?
            .change_to_wait_for_mode_change();
        Ok(())
    } else {
        end_walk(bot, walk, dialoge, mode).await
    }
}

pub async fn end_walk(
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

    dialoge.get_or_default().await?.change_to_start();
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
