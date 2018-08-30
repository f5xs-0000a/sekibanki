use either::Either;
use futures::{
    task::Context as FutContext,
    Async,
    Future,
    Never,
    Poll,
    Stream,
};
use futures_channel::{
    mpsc::{
        Receiver,
        Sender,
    },
    oneshot::Receiver as OShReceiver,
};
use std::sync::{
    Arc,
    Weak,
};

use actor::{
    Actor,
    BeAliveOrDead,
};
use address::{
    ActorSelfDestructor,
    Addr,
};
use message::{
    Message,
    MessageResponse,
    PackedMessage,
};

////////////////////////////////////////////////////////////////////////////////

/// The half of the context that is immutable.
#[derive(Debug)]
pub struct ContextImmutHalf<A>
where
    A: Actor, {
    tx: Sender<PackedMessage<A, Box<Message<A, Response = MessageResponse>>>>,

    // tx: Sender<Box<dyn Message<A, Response = MessageResponse>>>,
    //
    // this attribute is needed so he can make an exact clone of the address
    // without the need of the address
    // and this is guaranteed to be alive for most intents and purposes
    sd: Weak<ActorSelfDestructor>,
}

/// The half of the context that is mutable.
#[derive(Debug)]
struct ContextMutHalf<A>
where
    A: Actor, {
    rx: Receiver<Box<dyn Message<A, Response = MessageResponse>>>,
    self_destruct_rx: OShReceiver<()>,
    actor: A,
}

/// The context of the actor.
#[derive(Debug)]
pub struct Context<A>
where
    A: Actor, {
    immut_half: ContextImmutHalf<A>,
    mut_half:   ContextMutHalf<A>,
}

////////////////////////////////////////////////////////////////////////////////

impl<A> ContextImmutHalf<A>
where
    A: Actor,
{
    pub fn addr(&self) -> Addr<A> {
        // as a consequence of the initialization of the `sd` attribute, the
        // immutable half cannot be accessed when the actor is stopping.
        // TODO: you need your own method
        Addr {
            tx: self.tx.clone(),

            // it's got to be alive
            sd: Arc::upgrade(&self.sd).expect(
                "Attempted to upgrade an already dead weak shared pointer.",
            ),
        }
    }
}

impl<A> Stream for ContextMutHalf<A>
where
    A: Actor,
{
    type Error = Never;
    type Item = Either<Box<dyn Message<A, Response = MessageResponse>>, ()>;

    fn poll_next(
        &mut self,
        cx: &mut FutContext,
    ) -> Poll<Option<Self::Item>, Self::Error> {
        use self::{
            Async::*,
            Either::*,
        };

        // the error value of receivers is Never so they will never error, i.e.
        // they can be safely unwrapped.

        // prioritize rx first
        match self.rx.poll().unwrap() {
            // don't do anything if it's still pending
            Pending => {},

            // return the ready value
            Ready(polled_msg) => return Ok(Ready(Left(polled_msg))),
        }

        // then prioritize the self-destructor
        match self.self_destruct_rx.poll().unwrap() {
            // just return pending if it's still pending
            Pending => return Ok(Pending),

            // return if self-destruct sequence is requested
            Ready(_) => return Ok(Ready(Right(()))),
        };
    }
}

impl<A> Context<A>
where
    A: Actor,
{
    pub(crate) fn new(actor: A) -> Context<A> {
        unimplemented!() // TODO: this is your last edit
    }
}

impl<A> Stream for Context<A>
where
    A: Actor,
{
    type Error = Never;
    type Item = BeAliveOrDead;

    fn poll_next(
        &mut self,
        cx: &mut FutContext,
    ) -> Poll<Option<Self::Item>, Self::Error> {
        use self::{
            Async::*,
            Either::*,
        };

        let message = match (&mut self.mut_half).next() {
            NotReady => return NotReady,
            Ready(Right(_)) => return Ready(BeAliveOrDead::Dead),
            Ready(Left(msg)) => msg,
        };

        message.handle(&mut self.mut_half.actor, &self.immut_half);
        Ready(BeAliveOrDead::Alive);
    }
}
