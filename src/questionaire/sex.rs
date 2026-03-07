use anyhow::bail;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, types::InputPollOption};

use crate::{PollExt, Sex, State};

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

pub(crate) async fn erdkroete_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("Unselecting not supported!"),
        0..3 => Sex::Male,
        3 => Sex::Female,
        _ => Sex::Unknown,
    })
}

pub(crate) async fn gruenfrosch(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select whichever applies",
        [
            "Seitliche Schallblasen",
            "Oben Zitronengelb",
            "Keine Schallblasen",
            "Unsicher/Keins davon",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn gruenfrosch_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("No unselecting supported!"),
        0..2 => Sex::Male,
        2 => Sex::Female,
        _ => Sex::Unknown,
    })
}

pub(crate) async fn teichmolch(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select whichever applied",
        [
            "Große schwarze Punkte auf Unterseite",
            "Kleine schwarze Punkte auf Unterseite",
            "(kleiner) Rückenkamm vorhanden",
            "Kein Rückenkamm vorhanden",
            "Punkte unterbrechen orange Linie auf Schwanzunterseite",
            "Keine Punkte auf Schwanzunterseite, durchgängige orange Linie",
            "Unsicher/Keins davon",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn teichmolch_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("No unselecting supported"),
        0 | 2 | 4 => Sex::Male,
        1 | 3 | 5 => Sex::Female,
        _ => Sex::Unknown,
    })
}

pub(crate) async fn grasfrosch(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select whichever applies",
        [
            "Kehle sieht blau aus",
            "Schwarze Brunstschwielen an Daumen",
            "Kräftige (Unter-)Arme",
            "Keine Brunstschwielen",
            "Weiße/Helle Pickel an Körperseite/(hinteren) Rücken/Hinterbeinen",
            "Unsicher/Nichts trifft zu",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn grasfrosch_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("No unselecting supported"),
        0..3 => Sex::Male,
        3..5 => Sex::Female,
        _ => Sex::Unknown,
    })
}

pub(crate) async fn kammmolch(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select whichever applies",
        [
            "silbriger Spiegel (Streifen) an Schwanzseite",
            "Schwanzunterseite Orange, durchgängig keine Punkte",
            "Unsicher/Nichts trifft zu",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn kammmolch_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("No unselecting supported"),
        0 => Sex::Male,
        1 => Sex::Female,
        _ => Sex::Unknown,
    })
}

pub(crate) async fn bergmolch(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select whichever applies",
        [
            "Punktstreifen an der Seite gehen bis auf die Arme",
            "Orangefarbene Linie auf Schwanzunterseite unterbrochen",
            "Rückenleiste (bis 2mm) vorhanden",
            "Keine Rückenleiste",
            "Durchgängige Orangefarbene Linie auf Schwanzunterseite",
            "Unsicher/Nichts trifft zu",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn bergmolch_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("Unselecting not supported yet"),
        0..3 => Sex::Male,
        3..5 => Sex::Female,
        _ => Sex::Unknown,
    })
}

pub(crate) async fn springfrosch(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select whichever applies",
        [
            "Helle/Graue Brunstschwielen an Daumen",
            "Quakt",
            "Keine Brunstschwielen und kein Quaken",
            "Unsicher",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn springfrosch_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("Unselecting not supported"),
        0..2 => Sex::Male,
        2 => Sex::Female,
        _ => Sex::Unknown,
    })
}

pub(crate) async fn knoblauchkroete(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select whichever applies",
        [
            "Ovale Drüse am Oberarm",
            "Quakt",
            "Hat keine Drüse am Oberarm und quakt nicht",
            "Unsicher",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn knoblauchkroete_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("Cannot unselect answers"),
        0..2 => Sex::Male,
        2 => Sex::Female,
        _ => Sex::Unknown,
    })
}

pub(crate) async fn laubfrosch(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
) -> Result<(), anyhow::Error> {
    bot.send_poll(
        dialoge.chat_id(),
        "Select whichever applies",
        [
            "Schallblase/Quakt",
            "Kehle gelbbraun/braungrau",
            "Keine Schallblase",
            "Kehle weißlich/hellgrau",
            "Unsicher",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn laubfrosch_answered(poll: Poll) -> Result<Sex, anyhow::Error> {
    Ok(match poll.selected_index() {
        -1 => bail!("Cannot unselect options"),
        0..2 => Sex::Male,
        2..4 => Sex::Female,
        _ => Sex::Unknown,
    })
}
