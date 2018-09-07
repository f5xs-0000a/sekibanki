use futures::sync::{
    mpsc::UnboundedSender as Sender,
    oneshot::{
        channel as oneshot,
        Receiver as OneShotReceiver,
        Sender as OneShotSender,
    },
};
use std::{
    sync::Arc,
    time::Duration,
};

use actor::{
    Actor,
    Handles,
};
use message::{
    Envelope,
    Message,
};
use notify::{
    new_notify,
    NotifyHandle,
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

#[derive(Clone)]
pub struct WeakAddr<A>
where
    A: Actor, {
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

        // wrap the message and the sender into an envelope
        let envelope = Envelope::new(msg, rtx);
        self.tx.unbounded_send(envelope).expect(
            "Message sending unexpectedly failed. Perhaps the receiver \
             address was also unexpectedly dropped?",
        );

        // return the response channel
        ResponseFuture::with_receiver(rrx)
    }

    /// Send a message to the actor at a later time
    pub fn send_later<M>(
        &mut self,
        msg: M,
        sleep: Duration,
    ) -> (NotifyHandle<A, M>, ResponseFuture<A, M>)
    where
        M: Message + 'static + Sync,
        A: Handles<M>, {
        // create the response channel
        let (rtx, rrx) = oneshot();

        // create the notify channel
        let (nh, cnh) = new_notify();

        // clone the actor send channel
        let new_tx = self.tx.clone();

        // spawn a thread that will sleep for a given duration then forget about
        // it
        ::std::thread::spawn(move || {
            ::std::thread::sleep(sleep);

            // if the handle is still not dropped...
            if cnh.is_alive() {
                // send the message but don't care if the send failed
                new_tx.unbounded_send(Envelope::new(msg, rtx));
            }
        });

        let response_future = ResponseFuture::with_receiver(rrx);

        (nh, response_future)
    }

    pub fn weak(&self) -> WeakAddr<A> {
        WeakAddr {
            tx: self.tx.clone(),
        }
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

impl<A> WeakAddr<A>
where
    A: Actor,
{
    fn send<M>(
        &mut self,
        msg: M,
    ) -> ResponseFuture<A, M>
    where
        M: Message + 'static,
        A: Handles<M>, {
        // create the response channel
        let (rtx, rrx) = oneshot();

        let envelope = Envelope::new(msg, rtx);
        self.tx.unbounded_send(envelope).expect(
            "Message sending unexpectedly failed. Perhaps the receiver \
             address was also unexpectedly dropped?",
        );

        // return the response channel
        ResponseFuture::with_receiver(rrx)
    }
}
