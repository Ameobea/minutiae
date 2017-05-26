//! Defines a hybrid client that receives events from the server that don't directly correspond to pixel colors.
//! This is useful for simulations that have highly abstractable actions that affect multiple pixels.  It requires that
//! the client maintains a full copy of the universe's state including cell and entity states.

use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;

use minutiae::universe::Universe;
use minutiae::cell::{Cell, CellState};
use minutiae::entity::{Entity, EntityState, MutEntityState};
use minutiae::action::{CellAction, EntityAction};
use minutiae_libremote::{Message, ServerMessage};

use super::{Client, ClientState};

type HybridServerSnapshot<C: CellState, E: EntityState<C>, M: MutEntityState> = (Vec<Cell<C>>, Vec<Entity<C, E, M>>);

#[derive(Clone, Debug)]
pub enum HybridServerMessageContents<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, V: Event<C, E, M, CA, EA>
> {
    Snapshot(HybridServerSnapshot<C, E, M>),
    Event(Vec<V>),
    __phantom_ca(PhantomData<CA>),
    __phantom_ea(PhantomData<EA>),
}

#[derive(Clone)]
pub struct HybridServerMessage<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, V: Event<C, E, M, CA, EA>
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
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, V: Event<C, E, M, CA, EA>
> Debug for HybridServerMessage<C, E, M, CA, EA, V> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "HybridServerMessage {{seq: {}, contents: {{..}} }}", self.seq)
    }
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, V: Event<C, E, M, CA, EA>
> PartialEq for HybridServerMessage<C, E, M, CA, EA, V> {
    fn eq(&self, rhs: &Self) -> bool {
        self.seq == rhs.seq /*&& self.contents == rhs.seq*/
    }
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, V: Event<C, E, M, CA, EA>
> Eq for HybridServerMessage<C, E, M, CA, EA, V> {
    
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, V: Event<C, E, M, CA, EA>
> PartialOrd for HybridServerMessage<C, E, M, CA, EA, V> {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.seq.cmp(&rhs.seq))
    }
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, V: Event<C, E, M, CA, EA>
> Ord for HybridServerMessage<C, E, M, CA, EA, V> {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.seq.cmp(&rhs.seq)
    }
}

impl<
    C: CellState + Send, E: EntityState<C> + Send, M: MutEntityState + Send, CA: CellAction<C> + Send,
    EA: EntityAction<C, E> + Send, V: Event<C, E, M, CA, EA> + Send
> Message for HybridServerMessage<C, E, M, CA, EA, V> {
    fn serialize(&self) -> Result<Vec<u8>, String> {
        unimplemented!(); // TODO
    }

    fn deserialize(bin: &[u8]) -> Result<Self, String> {
        unimplemented!(); // TODO
    }
}

impl<
    C: CellState + Send, E: EntityState<C> + Send, M: MutEntityState + Send, CA: CellAction<C> + Send,
    EA: EntityAction<C, E> + Send, V: Event<C, E, M, CA, EA> + Send
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
    C: CellState + Send, E: EntityState<C> + Send, M: MutEntityState + Send, CA: CellAction<C> + Send,
    EA: EntityAction<C, E> + Send, V: Event<C, E, M, CA, EA> + Send
> {
    universe: Universe<C, E, M, CA, EA>,
    state: ClientState<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>>,
}

impl<
    C: CellState + Send, E: EntityState<C> + Send, M: MutEntityState + Send, CA: CellAction<C> + Send,
    EA: EntityAction<C, E> + Send, V: Event<C, E, M, CA, EA> + Send
> Client<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>> for HybridClient<C, E, M, CA, EA, V> {
    fn handle_message(&mut self, message: HybridServerMessage<C, E, M, CA, EA, V>) {
        unimplemented!(); // TODO
    }

    fn apply_snap(&mut self, snap: HybridServerSnapshot<C, E, M>) {
        unimplemented!(); // TODO
    }

    fn get_pixbuf_ptr(&self) -> *const u8 {
        unimplemented!(); // TODO
    }

    fn get_state(&mut self) -> &mut ClientState<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>> {
        &mut self.state
    }
}
