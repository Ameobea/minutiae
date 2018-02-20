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
use cell::{Cell, CellState};
use engine::parallel::ActionExecutor;
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction, OwnedAction};
use container::EntityContainer;
use server::ServerLogic;

use super::{Server, ServerMessage, ClientMessage};

/// Helper trait to contain some of the massive spam caused in trait definitions.  This requires that implementors are
pub trait HybParam : Send + Serialize + for<'de> Deserialize<'de> {}

pub type HybridServerSnapshot<
    C: CellState, E: EntityState<C>, M: MutEntityState
> = (Vec<Cell<C>>, EntityContainer<C, E, M>);

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "C: for<'d> Deserialize<'d>")]
pub enum HybridServerMessageContents<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U>
> {
    Snapshot(HybridServerSnapshot<C, E, M>),
    Event(Vec<V>),
    __phantom_c(PhantomData<C>),
    __phantom_e(PhantomData<E>),
    __phantom_m(PhantomData<M>),
    __phantom_ca(PhantomData<CA>),
    __phantom_ea(PhantomData<EA>),
    __phantom_u(PhantomData<U>),
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
#[serde(bound = "C: for<'d> Deserialize<'d>")]
pub struct HybridServerMessage<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U> + HybParam
> {
    pub seq: u32,
    pub contents: HybridServerMessageContents<C, E, M, CA, EA, U, V>,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
    __phantom_m: PhantomData<M>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
    __phantom_v: PhantomData<V>,
}

impl<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U> + HybParam
> Debug for HybridServerMessage<C, E, M, CA, EA, U, V> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "HybridServerMessage {{seq: {}, contents: {{..}} }}", self.seq)
    }
}

impl<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U> + HybParam
> PartialEq for HybridServerMessage<C, E, M, CA, EA, U, V> {
    fn eq(&self, rhs: &Self) -> bool {
        self.seq == rhs.seq /*&& self.contents == rhs.seq*/
    }
}

impl<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U> + HybParam
> Eq for HybridServerMessage<C, E, M, CA, EA, U, V> {}

impl<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U> + HybParam
> PartialOrd for HybridServerMessage<C, E, M, CA, EA, U, V> {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.seq.cmp(&rhs.seq))
    }
}

impl<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U> + HybParam
> Ord for HybridServerMessage<C, E, M, CA, EA, U, V> {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.seq.cmp(&rhs.seq)
    }
}

impl<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M> + HybParam,
    V: Event<C, E, M, CA, EA, U> + HybParam
> ServerMessage<HybridServerSnapshot<C, E, M>> for HybridServerMessage<C, E, M, CA, EA, U, V> {
    fn get_seq(&self) -> u32 { self.seq }

    fn get_snapshot(self) -> Result<HybridServerSnapshot<C, E, M>, Self> {
        match self.contents {
            HybridServerMessageContents::Snapshot(snap) => Ok(snap),
            HybridServerMessageContents::Event{ .. } => Err(self),
            _ => unreachable!(),
        }
    }
}

impl<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U> + HybParam
> HybridServerMessage<C, E, M, CA, EA, U, V> {
    pub fn new(
        seq: u32,
        contents: HybridServerMessageContents<C, E, M, CA, EA, U, V>
    ) -> Self {
        HybridServerMessage {
            seq, contents,
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
            __phantom_m: PhantomData,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
            __phantom_v: PhantomData,
        }
    }
}

pub struct HybridServer<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M>,
    V: Event<C, E, M, CA, EA, U> + HybParam,
    F: Fn(
        &mut U,
        &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>]
    ) -> Option<Vec<V>>,
> {
    pub pending_snapshot: bool,
    pub seq: Arc<AtomicU32>,
    pub event_generator: F,
    pub self_actions: Arc<RwLock<Vec<OwnedAction<C, E, CA, EA>>>>,
    pub cell_actions: Arc<RwLock<Vec<OwnedAction<C, E, CA, EA>>>>,
    pub entity_actions: Arc<RwLock<Vec<OwnedAction<C, E, CA, EA>>>>,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
    __phantom_m: PhantomData<M>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
    __phantom_v: PhantomData<V>,
    __phantom_u: PhantomData<U>,
}

impl<
    C: CellState + HybParam,
    E: EntityState<C> + HybParam,
    M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam,
    U: Universe<C, E, M> + HybParam,
    V: Event<C, E, M, CA, EA, U> + HybParam + Clone,
    F: Fn(
        &mut U,
        &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>]
    ) -> Option<Vec<V>>
