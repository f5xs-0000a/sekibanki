use futures_channel::{
    mpsc::Sender,
    oneshot::{
        channel as osh_channel,
        Receiver as OShReceiver,
        Sender as OShSender,
    },
};
use futures_util::SinkExt;
use std::sync::Arc;

use actor::Actor;
use channels::{
    PMChannelType,
    PMUnboxedType,
};
use message::{
    Message,
    MessageResponse,
    PackedMessage,
};
use response::ResponseFuture;

////////////////////////////////////////////////////////////////////////////////

/// Address pointing to an actor. Used to send messages to an actor.
#[derive(Clone)]
pub struct Addr<A>
where
    A: Actor, {
    sd: Arc<ActorSelfDestructor>,
    tx: Sender<PMChannelType<A>>,
}

/// When this object is dropped, it sends a message to the actor to kill itself.
///
/// This is meant to be wrapped in an `Arc` and a clone is held by an address.
/// If all addresses pointing to a specific actor were dropped, the `drop()`
/// method of this struct gets called, sending the signal to the actor to kill
/// itself.
#[derive(Debug)]
pub(crate) struct ActorSelfDestructor {
    tx: Option<OShSender<()>>,
}

////////////////////////////////////////////////////////////////////////////////

impl<A> Addr<A>
where
    A: Actor,
{
    /// Called during the creation of the actor
    pub(crate) fn new(
        tx: Sender<PMChannelType<A>>,
        sd: Arc<ActorSelfDestructor>,
    ) -> Addr<A> {
        Addr {
            tx,
            sd,
        }
    }

    /// Send a message to the actor
    pub fn send<M, MR>(
        &mut self,
        msg: M,
    ) -> ResponseFuture<A, M>
    where
        M: Message<A, Response = MR>,
        MR: MessageResponse, {
        // wrap the message and receive the response channel
        let (rx, pm) = PackedMessage::new_with_response_channel(msg);

        // send the packed message
        // self.tx.send(Box::new(pm as PMUnboxedType));
        self.tx.send(unsafe {
            // WARNING: DON'T TRY THIS AT HOME
            ::std::mem::transmute::<_, PMChannelType<A>>(Box::new(pm))
        });

        ResponseFuture::with_receiver(rx)
    }
}

impl ActorSelfDestructor {
    pub fn new() -> (ActorSelfDestructor, OShReceiver<()>) {
        let (tx, rx) = osh_channel();

        let asd = ActorSelfDestructor {
            tx: Some(tx)
        };

        (asd, rx)
    }
}

impl Drop for ActorSelfDestructor {
    fn drop(&mut self) {
        self.tx.take().unwrap().send(());
    }
}
