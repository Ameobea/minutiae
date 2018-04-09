//! Hybrid clients maintain the full universe state internally but only take action in response to events sent from
//! the server.  These events can be more complicated than single `EntityAction`s and can affect more than one pixel
//! at a time, such as translating every entity in the universe or removing entities en-masse.

use std::cmp::{Ord, Ordering};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

use futures::Future;
use futures::sync::oneshot::{channel as oneshot_channel, Sender as OneshotSender};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use action::OwnedAction;

use prelude::*;
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

#[derive(Debug, PartialEq, Eq, Serialize, Clone, Deserialize)]
#[serde(bound = "T: for<'d> Deserialize<'d>")]
pub struct HybridClientMessage<
    T: Serialize + for<'d> Deserialize<'d> + Clone + PartialEq + Eq
> {
    pub client_id: Uuid,
    pub contents: HybridClientMessageContents<T>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone, Deserialize)]
#[serde(bound = "T: for<'d> Deserialize<'d>")]
pub enum HybridClientMessageContents<
    T: Serialize + for<'d> Deserialize<'d> + Clone + PartialEq + Eq
> {
    RequestSnapshot,
    Custom(T),
}

impl<
    T: Serialize + for<'d> Deserialize<'d> + Clone + Debug + Send + PartialEq + Eq
> ClientMessage for HybridClientMessage<T> {
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

/// A response to a custom message from a client.  The response can contain an optional response back to the client
/// as well as an optional list of messages to be broadcast to all other connected clients.
pub struct ClientEventAction<T: Tys> {
    pub response_msg: Option<T::ServerMessage>,
    pub broadcast_msgs: Option<Vec<T::ServerMessage>>,
}

pub struct HybridServer<
    HCMT: Serialize + for<'d> Deserialize<'d> + Clone + Debug + Send + PartialEq + Eq,
    T: Tys<ClientMessage=HybridClientMessage<HCMT>>,
> where
    T::Snapshot: Clone,
    T::V: Clone,
{
    snapshot_requests: Vec<OneshotSender<(u32, T::Snapshot)>>,
    custom_actions: Vec<(OneshotSender<Option<HybridServerMessage<T>>>, HCMT)>,
    event_generator: fn(
        universe: &mut T::U,
        seq: u32,
        cell_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
        self_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
        entity_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>]
    ) -> Option<Vec<T::V>>,
    client_event_handler: fn(
        universe: &mut T::U,
        seq: u32,
        custom_action: HCMT
    ) -> ClientEventAction<T>,
    self_actions: Arc<RwLock<Vec<OwnedAction<T::C, T::E, T::CA, T::EA, T::I>>>>,
    cell_actions: Arc<RwLock<Vec<OwnedAction<T::C, T::E, T::CA, T::EA, T::I>>>>,
    entity_actions: Arc<RwLock<Vec<OwnedAction<T::C, T::E, T::CA, T::EA, T::I>>>>,
}

impl<
    HCMT: Serialize + for<'d> Deserialize<'d> + Clone + Debug + Send + Sync + PartialEq + Eq + 'static,
    T: Tys<
        ServerMessage=HybridServerMessage<T>,
        ClientMessage=HybridClientMessage<HCMT>,
    >,
