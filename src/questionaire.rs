use anyhow::bail;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, types::InputPollOption};

use crate::{CompleteWalk, PollExt, State, ask_sex};

#[derive(Debug, Clone, Copy)]
pub enum Species {
    Molch,
    Frog,
    Toad,
}

#[derive(Debug, Default, Clone)]
pub struct QuestionaireFrogName {
    pub species: Option<Species>,
}

pub(crate) async fn start(
    bot: Bot,
    dialoge: Dialogue<crate::State, InMemStorage<crate::State>>,
) -> anyhow::Result<()> {
    bot.send_poll(
        dialoge.chat_id(),
        "Is it a frog, toad or a Molch?",
        [
            "Molch (Has Tail)",
            "Toad (Has Wards)",
            "Frog (No Wards)",
            "Unsure",
        ]
        .map(InputPollOption::new),
    )
    .await?;
    Ok(())
}

pub(crate) async fn found_species(
    bot: Bot,
    dialoge: Dialogue<crate::State, InMemStorage<crate::State>>,
    (walk, mut questionaire): (CompleteWalk, QuestionaireFrogName),
    poll: Poll,
) -> anyhow::Result<()> {
    let species = match poll.selected() {
        "Molch (Has Tail)" => Species::Molch,
        "Toad (Has Wards)" => Species::Toad,
        "Frog (No Wards)" => Species::Frog,
        "Unsure" => bail!("TODO"),
        _ => unreachable!(),
    };
    questionaire.species = Some(species);
    dialoge
        .update(State::QuestionaireFrogName { walk, questionaire })
        .await?;
    let (question, options): (&'static str, &'static [&'static str]) = match species {
        Species::Molch => (
            "Check its skin now!",
            &[
                "There are white dots",
                "No dark markings on the bottom side",
                "Otherwise it is a Teichmolch",
                "Unsure",
            ],
        ),
        Species::Frog => (
            "Check its Skin and its Nose!",
            &[
                "It has markings on its back",
                "Its nose is pointy",
                "Its nose is more stump",
                "Unsure",
            ],
        ),
        Species::Toad => (
            "Check its Skin now!",
            &[
                "Has markings on its back",
                "Red marks",
                "Has dark markings",
                "Has a lot of wards",
                "Unsure",
            ],
        ),
    };
    bot.send_poll(
        dialoge.chat_id(),
        question,
        options
            .into_iter()
            .copied()
            .map(InputPollOption::new)
            .collect::<Vec<_>>(),
    )
    .await?;
    Ok(())
}

pub(crate) async fn found_frog_name(
    bot: Bot,
    dialoge: Dialogue<crate::State, InMemStorage<crate::State>>,
    (walk, questionaire): (CompleteWalk, QuestionaireFrogName),
    poll: Poll,
) -> anyhow::Result<()> {
    let name = match (
        questionaire
            .species
            .expect("Can only go here, if we have a species"),
        poll.selected_index(),
    ) {
        (Species::Molch, 0) => "Kammmolch",
        (Species::Molch, 1) => "Bergmolch",
        (Species::Molch, 2) => "Teichmolch",
        (Species::Molch, 3) => bail!("TODO"),
        (Species::Toad, 0 | 1) => "Knoblauchkröte",
        (Species::Toad, 2 | 3) => "Erdkröte",
        (Species::Toad, 4) => bail!("TODO"),
        (Species::Frog, 0) => "Grünfrosch",
        (Species::Frog, 1) => "Springfrosch",
        (Species::Frog, 2) => "Grasfrosch",
        (Species::Frog, 3) => bail!("TODO"),
        (_, _) => unreachable!(),
    };
    dialoge
        .update(State::FrogIdentified {
            name: name.into(),
            walk,
        })
        .await?;
    ask_sex(bot, name, dialoge.chat_id()).await?;
    Ok(())
}
