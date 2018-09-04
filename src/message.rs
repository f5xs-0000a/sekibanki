use futures::sync::oneshot::Sender as OneShotSender;

use actor::{
    Actor,
    Handles,
};
use context::ContextImmutHalf;

////////////////////////////////////////////////////////////////////////////////

pub trait Message: Send {}

pub trait EnvelopeInnerTrait: Send {
    type A: Actor;

    fn handle(
        &mut self,
        actor: &mut Self::A,
        ctx: &ContextImmutHalf<Self::A>,
    );
}

////////////////////////////////////////////////////////////////////////////////

pub struct EnvelopeInner<A, M>
where
    A: Handles<M>,
    M: Message, {
    tx:  Option<OneShotSender<A::Response>>,
    msg: Option<M>,
}

pub struct Envelope<A: Actor>(Box<EnvelopeInnerTrait<A = A>>);

////////////////////////////////////////////////////////////////////////////////

impl<M> Message for M where M: Send {
}

impl<A, M> EnvelopeInner<A, M>
where
    A: Handles<M>,
    M: Message,
{
    fn boxed_new(
        msg: M,
        tx: OneShotSender<A::Response>,
    ) -> Box<EnvelopeInner<A, M>> {
        Box::new(EnvelopeInner {
            tx:  Some(tx),
            msg: Some(msg),
        })
    }
}

impl<A, M> EnvelopeInnerTrait for EnvelopeInner<A, M>
where
    A: Handles<M>,
    M: Message,
{
    type A = A;

    fn handle(
        &mut self,
        actor: &mut Self::A,
        ctx: &ContextImmutHalf<Self::A>,
    ) {
        if let Some(msg) = self.msg.take() {
            let response = actor.handle(msg, ctx);

            if let Some(tx) = self.tx.take() {
                // don't care if it fails
                tx.send(response);
            }
        }
    }
}

impl<A> Envelope<A>
where
    A: Actor,
{
    pub fn new<M>(
        msg: M,
        tx: OneShotSender<A::Response>,
    ) -> Envelope<A>
    where
        M: Message + 'static,
        A: Handles<M> + 'static, {
        Envelope(EnvelopeInner::boxed_new(msg, tx))
    }

    pub fn handle(
        mut self,
        actor: &mut A,
        ctx: &ContextImmutHalf<A>,
    ) {
        self.0.handle(actor, ctx);
    }
}
