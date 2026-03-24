use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, atomic::AtomicBool},
};

use chrono::{DateTime, Local};
use parking_lot::Mutex;
use teloxide::{
    dispatching::{
        DpHandlerDescription,
        dialogue::{GetChatId, InMemStorage},
    },
    dptree::di::Injectable,
    prelude::*,
    types::{Location, MessageId, UpdateKind},
};

use crate::state::State;

pub type R = anyhow::Result<()>;
pub type DialogueState = Dialogue<State, InMemStorage<State>>;

pub trait PollExt {
    fn selected(&self) -> &str;
    fn selected_index(&self) -> isize;
}

impl PollExt for Poll {
    fn selected(&self) -> &str {
        self.options
            .iter()
            .filter(|o| o.voter_count > 0)
            .map(|o| o.text.as_str())
            .next()
            .unwrap_or_default()
    }

    fn selected_index(&self) -> isize {
        self.options
            .iter()
            .enumerate()
            .filter(|(_, o)| o.voter_count > 0)
            .map(|(i, _)| i as isize)
            .next()
            .unwrap_or(-1)
    }
}

pub fn is_command<'a>(cmd: &'static str) -> Handler<'a, R, DpHandlerDescription> {
    dptree::filter(move |m: Message| {
        m.text()
            .is_some_and(|t| &t.trim()[..1] == "/" && &t.trim()[1..] == cmd)
    })
}

pub fn if_is_command<'a, FnArgs, F: Injectable<R, FnArgs> + Send + Sync + 'a>(
    cmd: &'static str,
    handler: F,
) -> Handler<'a, R, DpHandlerDescription> {
    is_command(cmd).endpoint(handler)
}

#[derive(Debug, Clone)]
pub struct Mode(Arc<AtomicBool>);

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub enum MessageClassification {
    #[default]
    None,
    Weather,
    Frog(usize),
    DeadFrog(usize),
}

#[derive(Debug, Clone, Default)]
pub struct SentMessage {
    context: Arc<parking_lot::Mutex<HashMap<MessageId, MessageClassification>>>,
    history: Arc<parking_lot::Mutex<Vec<MessageId>>>,
}

impl SentMessage {
    pub fn clear_history(&self) {
        self.history.lock().clear();
    }
    pub fn clear(&self) {
        self.context.lock().clear();
        self.history.lock().clear();
    }

    pub fn add_weather(&self, id: MessageId) {
        self.add_to_history(id);
        self.context
            .lock()
            .insert(id, MessageClassification::Weather);
    }

    pub fn add_frog(&self, id: MessageId, index: usize) {
        self.add_to_history(id);
        self.context
            .lock()
            .insert(id, MessageClassification::Frog(index));
    }

    pub fn add_dead_frog(&self, id: MessageId, index: usize) {
        self.add_to_history(id);
        self.context
            .lock()
            .insert(id, MessageClassification::DeadFrog(index));
    }

    pub fn get(&self, id: MessageId) -> Option<MessageClassification> {
        self.context.lock().get(&id).copied()
    }

    pub fn add_to_history(&self, id: MessageId) {
        self.history.lock().push(id);
    }

    pub async fn go_back(
        &self,
        bot: Bot,
        dialoge: Dialogue<State, InMemStorage<State>>,
    ) -> anyhow::Result<()> {
        let Some(id) = self.history.lock().pop() else {
            eprintln!("No going back possible!");
            return Ok(());
        };
        let r = bot.delete_message(dialoge.chat_id(), id).await?;
        dbg!(r);
        let previous = dialoge.get_or_default().await?.go_back();
        dialoge.update(previous).await?;
        Ok(())
    }
}

impl Mode {
    pub fn as_path(&self) -> &'static str {
        if self.is_debug() {
            "debug-walks"
        } else {
            "walks"
        }
    }

    pub fn create_debug() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }
    pub fn create_release() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn is_debug(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct LastLocation(Arc<Mutex<TimedLocation>>);

impl Default for LastLocation {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(TimedLocation::error())))
    }
}

struct LastLocationUpdater;

impl Injectable<R, (Location, LastLocation)> for LastLocationUpdater {
    fn inject<'a>(&'a self, container: &'a DependencyMap) -> dptree::di::CompiledFn<'a, R> {
        Arc::new(move || {
            let tl: Arc<LastLocation> = container.get();
            let l: Arc<Location> = container.get();
            let x = async move {
                tl.update(&l);
                Ok(())
            };
            Box::pin(x)
        })
    }

    fn input_types() -> std::collections::BTreeSet<dptree::Type> {
        BTreeSet::from_iter(vec![
            dptree::Type::of::<Location>(),
            dptree::Type::of::<LastLocation>(),
        ])
    }
}

impl LastLocation {
    fn update(&self, l: &Location) {
        let mut tl = self.0.lock();
        tl.latitude = l.latitude;
        tl.longitude = l.longitude;
        tl.time = Local::now();
    }

    pub fn as_location(&self) -> Option<TimedLocation> {
        let last_location = self.0.lock();
        if last_location.latitude.is_nan() || last_location.longitude.is_nan() {
            None
        } else if (Local::now() - last_location.time).num_minutes() > 5 {
            None
        } else {
            Some(last_location.clone())
        }
    }

    pub fn update_handler() -> Handler<'static, R, DpHandlerDescription> {
        Update::filter_edited_message()
            .filter_map(|u: Update| {
                if let UpdateKind::EditedMessage(m) = u.kind {
                    m.location().cloned()
                } else {
                    None
                }
            })
            .endpoint(LastLocationUpdater)
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TimedLocation {
    latitude: f64,
    longitude: f64,
    time: DateTime<Local>,
}

impl TimedLocation {
    pub fn error() -> Self {
        Self {
            latitude: f64::NAN,
            longitude: f64::NAN,
            time: DateTime::UNIX_EPOCH.with_timezone(&Local),
        }
    }
}

#[derive(Clone)]
#[allow(unused)]
pub struct UpdateWithSuppliedChatId(Update, ChatId);

impl UpdateWithSuppliedChatId {
    pub fn ensure_id(update: Update) -> Self {
        let id = update.chat_id().unwrap_or_else(|| match &update.kind {
            teloxide::types::UpdateKind::PollAnswer(poll_answer) => poll_answer
                .voter
                .chat()
                .map(|c| c.id)
                .or(poll_answer.voter.user().map(|u| ChatId::from(u.id)))
                .unwrap(),
            err => todo!("{err:#?}"),
        });
        UpdateWithSuppliedChatId(update, id)
    }
}

impl GetChatId for UpdateWithSuppliedChatId {
    fn chat_id(&self) -> Option<ChatId> {
        Some(self.1)
    }
}
