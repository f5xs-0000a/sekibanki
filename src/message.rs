use futures::sync::oneshot::Sender as OneShotSender;

use actor::{Actor, Handles};
use context::ContextImmutHalf;

////////////////////////////////////////////////////////////////////////////////

pub trait Message<R>: Send {}

pub trait EnvelopeInnerTrait: Send {
    type A: Actor;

    fn handle(&mut self, actor: &mut Self::A, ctx: &ContextImmutHalf<Self::A>);
}

////////////////////////////////////////////////////////////////////////////////

pub struct EnvelopeInner<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    tx: Option<OneShotSender<A::Response>>,
    msg: Option<M>,
}

pub struct Envelope<A: Actor>(Box<EnvelopeInnerTrait<A = A>>);

////////////////////////////////////////////////////////////////////////////////

impl<A, M> Message<A> for M where M: Send {}

impl<A, M> EnvelopeInner<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    fn boxed_new(msg: M, tx: OneShotSender<A::Response>) -> Box<EnvelopeInner<A, M>> {
        Box::new(EnvelopeInner {
            tx: Some(tx),
            msg: Some(msg),
        })
    }

    fn boxed_new_without_response(msg: M) -> Box<EnvelopeInner<A, M>> {
        Box::new(EnvelopeInner {
            tx: None,
            msg: Some(msg),
        })
    }
}

impl<A, M> EnvelopeInnerTrait for EnvelopeInner<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    type A = A;

    fn handle(&mut self, actor: &mut Self::A, ctx: &ContextImmutHalf<Self::A>) {
        if let Some(msg) = self.msg.take() {
            // let the actor handle the message
            let response = actor.handle(msg, ctx);

            // if we have a sender, we send the message
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
    pub fn new<M>(msg: M, tx: OneShotSender<A::Response>) -> Envelope<A>
    where
        M: Message<A> + 'static,
        A: Handles<M> + 'static,
    {
        Envelope(EnvelopeInner::boxed_new(msg, tx))
    }

    pub fn new_without_response<M>(msg: M) -> Envelope<A>
    where
        M: Message<A> + 'static,
        A: Handles<M> + 'static,
    {
        Envelope(EnvelopeInner::boxed_new_without_response(msg))
    }

    pub fn handle(mut self, actor: &mut A, ctx: &ContextImmutHalf<A>) {
        self.0.handle(actor, ctx);
    }
}
