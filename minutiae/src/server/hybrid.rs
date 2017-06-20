//! Hybrid clients maintain the full universe state internally but only take action in response to events sent from
//! the server.  These events can be more complicated than single `EntityAction`s and can affect more than one pixel
//! at a time, such as translating every entity in the universe or removing entities en-masse.

use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

use universe::Universe;
use cell::{Cell, CellState};
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction};
use container::{EntityContainer, };

use super::{ServerMessage, ClientMessage};

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
    Event(Vec<V>),
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
            HybridServerMessageContents::Event(_) => Err(self),
            _ => unreachable!(),
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
