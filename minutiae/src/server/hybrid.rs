//! Hybrid clients maintain the full universe state internally but only take action in response to events sent from
//! the server.  These events can be more complicated than single `EntityAction`s and can affect more than one pixel
//! at a time, such as translating every entity in the universe or removing entities en-masse.

use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use serde::{Serialize, Deserialize};
use uuid::Uuid;

use universe::Universe;
use cell::CellState;
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction, OwnedAction};

use super::{ServerMessage, ServerLogic, Tys, ClientMessage};

/// Helper trait to contain some of the massive spam caused in trait definitions.  This requires that implementors are
pub trait HybParam : Send + Clone + Serialize + for<'de> Deserialize<'de> {}

/// Defines an event that takes place in the universe.  Given the event, the hybrid client must be able to
/// apply it to the universe.
pub trait Event<T: Tys> : Serialize + for<'u> Deserialize<'u> {
    fn apply(&self, universe: &mut T::Snapshot);
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "T::C: for<'d> Deserialize<'d>, T::I: ::serde::Serialize + for<'d> Deserialize<'d>, T::Snapshot: ::serde::Serialize + for<'d> Deserialize<'d>")]
pub enum HybridServerMessageContents<T: Tys> {
    Snapshot(T::Snapshot),
    Event(Vec<T::V>),
    __phantom_T(PhantomData<T>),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HybridClientMessage {
    pub client_id: Uuid,
    pub contents: HybridClientMessageContents,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HybridClientMessageContents {
    RequestSnapshot,
}

impl ClientMessage for HybridClientMessage {
    fn get_client_id(&self) -> Uuid { self.client_id }

    fn create_snapshot_request(client_id: Uuid) -> Self {
        HybridClientMessage {
            client_id,
            contents: HybridClientMessageContents::RequestSnapshot,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound = "T::C: for<'d> Deserialize<'d>, T::I: ::serde::Serialize + for<'d> Deserialize<'d>, T::Snapshot: ::serde::Serialize + for<'d> Deserialize<'d>")]
pub struct HybridServerMessage<T: Tys> where T::Snapshot: Clone, T::V: Clone {
    pub seq: u32,
    pub contents: HybridServerMessageContents<T>,
}

impl<T: Tys> Debug for HybridServerMessage<T> where T::Snapshot: Clone, T::V: Clone {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "HybridServerMessage {{seq: {}, contents: {{..}} }}", self.seq)
    }
}

impl<T: Tys> PartialEq for HybridServerMessage<T> where T::Snapshot: Clone, T::V: Clone {
    fn eq(&self, rhs: &Self) -> bool {
        self.seq == rhs.seq /*&& self.contents == rhs.seq*/
    }
}

impl<T: Tys> Eq for HybridServerMessage<T> where T::Snapshot: Clone, T::V: Clone {}

impl<T: Tys> PartialOrd for HybridServerMessage<T> where T::Snapshot: Clone, T::V: Clone {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.seq.cmp(&rhs.seq))
    }
}

impl<T: Tys> Ord for HybridServerMessage<T> where T::Snapshot: Clone, T::V: Clone {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.seq.cmp(&rhs.seq)
    }
}

impl<T: Tys> ServerMessage<T::Snapshot> for HybridServerMessage<T> where
    T::I: Serialize + for<'de> Deserialize<'de>,
    T::V: Clone + Send,
    T::U: Serialize + for<'de> Deserialize<'de> + Clone + Send,
    T::Snapshot: Serialize + for<'de> Deserialize<'de> + Clone + Send,
{
    fn get_seq(&self) -> u32 { self.seq }

    fn is_snapshot(&self) -> bool {
        if let HybridServerMessageContents::Snapshot(_) = self.contents {
            true
        } else {
            false
        }
    }

    fn get_snapshot(self) -> Option<T::Snapshot> {
        match self.contents {
            HybridServerMessageContents::Snapshot(snap) => Some(snap),
            HybridServerMessageContents::Event{ .. } => None,
            _ => unreachable!(),
        }
    }
}

impl<T: Tys> HybridServerMessage<T> where T::Snapshot: Clone, T::V: Clone {
    pub fn new(
        seq: u32,
        contents: HybridServerMessageContents<T>
    ) -> Self {
        HybridServerMessage {
            seq,
            contents,
        }
    }
}

#[derive(Clone)]
pub struct HybridServer<T: Tys> {
    pub pending_snapshot: bool,
    pub seq: Arc<AtomicU32>,
    pub event_generator: fn(
        &mut T::U,
        &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
        &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
        &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>]
    ) -> Option<Vec<T::V>>,
    pub self_actions: Arc<RwLock<Vec<OwnedAction<T::C, T::E, T::CA, T::EA, T::I>>>>,
    pub cell_actions: Arc<RwLock<Vec<OwnedAction<T::C, T::E, T::CA, T::EA, T::I>>>>,
    pub entity_actions: Arc<RwLock<Vec<OwnedAction<T::C, T::E, T::CA, T::EA, T::I>>>>,
}

