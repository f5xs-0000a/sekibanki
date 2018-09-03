use futures_channel::{
    mpsc::UnboundedSender as Sender,
    oneshot::{
        channel as oneshot,
        Receiver as OneShotReceiver,
        Sender as OneShotSender,
    },
};
use std::sync::Arc;

use actor::{
    Actor,
    Handles,
};
use message::{
    Envelope,
    Message,
};
use response::ResponseFuture;

////////////////////////////////////////////////////////////////////////////////

/// Address pointing to an actor. Used to send messages to an actor.
#[derive(Clone)]
pub struct Addr<A>
where
    A: Actor, {
    sd: Arc<ActorSelfDestructor>,
    tx: Sender<Envelope<A>>,
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
    A: Actor + 'static,
{
    /// Called during the creation of the actor
    pub(crate) fn new(
        tx: Sender<Envelope<A>>,
        sd: Arc<ActorSelfDestructor>,
    ) -> Addr<A> {
        Addr {
            tx,
            sd,
        }
    }

    /// Send a message to the actor
    pub fn send<M>(
        &mut self,
        msg: M,
    ) -> ResponseFuture<A, M>
    where
        M: Message + 'static,
        A: Handles<M>, {
        // create the response channel
        let (rtx, rrx) = oneshot();

        //
        // create the closure
        // let closure = move |actor: &mut A, ctx: &ContextImmutHalf<A>| {
        // perform the operation and retrieve the response
        // let response = msg.handle(actor, ctx);
        //
        // send the response
        // rtx.send(response);
        // };
        //
        // send the message closure
        // self.tx.unbounded_send(Box::new(closure));
        //

        let envelope = Envelope::new(msg, rtx);
        self.tx.unbounded_send(envelope).expect(
            "Message sending unexpectedly failed. Perhaps the receiver \
             address was also unexpectedly dropped?",
        );

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
        // don't care if it fails
        self.tx.take().unwrap().send(());
    }
}
