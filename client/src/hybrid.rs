//! Defines a hybrid client that receives events from the server that don't directly correspond to pixel colors.
//! This is useful for simulations that have highly abstractable actions that affect multiple pixels.  It requires that
//! the client maintains a full copy of the universe's state including cell and entity states.

use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;

use minutiae::universe::{Universe, UniverseConf};
use minutiae::cell::{Cell, CellState};
use minutiae::entity::{Entity, EntityState, MutEntityState};
use minutiae::action::{CellAction, EntityAction};
use minutiae::container::{EntityContainer, };
use minutiae_libremote::{Message, ServerMessage, ClientMessage};
use serde::{Serialize, Deserialize, Deserializer};
use uuid::Uuid;

use super::{Client, ClientState};

/// Helper trait to contain some of the massive spam caused in trait definitions.  This requires that implementors are
pub trait HybParam : Send + Serialize + for<'de> Deserialize<'de> {}

type HybridServerSnapshot<C: CellState, E: EntityState<C>, M: MutEntityState> = (Vec<C>, EntityContainer<C, E, M>);

#[derive(Clone, Debug, Serialize)]
pub enum HybridServerMessageContents<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA>
> {
    Snapshot(HybridServerSnapshot<C, E, M>),
    Event(Vec<V>),
    __phantom_c(PhantomData<C>),
    __phantom_e(PhantomData<E>),
    __phantom_m(PhantomData<M>),
    __phantom_ca(PhantomData<CA>),
    __phantom_ea(PhantomData<EA>),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HybridClientMessage {
    client_id: Uuid,
    contents: HybridClientMessageContents,
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

#[derive(Clone, Serialize)]
pub struct HybridServerMessage<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> {
    seq: u32,
    contents: HybridServerMessageContents<C, E, M, CA, EA, V>,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
    __phantom_m: PhantomData<M>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
    __phantom_v: PhantomData<V>,
}

impl<
    'de, C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam,
    CA: CellAction<C> + HybParam, EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> Deserialize<'de> for HybridServerMessage<C, E, M, CA, EA, V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        unimplemented!();
    }
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
            HybridServerMessageContents::Event(_) => Err(self),
            _ => unreachable!(),
        }
    }
}

/// Defines an event that takes place in the universe.  Given the event, the hybrid client must be able to
/// apply it to the universe.
pub trait Event<C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>> {
    fn apply(&self, universe: &mut Universe<C, E, M, CA, EA>);
}

pub struct HybridClient<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam, CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> {
    universe_length: usize,
    universe: Universe<C, E, M, CA, EA>,
    state: ClientState<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>>,
    pixbuf: Vec<[u8; 4]>,
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam, CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> Client<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>> for HybridClient<C, E, M, CA, EA, V> {
    fn handle_message(&mut self, message: HybridServerMessage<C, E, M, CA, EA, V>) {
        match message.contents {
            HybridServerMessageContents::Event(evts) => for e in evts { e.apply(&mut self.universe); },
            HybridServerMessageContents::Snapshot(snap) => self.apply_snap(snap),
            _ => unreachable!(),
        }
    }

    fn apply_snap(&mut self, snap: HybridServerSnapshot<C, E, M>) {
        let (cells, entities) = snap;
        unimplemented!(); // TODO
        // self.universe.cells = cells
        //     .into_iter()
        //     .map(|state| Cell {state: state})
        //     .collect();
        // self.universe.entities = entities.into_entity_container();
    }

    fn get_pixbuf_ptr(&self) -> *const u8 {
        self.pixbuf.as_ptr() as *const u8
    }

    fn get_state(&mut self) -> &mut ClientState<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>> {
        &mut self.state
    }
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam, CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> HybridClient<C, E, M, CA, EA, V> {
    pub fn new(universe_size: usize) -> Self {
        let universe_length = universe_size * universe_size;
        HybridClient {
            universe_length,
            state: ClientState::new(),
            universe: Universe::uninitialized(universe_size),
            pixbuf: vec![[0u8; 4]; universe_length],
        }
    }
}
