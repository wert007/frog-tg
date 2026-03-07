use anyhow::bail;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, types::InputPollOption};

use crate::{CompleteWalk, PollExt, Sex, State, ask_for_location, ask_sex};

mod sex;

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
#[derive(Debug, Default, Clone)]
pub struct QuestionaireSex {
    pub name: String,
    pub result: Option<Sex>,
}
impl QuestionaireSex {
    pub(crate) fn new(name: String) -> Self {
        Self { name, result: None }
    }
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

pub(crate) async fn start_sex(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
    name: &str,
) -> anyhow::Result<()> {
    match name {
        "Erdkröte" => sex::erdkroete(bot, dialoge).await,
        "Knoblauchkröte" => sex::knoblauchkroete(bot, dialoge).await,
        "Springfrosch" => sex::springfrosch(bot, dialoge).await,
        "Grünfrosch" => sex::gruenfrosch(bot, dialoge).await,
        "Grasfrosch" => sex::grasfrosch(bot, dialoge).await,
        "Laubfrosch" => sex::laubfrosch(bot, dialoge).await,
        "Teichmolch" => sex::teichmolch(bot, dialoge).await,
        "Bergmolch" => sex::bergmolch(bot, dialoge).await,
        "Kammmolch" => sex::kammmolch(bot, dialoge).await,
        _ => bail!("Unhandled species {name}!"),
    }
}

pub(crate) async fn found_sex(
    bot: Bot,
    dialoge: Dialogue<State, InMemStorage<State>>,
    (walk, questionaire): (CompleteWalk, QuestionaireSex),
    poll: Poll,
) -> anyhow::Result<()> {
    let chat_id = dialoge.chat_id();
    let sex = match questionaire.name.as_str() {
        "Erdkröte" => sex::erdkroete_answered(poll).await,
        "Knoblauchkröte" => sex::knoblauchkroete_answered(poll).await,
        "Springfrosch" => sex::springfrosch_answered(poll).await,
        "Grünfrosch" => sex::gruenfrosch_answered(poll).await,
        "Grasfrosch" => sex::grasfrosch_answered(poll).await,
        "Laubfrosch" => sex::laubfrosch_answered(poll).await,
        "Teichmolch" => sex::teichmolch_answered(poll).await,
        "Bergmolch" => sex::bergmolch_answered(poll).await,
        "Kammmolch" => sex::kammmolch_answered(poll).await,
        _ => bail!("Unhandled species {}!", questionaire.name),
    }?;
    dialoge
        .update(State::FrogIdentifiedSex {
            name: questionaire.name,
            walk,
            sex,
        })
        .await?;
    ask_for_location(bot, chat_id).await?;
    Ok(())
}
