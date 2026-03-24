use anyhow::bail;
use chrono::Local;
use teloxide::{
    prelude::*,
    types::{
        CallbackQueryId, InlineKeyboardButton, InlineKeyboardMarkup, InputPollOption, MessageId,
    },
};

use crate::{
    CompleteWalk,
    end_walk::{end_walk, maybe_end_walk},
    polls::MainQuestion,
    state::State,
    utils::*,
    weather::{self, WeatherStats},
};

pub async fn inline_keyboard_end_pressed(
    bot: Bot,
    argument: &str,
    cb_id: CallbackQueryId,
    walk: CompleteWalk,
    dialoge: DialogueState,
) -> R {
    bot.answer_callback_query(cb_id).await?;
    let mode = if argument == "debug" {
        Mode::create_debug()
    } else {
        Mode::create_release()
    };
    end_walk(bot, walk, dialoge, mode).await?;
    Ok(())
}

pub async fn inline_keyboard_weather_pressed(
    bot: Bot,
    argument: &str,
    cb_id: CallbackQueryId,
    state: State,
    message_id: MessageId,
    dialoge: DialogueState,
    sent: SentMessage,
) -> R {
    let before = state.as_walk().unwrap().weather;
    let weather = {
        let mut walk = state.as_walk_mut().unwrap();
        match argument {
            "wind-0" => {
                walk.weather.wind_beaufort = weather::Beaufort::Zero;
            }
            "wind-minus" => {
                walk.weather.wind_beaufort = walk.weather.wind_beaufort.decrease();
            }
            "wind-plus" => {
                walk.weather.wind_beaufort = walk.weather.wind_beaufort.increase();
            }
            "wind-6" => {
                walk.weather.wind_beaufort = weather::Beaufort::Six;
            }
            "clouds-0" => {
                walk.weather.cloudiness = weather::Cloudiness::Clear;
            }
            "clouds-less" => {
                walk.weather.cloudiness = walk.weather.cloudiness.decrease();
            }
            "clouds-more" => {
                walk.weather.cloudiness = walk.weather.cloudiness.increase();
            }
            "clouds-100" => {
                walk.weather.cloudiness = weather::Cloudiness::AllClouds;
            }
            "ground-wet" => {
                walk.weather.ground_humidity = weather::GroundHumidity::Wet;
            }
            "ground-humid" => {
                walk.weather.ground_humidity = weather::GroundHumidity::Humid;
            }
            "ground-dry" => {
                walk.weather.ground_humidity = weather::GroundHumidity::Dry;
            }
            "ground-very-dry" => {
                walk.weather.ground_humidity = weather::GroundHumidity::VeryDry;
            }
            "temperature-start-change" => {
                drop(walk);
                state.change_to_enter_temperature(true);
                let id = bot
                    .send_message(dialoge.chat_id(), "Enter now your starting temperature:")
                    .await?
                    .id;
                sent.add_weather(id);
                bot.answer_callback_query(cb_id).await?;
                return Ok(());
            }
            "temperature-end-change" => {
                drop(walk);
                state.change_to_enter_temperature(false);
                let id = bot
                    .send_message(dialoge.chat_id(), "Enter now your ending temperature:")
                    .await?
                    .id;
                sent.add_weather(id);
                bot.answer_callback_query(cb_id).await?;
                return Ok(());
            }
            "percipation-change" => {
                drop(walk);
                state.change_to_percipation();
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
                    .is_anonymous(false)
                    .await?
                    .id;
                sent.add_weather(id);
                bot.answer_callback_query(cb_id).await?;
                return Ok(());
            }
            // None => todo!(),
            _ => bail!("TODO"),
        }
        walk.weather.clone()
    };

    // let message_id = cb.message.unwrap().id();
    bot.answer_callback_query(cb_id).await?;

    if before != weather {
        let m = bot.edit_message_text(dialoge.chat_id(), message_id, weather.as_message());
        m.reply_markup(WeatherStats::default_weather_keyboard_markup())
            .await?;
    }

    Ok(())
}

async fn inline_keyboard_found_pressed(
    bot: Bot,
    argument: &str,
    cb_id: CallbackQueryId,
    state: State,
    dialoge: DialogueState,
    sent: SentMessage,
    last_location: LastLocation,
    message_id: MessageId,
    mode: Mode,
) -> R {
    bot.answer_callback_query(cb_id).await?;
    match argument {
        "repeat" => {
            let mut frog = state.as_walk().unwrap().frogs.last().cloned().unwrap();
            frog.time = Local::now();
            frog.gps_location = last_location.as_location();
            state.as_walk_mut().unwrap().frogs.push(frog);
            // let message_id = cb.message.unwrap().id();

            state.as_walk_mut().unwrap().repeats += 1;
            let walk = state.as_walk().unwrap();
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
        }
        "next" => {
            sent.clear_history();
            sent.add_to_history(
                MainQuestion::FoundSomething
                    .ask(bot, dialoge.chat_id())
                    .await?,
            );
        }
        "end" => {
            let walk = state.as_walk().unwrap();
            maybe_end_walk(bot, walk, dialoge, mode, sent).await?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

pub fn inline_keyboard_button_pressed(
    bot: Bot,
    state: State,
    dialoge: DialogueState,
    cb: CallbackQuery,
    last_location: LastLocation,
    mode: Mode,
    sent: SentMessage,
) -> impl Future<Output = anyhow::Result<()>> + Send {
    async move {
        let message_id = cb.regular_message().unwrap().id;
        let Some(id) = cb.data.as_ref() else {
            return Ok(());
        };
        let mut parts = id.split(':');
        let area = parts.next().unwrap();
        let argument = parts.next().unwrap();
        match area {
            "end" => {
                inline_keyboard_end_pressed(bot, argument, cb.id, state.as_walk().unwrap(), dialoge)
                    .await
            }
            "found" => {
                inline_keyboard_found_pressed(
                    bot,
                    argument,
                    cb.id,
                    state,
                    dialoge,
                    sent,
                    last_location,
                    message_id,
                    mode,
                )
                .await
            }
            "weather" => {
                inline_keyboard_weather_pressed(
                    bot, argument, cb.id, state, message_id, dialoge, sent,
                )
                .await
            }
            _ => unreachable!(),
        }
    }
}
