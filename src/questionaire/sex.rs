use anyhow::{anyhow, bail};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, types::InputPollOption};

use crate::{CompleteWalk, PollExt, Sex, State};

pub async fn erdkroete(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select which ever applies",
        [
            "Schwarze Brunstschwielen an den inneren drei Fingern",
            "Kräftige Arme",
            "Klammerreflex",
            "Schallblase",
            "Keins davon",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn erdkroete_answered(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
    walk: CompleteWalk,
    poll: Poll,
) -> Result<(), anyhow::Error> {
    let sex = match poll.selected_index() {
        -1 => bail!("Unselecting not supported!"),
        0..3 => Sex::Male,
        3 => Sex::Female,
        _ => Sex::Unknown,
    };
    dialoge
        .update(State::FrogIdentifiedSex {
            name: "Erdkröte".into(),
            walk,
            sex,
        })
        .await?;
    Ok(())
}