impl<T: Tys<ServerMessage=HybridServerMessage<T>>> ServerLogic<T, HybridClientMessage> for HybridServer<T> where
    T::C: Clone,
    T::E: Clone,
    T::M: Clone,
    T::I: Serialize + for<'de> Deserialize<'de> + Clone,
    T::U: Serialize + for<'de> Deserialize<'de> + Clone + Send,
    T::CA: Clone,
    T::EA: Clone,
    T::V: Clone + Send,
    T::Snapshot: Serialize + for<'de> Deserialize<'de> + Clone + Send,
{
    fn tick(
        &mut self, universe: &mut T::U
    ) -> Option<Vec<HybridServerMessage<T>>> {
        let mut pending_messages: Option<Vec<T::V>> = None;
        if self.pending_snapshot {
            self.pending_snapshot = false;
            pending_messages = Some(vec![]);
        }

        self.seq.fetch_add(1, AtomicOrdering::Relaxed);

        // use the user-defined logic to get the events to pass through to the clients.  The internal action buffers
        // are used as the sources for this.
        // TODO: Look into recycling the buffers rather than reallocating if we're going to keep this system (we shouldn't)
        let merged_events: Vec<T::V> = match (self.event_generator)(
            universe,
            &*self.self_actions.read().unwrap(),
            &*self.cell_actions.read().unwrap(),
            &*self.entity_actions.read().unwrap()
        ) {
            Some(events) => {
                match pending_messages {
                    Some(pending) => {
                        [&pending[..], &events[..]].concat()
                    },
                    None => events,
                }
            },
            None => pending_messages.unwrap_or(vec![]),
        };

        Some(vec![HybridServerMessage::new(
            self.seq.load(AtomicOrdering::Relaxed),
            HybridServerMessageContents::Event(merged_events)
        )])
    }

    fn handle_client_message(
        &mut self,
        _seq: Arc<AtomicU32>,
        message: &HybridClientMessage
    ) -> Option<Vec<HybridServerMessage<T>>> {
        match message.contents {
            HybridClientMessageContents::RequestSnapshot => {
                // don't have access to the universe, so we really can't send an accurate snapshot.  Instead,
                // set a flag to send the message in the future.
                self.pending_snapshot = true;
                None
            },
        }
    }
}

impl<T: Tys> HybridServer<T> where OwnedAction<T::C, T::E, T::CA, T::EA, T::I> : Clone {
    /// Takes the action handlers for the engine and hooks them, getting an intermediate view of the actions
    /// so that they can be transmitted to the client before handling them on the client side.
    pub fn hook_handler(
        action_executor: fn(&mut T::U, &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>]
        ),
        event_generator: fn(
            &mut T::U,
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>]
        ) -> Option<Vec<T::V>>
    ) -> (
        impl Fn(
            &mut T::U,
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>]
        ),
        Self,
    ) {
        let hybrid_server = HybridServer::new(event_generator);
        // create copies of the buffers so that we can write to them from outside
        let self_action_buf = hybrid_server.self_actions.clone();
        let cell_action_buf = hybrid_server.cell_actions.clone();
        let entity_action_buf = hybrid_server.entity_actions.clone();

        // create an action handler that can be passed back which handles our logic as well as the original logic
        let hooked_handler = move |
            universe: &mut T::U,
            self_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            cell_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            entity_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>]
        | {
            // copy the actions into our internal buffers so that they can be used later once we have access
            // to the universe to create our final events.
            *&mut *self_action_buf.write().unwrap() = self_actions.into();
            *&mut *cell_action_buf.write().unwrap() = cell_actions.into();
            *&mut *entity_action_buf.write().unwrap() = entity_actions.into();

            // Call the consumed action handler and actually mutate the universe.
            action_executor(universe, self_actions, cell_actions, entity_actions);
        };

        (hooked_handler, hybrid_server)
    }

    pub fn new(
        event_generator: fn(
            &mut T::U,
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>]
        ) -> Option<Vec<T::V>>
    ) -> Self {
        HybridServer {
            pending_snapshot: false,
            seq: Arc::new(AtomicU32::new(0)),
            event_generator,
            self_actions: Arc::new(RwLock::new(Vec::new())),
            cell_actions: Arc::new(RwLock::new(Vec::new())),
            entity_actions: Arc::new(RwLock::new(Vec::new())),
        }
    }
}
