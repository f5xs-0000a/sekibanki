use futures_channel::{
    mpsc::Sender,
    oneshot::Sender as OShSender,
};
use std::sync::Arc;

use actor::Actor;
use message::{
    Message,
    MessageResponse,
    PackedMessage,
};
use response::ResponseFuture;

////////////////////////////////////////////////////////////////////////////////

/// Address pointing to an actor. Used to send messages to an actor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Addr<A>
where
    A: Actor, {
    tx: Sender<Box<dyn Message<A, Response = MessageResponse>>>,
    sd: Arc<ActorSelfDestructor>,
}

/// When this object is dropped, it sends a message to the actor to kill itself.
///
/// This is meant to be wrapped in an `Arc` and a clone is held by an address.
/// If all addresses pointing to a specific actor were dropped, the `drop()`
/// method of this struct gets called, sending the signal to the actor to kill
/// itself.
pub(crate) struct ActorSelfDestructor {
    tx: Option<OShSender<()>>,
}

impl Drop for ActorSelfDestructor {
    fn drop(&mut self) {
        self.tx.take().unwrap().send(());
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<A> Addr<A>
where
    A: Actor,
{
    /// Called during the creation of the actor
    pub(crate) fn on_new_actor(
        sender: Sender<Box<dyn Message<A, Response = MessageResponse>>>,
        destructor: Sender<()>,
    ) -> Addr<A> {
        // create the self-destructor
        let sd = Arc::new(ActorSelfDestructor {
            tx: destructor
        });

        Addr {
            tx: sender,
            sd,
        }
    }

    /// Send a message to the actor
    pub fn send<M, MR>(
        &mut self,
        msg: PackedMessage<A, M>,
    ) -> ResponseFuture<A, M>
    where
        M: Message<A, Response = MR>,
        MR: MessageResponse, {
        // wrap the message and receive the response channel
        let (pm, rx) = PackedMessage::new_with_response_channel(msg);

        // send the packed message
        self.tx.send(pm);

        ResponseFuture::with_receiver(rx)
    }
}
