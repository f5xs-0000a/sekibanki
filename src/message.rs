use std::marker::PhantomData;
use futures_channel::{
    oneshot::Sender as OneShotSender,
};

use actor::Actor;
use actor::Handles;
use context::ContextImmutHalf;

///////////////////////////////////////////////////////////////////////////////

pub trait Message {
}

impl<M> Message for M
where M: Send {
}

pub struct EnvelopeInner<A, M>
where A: Handles<M>,
      M: Message {
    tx: Option<OneShotSender<A::Response>>,
    msg: Option<M>,
}

impl<A, M> EnvelopeInner<A, M>
where A: Handles<M>,
      M: Message {
    fn boxed_new(msg: M, tx: OneShotSender<A::Response>) -> Box<EnvelopeInner<A, M>> {
        Box::new(
            EnvelopeInner {
                tx: Some(tx),
                msg: Some(msg),
            }
        )
    }
}

pub trait EnvelopeInnerTrait {
    type A: Actor;

    fn handle(&mut self, actor: &mut Self::A, ctx: &ContextImmutHalf<Self::A>);
}

pub struct Envelope<A: Actor>(Box<EnvelopeInnerTrait<A = A>>);

impl<A, M> EnvelopeInnerTrait for EnvelopeInner<A, M>
where A: Handles<M>,
      M: Message {
    type A = A;

    fn handle(&mut self, actor: &mut Self::A, ctx: &ContextImmutHalf<Self::A>) {
        if let Some(msg) = self.msg.take() {
            actor.handle(msg, ctx);
        }

        unimplemented!();
    }
}

impl<A> Envelope<A> where A: Actor {
    pub fn new<M>(msg: M, tx: OneShotSender<A::Response>) -> Envelope<A>
    where M: Message + 'static,
          A: Handles<M> + 'static {
        Envelope(EnvelopeInner::boxed_new(msg, tx))
    }

    pub fn handle(mut self, actor: &mut A, ctx: &ContextImmutHalf<A>) {
        self.0.handle(actor, ctx);
    }
}
