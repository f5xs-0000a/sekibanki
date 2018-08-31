use futures_channel::oneshot::{
    channel as osh_channel,
    Receiver as OShReceiver,
    Sender as OShSender,
};

use actor::Actor;
use context::ContextImmutHalf;

///////////////////////////////////////////////////////////////////////////////

/// Trait for objects that can be sent to actors as messages
pub trait Message<A>
where
    A: Actor, {
    type Response: MessageResponse;

    fn handle(
        self,
        actor: &mut A,
        ctx: &ContextImmutHalf<A>,
    ) -> Self::Response;
}

pub trait MessageResponse {}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct PackedMessage<A, M>
where
    A: Actor,
    M: Message<A> + ?Sized, {
    tx:  OShSender<M::Response>,
    msg: M,
}

////////////////////////////////////////////////////////////////////////////////

impl<T> MessageResponse for T {
}

impl<A, M> Message<A> for Box<M>
where
    A: Actor,
    M: Message<A>,
{
    type Response = M::Response;

    fn handle(
        self,
        actor: &mut A,
        ctx: &ContextImmutHalf<A>,
    ) -> Self::Response {
        (*self).handle(actor, ctx)
    }
}

impl<A> Message<A> for ()
where
    A: Actor,
{
    type Response = ();

    fn handle(
        self,
        actor: &mut A,
        ctx: &ContextImmutHalf<A>,
    ) -> Self::Response {
        ()
    }
}

impl<A, M> PackedMessage<A, M>
where
    A: Actor,
    M: Message<A>,
{
    pub fn new_with_response_channel(
        msg: M,
    ) -> (OShReceiver<M::Response>, PackedMessage<A, M>) {
        let (tx, rx) = osh_channel();

        let pm = PackedMessage {
            msg,
            tx,
        };

        (rx, pm)
    }

    pub(crate) fn into_parts(self) -> (OShSender<M::Response>, M) {
        (self.tx, self.msg)
    }

    //
    // pub (crate) fn handle(
    // self,
    // actor: &mut A,
    // immut_half: &ContextImmutHalf<A>,
    // ) {
    // process the message and get the response
    // let response = self.msg.handle(actor, immut_half);
    //
    // send the response back to the sender
    // self.tx.send(response);
    // }
    //
}

//
// impl<A> Into<PMUnboxedType<A>> for PackedMessage<A, M>
// where A: Actor, M: Message<A>, {
// fn into(self) -> PMUnboxedType<A> {
// self as PMUnboxedType<A>
// }
// }
//
