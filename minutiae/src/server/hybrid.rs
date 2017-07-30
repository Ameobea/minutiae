//! Hybrid clients maintain the full universe state internally but only take action in response to events sent from
//! the server.  These events can be more complicated than single `EntityAction`s and can affect more than one pixel
//! at a time, such as translating every entity in the universe or removing entities en-masse.

use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;
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

pub type HybridServerSnapshot<C: CellState, E: EntityState<C>, M: MutEntityState> = (Vec<Cell<C>>, EntityContainer<C, E, M>);

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "C: for<'d> Deserialize<'d>")]
pub enum HybridServerMessageContents<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA>
> {
    Snapshot(HybridServerSnapshot<C, E, M>),
    Event {
        self_actions: Vec<V>,
        cell_actions: Vec<V>,
        entity_cations: Vec<V>,
    },
    __phantom_c(PhantomData<C>),
    __phantom_e(PhantomData<E>),
    __phantom_m(PhantomData<M>),
    __phantom_ca(PhantomData<CA>),
    __phantom_ea(PhantomData<EA>),
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
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> {
    pub seq: u32,
    pub contents: HybridServerMessageContents<C, E, M, CA, EA, V>,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
    __phantom_m: PhantomData<M>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
    __phantom_v: PhantomData<V>,
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> Debug for HybridServerMessage<C, E, M, CA, EA, V> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "HybridServerMessage {{seq: {}, contents: {{..}} }}", self.seq)
    }
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> PartialEq for HybridServerMessage<C, E, M, CA, EA, V> {
    fn eq(&self, rhs: &Self) -> bool {
        self.seq == rhs.seq /*&& self.contents == rhs.seq*/
    }
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> Eq for HybridServerMessage<C, E, M, CA, EA, V> {}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> PartialOrd for HybridServerMessage<C, E, M, CA, EA, V> {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.seq.cmp(&rhs.seq))
    }
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> Ord for HybridServerMessage<C, E, M, CA, EA, V> {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.seq.cmp(&rhs.seq)
    }
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam, CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> ServerMessage<HybridServerSnapshot<C, E, M>> for HybridServerMessage<C, E, M, CA, EA, V> {
    fn get_seq(&self) -> u32 { self.seq }

    fn get_snapshot(self) -> Result<HybridServerSnapshot<C, E, M>, Self> {
        match self.contents {
            HybridServerMessageContents::Snapshot(snap) => Ok(snap),
            HybridServerMessageContents::Event{ .. } => Err(self),
            _ => unreachable!(),
        }
    }
}

pub struct HybridServer<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam,
> {
    pub pending_snapshot: bool,
    pub seq: Arc<AtomicU32>,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
    __phantom_m: PhantomData<M>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
    __phantom_v: PhantomData<V>,
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam,
> ServerLogic<C, E, M, CA, EA, HybridServerMessage<C, E, M, CA, EA, V>, HybridClientMessage> for HybridServer<C, E, M, CA, EA, V> {
    fn tick(&mut self, universe: &mut Universe<C, E, M, CA, EA>) -> Option<HybridServerMessage<C, E, M, CA, EA, V>> {
        if self.pending_snapshot {
            self.pending_snapshot = false;
            unimplemented!(); // TODO
        }
        self.seq.fetch_add(1, AtomicOrdering::Relaxed);
        unimplemented!(); // TODO
    }

    fn handle_client_message(
        server: &mut Server<C, E, M, CA, EA, HybridServerMessage<C, E, M, CA, EA, V>, HybridClientMessage, Self>,
        message: &HybridClientMessage
    ) -> Option<Vec<HybridServerMessage<C, E, M, CA, EA, V>>> {
        match message.contents {
            HybridClientMessageContents::RequestSnapshot => {
                // don't have access to the universe, so we really can't send an accurate snapshot.  Instead,
                // set a flag to send the message in the future.
                server.logic.pending_snapshot = true;
                None
            },
            _ => unreachable!(),
        }
    }
}

impl<
    C: CellState + HybParam + 'static, E: EntityState<C> + HybParam + 'static, M: MutEntityState + HybParam + 'static,
    CA: CellAction<C> + HybParam + 'static, EA: EntityAction<C, E> + HybParam + 'static, V: Event<C, E, M, CA, EA> + HybParam + 'static,
> HybridServer<C, E, M, CA, EA, V> {
    /// Takes the action handlers for the engine and hooks them, getting an intermediate view of the actions
    /// so that they can be transmitted to the client before handling them on the client side.
    pub fn hook_handler(
        action_executor: fn(&mut Universe<C, E, M, CA, EA>, &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>], &[OwnedAction<C, E, CA, EA>])
    ) -> (ActionExecutor<C, E, M, CA, EA>, Self) {
        // create an action handler that can be passed back which handles our logic as well as the original logic
        let hooked_handler = move |
            universe: &mut Universe<C, E, M, CA, EA>, self_actions: &[OwnedAction<C, E, CA, EA>],
            cell_actions: &[OwnedAction<C, E, CA, EA>], entity_actions: &[OwnedAction<C, E, CA, EA>]
        | {
            // form the actions into messages and send them to the client
            unimplemented!(); // TODO

            // Call the consumed action handler and actually mutate the universe.
            action_executor(universe, self_actions, cell_actions, entity_actions);
        };

        (Box::new(hooked_handler), HybridServer::new())
    }

    pub fn new() -> Self {
        HybridServer {
            pending_snapshot: false,
            seq: Arc::new(AtomicU32::new(0)),
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
            __phantom_m: PhantomData,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
            __phantom_v: PhantomData,
        }
    }
}

/// Defines an event that takes place in the universe.  Given the event, the hybrid client must be able to
/// apply it to the universe.
pub trait Event<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>
> : Serialize + for<'u> Deserialize<'u> {
    fn apply(&self, universe: &mut Universe<C, E, M, CA, EA>);
}