> ServerLogic<T> for HybridServer<HCMT, T> where
    T: 'static,
    T::C: Send + Sync + Clone,
    T::E: Send + Sync + Clone,
    T::M: Send + Sync + Clone,
    T::I: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    T::U: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    T::CA: Send + Sync + Clone,
    T::EA: Send + Sync + Clone,
    T::V: Clone + Send,
    T::Snapshot: Serialize + for<'de> Deserialize<'de> + Clone + Send + From<T::U>,
    T::ServerMessage: Send + Sync,
{
    fn tick(
        &mut self,
        seq: u32,
        universe: &mut T::U
    ) -> Option<Vec<HybridServerMessage<T>>> {
        // Handle any pending snapshot requests
        for oneshot_tx in self.snapshot_requests.drain(..) {
            println!("Fulfilling snapshot request...");
            let _ = oneshot_tx.send((seq, universe.clone().into(),));
        }

        // Handle any pending custom action
        let mut msgs_to_broadcast: Option<Vec<HybridServerMessage<T>>> = None;
        for (oneshot_tx, hcmt) in self.custom_actions.drain(..) {
            let ClientEventAction { response_msg, broadcast_msgs } = (self.client_event_handler)(universe, seq, hcmt);
            let _ = oneshot_tx.send(response_msg);

            // If this custom event produced messages to broadcast, merge them with the existing list of messages to broadcast.
            msgs_to_broadcast = match (msgs_to_broadcast, broadcast_msgs) {
                (None, Some(msgs)) => Some(msgs),
                (None, None) => None,
                (Some(msgs), None) => Some(msgs),
                (Some(output_msgs), Some(msgs)) => Some([&output_msgs[..], &msgs[..]].concat()),
            };
        }

        let pending_messages: Option<Vec<T::V>> = None;

        // use the user-defined logic to get the events to pass through to the clients.  The internal action buffers
        // are used as the sources for this.
        // TODO: Look into recycling the buffers rather than reallocating if we're going to keep this system (we shouldn't)
        let merged_events: Vec<T::V> = match (self.event_generator)(
            universe,
            seq,
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

        let merged_events_msg = HybridServerMessage::new(seq, HybridServerMessageContents::Event(merged_events));
        match msgs_to_broadcast {
            Some(mut msgs) => {
                msgs.push(merged_events_msg);
                Some(msgs)
            },
            None => Some(vec![merged_events_msg]),
        }
    }

    fn handle_client_message(
        &mut self,
        _seq: u32,
        message: HybridClientMessage<HCMT>
    ) -> Box<Future<Item=Option<HybridServerMessage<T>>, Error=!>> {
        println!("Handling client message: {:?}", message);
        match message.contents {
            HybridClientMessageContents::RequestSnapshot => {
                box self.request_snapshot()
                    .map(|(seq, snapshot)| {
                        let server_message = HybridServerMessage::new(
                            seq,
                            HybridServerMessageContents::Snapshot(snapshot)
                        );
                        Some(server_message)
                    })
            },
            HybridClientMessageContents::Custom(hcmt) => {
                box self.handle_custom_message(hcmt)
            }
        }
    }
}

impl<
    CS: CellState + Send + 'static,
    ES: EntityState<CS> + Send,
    MES: MutEntityState + Send,
    CA: CellAction<CS> + Send,
    EA: EntityAction<CS, ES> + Send,
    I: Ord + Copy + Send + 'static,
    U: Universe<CS, ES, MES, Coord=I>,
    HCMT: Serialize + for<'d> Deserialize<'d> + Clone + Debug + Send + PartialEq + Eq,
    T: Tys<
        C=CS,
        E=ES,
        M=MES,
        CA=CA,
        EA=EA,
        I=I,
        U=U,
        ClientMessage=HybridClientMessage<HCMT>,
    >,
> HybridServer<HCMT, T> where
    OwnedAction<T::C, T::E, T::CA, T::EA, T::I>: Clone,
    T::V: Clone,
    T::Snapshot: Clone,
{
    /// Takes the action handlers for the engine and hooks them, getting an intermediate view of the actions
    /// so that they can be transmitted to the client before handling them on the client side.
    pub fn hook_handler(
        action_executor: fn(
            &mut U,
            &[OwnedAction<CS, ES, CA, EA, I>],
            &[OwnedAction<CS, ES, CA, EA, I>],
            &[OwnedAction<CS, ES, CA, EA, I>]
        ),
        event_generator: fn(
            universe: &mut U,
            seq: u32,
            cell_actions: &[OwnedAction<CS, ES, CA, EA, I>],
            self_actions: &[OwnedAction<CS, ES, CA, EA, I>],
            entity_actions: &[OwnedAction<CS, ES, CA, EA, I>]
        ) -> Option<Vec<T::V>>,
        client_event_handler: fn(
            universe: &mut T::U,
            seq: u32,
            custom_event: HCMT
        ) -> ClientEventAction<T>
    ) -> (
        impl Fn(
            &mut U,
            &[OwnedAction<CS, ES, CA, EA, I>],
            &[OwnedAction<CS, ES, CA, EA, I>],
            &[OwnedAction<CS, ES, CA, EA, I>]
        ),
        Self,
    ) {
        let hybrid_server = HybridServer::new(event_generator, client_event_handler);
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

    fn new(
        event_generator: fn(
            universe: &mut T::U,
            seq: u32,
            cell_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            self_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>],
            entity_actions: &[OwnedAction<T::C, T::E, T::CA, T::EA, T::I>]
        ) -> Option<Vec<T::V>>,
        client_event_handler: fn(
            universe: &mut T::U,
            seq: u32,
            custom_action: HCMT
        ) -> ClientEventAction<T>
    ) -> Self {
        HybridServer {
            event_generator,
            client_event_handler,
            snapshot_requests: Vec::new(),
            custom_actions: Vec::new(),
            self_actions: Arc::new(RwLock::new(Vec::new())),
            cell_actions: Arc::new(RwLock::new(Vec::new())),
            entity_actions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn request_snapshot(&mut self) -> impl Future<Item=(u32, T::Snapshot), Error=!> {
        let (oneshot_tx, oneshot_rx) = oneshot_channel();
        self.snapshot_requests.push(oneshot_tx);
        oneshot_rx.map_err(|_| unreachable!())
    }

    fn handle_custom_message(
        &mut self,
        custom_message: HCMT
    ) -> impl Future<Item=Option<HybridServerMessage<T>>, Error=!> {
        let (oneshot_tx, oneshot_rx) = oneshot_channel();
        self.custom_actions.push((oneshot_tx, custom_message,));
        oneshot_rx.map_err(|_| unreachable!())
    }
}

#[test]
fn hybrid_server_message_binary_serialization() {
    use prelude::*;
    use server::{Message, Tys};
    use universe::Universe2D;

    #[derive(Clone, Copy)]
    struct TestTys;
    impl Tys for TestTys {
        type C = CS;
        type E = ES;
        type M = MES;
        type CA = CA;
        type EA = EA;
        type I = usize;
        type U = Universe2D<CS, ES, MES>;
        type V = TestEvent;
        type Snapshot = Self::U;
        type ServerMessage = HybridServerMessage<Self>;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
    struct CS;
    impl CellState for CS {}
    #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
    struct ES;
    impl EntityState<CS> for ES {}
    #[derive(Clone, Default, Copy, Debug, PartialEq, Serialize, Deserialize)]
    struct MES;
    impl MutEntityState for MES {}
    #[derive(Clone, Default, Copy, Debug, PartialEq, Serialize, Deserialize)]
    struct CA;
    impl CellAction<CS> for CA {}
    #[derive(Clone, Default, Copy, Debug, PartialEq, Serialize, Deserialize)]
    struct EA;
    impl EntityAction<CS, ES> for EA {}
    #[derive(Clone, Default, Copy, Debug, PartialEq, Serialize, Deserialize)]
    struct TestEvent;
    impl Event<TestTys> for TestEvent {
        fn apply(&self, universe: &mut <TestTys as Tys>::U) {}
    }

    let universe: Universe2D<CS, ES, MES> = Universe2D::default();
    let sm_contents = HybridServerMessageContents::Snapshot(universe);
    let sm: <TestTys as Tys>::ServerMessage = HybridServerMessage::new(300, sm_contents);
    let sm_clone = sm.clone();
    let encoded = sm.bin_serialize().unwrap();
    let decoded = <TestTys as Tys>::ServerMessage::bin_deserialize(&encoded).unwrap();
    assert_eq!(sm_clone, decoded);
}
