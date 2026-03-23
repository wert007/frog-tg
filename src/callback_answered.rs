use anyhow::bail;
use chrono::Local;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, InputPollOption},
};

use crate::{
    end_walk::{end_walk, maybe_end_walk},
    polls::MainQuestion,
    state::State,
    utils::*,
    weather::{self, WeatherStats},
};

pub async fn inline_keyboard_button_pressed(
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