> ServerLogic<
    C, E, M, CA, EA, HybridServerMessage<C, E, M, CA, EA, U, V>, HybridClientMessage, U
> for HybridServer<C, E, M, CA, EA, U, V, F> {
    fn tick(
        &mut self, universe: &mut U
    ) -> Option<Vec<HybridServerMessage<C, E, M, CA, EA, U, V>>> {
        let mut pending_messages: Option<Vec<V>> = None;
        if self.pending_snapshot {
            self.pending_snapshot = false;
            pending_messages = Some(vec![]);
        }

        self.seq.fetch_add(1, AtomicOrdering::Relaxed);

        // use the user-defined logic to get the events to pass through to the clients.  The internal action buffers
        // are used as the sources for this.
        // TODO: Look into recycling the buffers rather than reallocating if we're going to keep this system (we shouldn't)
        let merged_events: Vec<V> = match (self.event_generator)(
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
        server: &mut Server<
            C, E, M, CA, EA, HybridServerMessage<C, E, M, CA, EA, U, V>, HybridClientMessage, U, Self
        >,
        message: &HybridClientMessage
    ) -> Option<Vec<HybridServerMessage<C, E, M, CA, EA, U, V>>> {
        match message.contents {
            HybridClientMessageContents::RequestSnapshot => {
                // don't have access to the universe, so we really can't send an accurate snapshot.  Instead,
                // set a flag to send the message in the future.
                server.logic.pending_snapshot = true;
                None
            },
        }
    }
}

impl<
    C: CellState + HybParam + 'static,
    E: EntityState<C> + HybParam + 'static,
    M: MutEntityState + HybParam + 'static,
    CA: CellAction<C> + HybParam + 'static,
    EA: EntityAction<C, E> + HybParam + 'static,
    U: Universe<C, E, M> + 'static,
    V: Event<C, E, M, CA, EA, U> + HybParam + 'static,
    F: Fn(
        &mut U,
        &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>]
    ) -> Option<Vec<V>>
> HybridServer<C, E, M, CA, EA, U, V, F> where OwnedAction<C, E, CA, EA>:Clone {
    /// Takes the action handlers for the engine and hooks them, getting an intermediate view of the actions
    /// so that they can be transmitted to the client before handling them on the client side.
    pub fn hook_handler(
        action_executor: fn(&mut U, &[OwnedAction<C, E, CA, EA>],
            &[OwnedAction<C, E, CA, EA>],
            &[OwnedAction<C, E, CA, EA>]
        ),
        event_generator: F
    ) -> (ActionExecutor<C, E, CA, EA, U>, Self) {
        let hybrid_server = HybridServer::new(event_generator);
        // create copies of the buffers so that we can write to them from outside
        let self_action_buf = hybrid_server.self_actions.clone();
        let cell_action_buf = hybrid_server.cell_actions.clone();
        let entity_action_buf = hybrid_server.entity_actions.clone();

        // create an action handler that can be passed back which handles our logic as well as the original logic
        let hooked_handler = move |
            universe: &mut U, self_actions: &[OwnedAction<C, E, CA, EA>],
            cell_actions: &[OwnedAction<C, E, CA, EA>], entity_actions: &[OwnedAction<C, E, CA, EA>]
        | {
            // copy the actions into our internal buffers so that they can be used later once we have access
            // to the universe to create our final events.
            *&mut *self_action_buf.write().unwrap() = self_actions.into();
            *&mut *cell_action_buf.write().unwrap() = cell_actions.into();
            *&mut *entity_action_buf.write().unwrap() = entity_actions.into();

            // Call the consumed action handler and actually mutate the universe.
            action_executor(universe, self_actions, cell_actions, entity_actions);
        };

        (Box::new(hooked_handler), hybrid_server)
    }

    pub fn new(event_generator: F) -> Self {
        HybridServer {
            pending_snapshot: false,
            seq: Arc::new(AtomicU32::new(0)),
            event_generator,
            self_actions: Arc::new(RwLock::new(Vec::new())),
            cell_actions: Arc::new(RwLock::new(Vec::new())),
            entity_actions: Arc::new(RwLock::new(Vec::new())),
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
            __phantom_m: PhantomData,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
            __phantom_v: PhantomData,
            __phantom_u: PhantomData,
        }
    }
}

/// Defines an event that takes place in the universe.  Given the event, the hybrid client must be able to
/// apply it to the universe.
pub trait Event<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    U: Universe<C, E, M>
> : Serialize + for<'u> Deserialize<'u> {
    fn apply(&self, universe: &mut U);
}
