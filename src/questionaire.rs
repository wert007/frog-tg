use anyhow::bail;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, types::MessageId};

use crate::{
    CompleteWalk, LastLocation, PartialFrog, PollExt, Sex, State, if_is_relevant,
    polls::{MainQuestion, QuestionaireQuestion},
};

mod sex;

#[derive(Debug, Clone, Copy)]
pub enum Species {
    Molch,
    Frog,
    Toad,
}

#[derive(Debug, Default, Clone)]
pub struct QuestionaireFrogName {
    pub last_message_id: MessageId,
    pub species: Option<Species>,
}

impl QuestionaireFrogName {
    pub fn new(last_message_id: MessageId) -> Self {
        Self {
            last_message_id,
            species: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct QuestionaireSex {
    pub frog: PartialFrog,
    pub result: Option<Sex>,
}
impl QuestionaireSex {
    pub(crate) fn new(frog: PartialFrog) -> Self {
        Self { frog, result: None }
    }
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
        "" => {
            dialoge.update(State::WalkStarted { walk }).await?;
            return Ok(());
        }
        _ => unreachable!(),
    };
    questionaire.species = Some(species);
    let question = match species {
        Species::Molch => QuestionaireQuestion::ItIsAMolch,
        Species::Frog => QuestionaireQuestion::ItIsAFrog,
        Species::Toad => QuestionaireQuestion::ItIsAToad,
    };
    questionaire.last_message_id = question.ask(bot, dialoge.chat_id()).await?;
    dialoge
        .update(State::QuestionaireFrogName { walk, questionaire })
        .await?;
    Ok(())
}

pub(crate) async fn found_frog_name(
    bot: Bot,
    dialoge: Dialogue<crate::State, InMemStorage<crate::State>>,
    last_location: LastLocation,
    (walk, questionaire): (CompleteWalk, QuestionaireFrogName),
    poll: Poll,
) -> anyhow::Result<()> {
    let name = match (
        questionaire
            .species
            .expect("Can only go here, if we have a species"),
        poll.selected_index(),
    ) {
        (_, -1) => {
            bot.delete_message(dialoge.chat_id(), questionaire.last_message_id)
                .await?;
            return Ok(());
        }
        (Species::Molch, 0) => "Kammmolch",
        (Species::Molch, 1) => "Bergmolch",
        (Species::Molch, 2) => "Teichmolch",
        (Species::Molch, 3) => "Molch",
        (Species::Toad, 0 | 1) => "Knoblauchkröte",
        (Species::Toad, 2 | 3) => "Erdkröte",
        (Species::Toad, 4) => "Kröte",
        (Species::Frog, 0) => "Grünfrosch",
        (Species::Frog, 1) => "Springfrosch",
        (Species::Frog, 2) => "Grasfrosch",
        (Species::Frog, 3) => "Frosch",
        (_, _) => unreachable!(),
    };
    let last_message_id = MainQuestion::AskForSex(name.into())
        .ask(bot, dialoge.chat_id())
        .await?;
    dialoge
        .update(State::FrogIdentified {
            frog: crate::PartialFrog {
                name: name.into(),
                gps_location: if_is_relevant(last_location),
                ..Default::default()
            },
            walk,
            last_message_id,
        })
        .await?;
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
    (walk, mut questionaire): (CompleteWalk, QuestionaireSex),
    poll: Poll,
) -> anyhow::Result<()> {
    let chat_id = dialoge.chat_id();
    let sex = match questionaire.frog.name.as_str() {
        "Erdkröte" => sex::erdkroete_answered(poll).await,
        "Knoblauchkröte" => sex::knoblauchkroete_answered(poll).await,
        "Springfrosch" => sex::springfrosch_answered(poll).await,
        "Grünfrosch" => sex::gruenfrosch_answered(poll).await,
        "Grasfrosch" => sex::grasfrosch_answered(poll).await,
        "Laubfrosch" => sex::laubfrosch_answered(poll).await,
        "Teichmolch" => sex::teichmolch_answered(poll).await,
        "Bergmolch" => sex::bergmolch_answered(poll).await,
        "Kammmolch" => sex::kammmolch_answered(poll).await,
        _ => bail!("Unhandled species {}!", questionaire.frog.name),
    }?;
    questionaire.frog.sex = Some(sex);
    let last_message_id = MainQuestion::WhereAreYou.ask(bot, chat_id).await?;
    dialoge
        .update(State::FrogIdentified {
            frog: questionaire.frog,
            walk,
            last_message_id,
        })
        .await?;
    Ok(())
}
