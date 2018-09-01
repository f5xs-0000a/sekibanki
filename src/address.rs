use futures_channel::{
    mpsc::UnboundedSender as Sender,
    oneshot::{
        channel as oneshot,
        Receiver as OneShotReceiver,
        Sender as OneShotSender,
    },
};
use std::sync::Arc;

use actor::Actor;
use channels::PMChannelType;
use context::ContextImmutHalf;
use message::{
    Message,
    MessageResponse,
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
    tx: Option<OneShotSender<()>>,
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
        M: Message<A, Response = MR> + 'static,
        MR: MessageResponse + 'static, {
        // create the response channel
        let (rtx, rrx) = oneshot();

        // create the closure
        let closure = move |actor: &mut A, ctx: &ContextImmutHalf<A>| {
            // perform the operation and retrieve the response
            let response = msg.handle(actor, ctx);

            // send the response
            rtx.send(response);
        };

        // send the message closure
        self.tx.unbounded_send(Box::new(closure));

        // return the response channel
        ResponseFuture::with_receiver(rrx)
    }
}

impl ActorSelfDestructor {
    pub fn new() -> (ActorSelfDestructor, OneShotReceiver<()>) {
        let (tx, rx) = oneshot();

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
