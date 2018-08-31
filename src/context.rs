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
        channel,
        Receiver,
        Sender,
    },
    oneshot::Receiver as OShReceiver,
};
use futures_util::StreamExt;
use std::sync::{
    Arc,
    Weak,
};

use actor::{
    Actor,
    ActorBuilder,
    BeAliveOrDead,
};
use address::{
    ActorSelfDestructor,
    Addr,
};
use channels::PMChannelType;
use message::Message;

////////////////////////////////////////////////////////////////////////////////

/// The half of the context that is immutable.
pub(crate) struct ContextImmutHalf<A>
where
    A: Actor, {
    // this attribute is needed so he can make an exact clone of the address
    // without the need of the address
    // and this is guaranteed to be alive for most intents and purposes
    sd: Weak<ActorSelfDestructor>,

    tx: Sender<PMChannelType<A>>,
    // tx: Sender<Box<dyn Message<A, Response = MessageResponse>>>,
}

/// The half of the context that is mutable.
struct ContextMutHalf<A>
where
    A: Actor, {
    self_destruct_rx: OShReceiver<()>,
    rx: Receiver<PMChannelType<A>>,
    actor: A,
}

/// The context of the actor.
pub(crate) struct Context<A>
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
        let sd = Weak::upgrade(&self.sd).expect(
            "Attempted to upgrade an already dead weak shared pointer.",
        );

        Addr::new(self.tx.clone(), sd)
    }
}

impl<A> Stream for ContextMutHalf<A>
where
    A: Actor,
{
    type Error = Never;
    type Item = Either<PMChannelType<A>, ()>;

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
        // unwrap since it never fails
        match self.rx.poll_next(cx).unwrap() {
            // don't do anything if it's still pending
            Pending => {},

            // return the ready value
            Ready(Some(polled_msg)) => return Ok(Ready(Some(Left(polled_msg)))),

            // if it finished, just wait for the destructor to happen
            Ready(None) => {},
        }

        // then prioritize the self-destructor
        match self.self_destruct_rx.poll(cx).unwrap() {
            // just return pending if it's still pending
            Pending => return Ok(Pending),

            // return if self-destruct sequence is requested
            // this will happen if either sender has sent its message or the
            // sender dropped
            // NOTE: it may be better if we just drop the only sender. the
            // receiver will then return a None, thus
            Ready(_) => return Ok(Ready(Some(Right(())))),
        };
    }
}

impl<A> Context<A>
where
    A: Actor,
{
    pub(crate) fn new(
        actor: A,
        builder: ActorBuilder,
    ) -> (Context<A>, Addr<A>) {
        let (self_destructor, sd_rx) = ActorSelfDestructor::new();
        let self_destructor = Arc::new(self_destructor);
        let sd_weak = Arc::downgrade(&self_destructor);

        let (tx, rx) = channel(builder.buffer_size);

        let addr = Addr::new(tx.clone(), self_destructor);

        let immut_half = ContextImmutHalf {
            sd: sd_weak,
            tx,
        };

        let mut_half = ContextMutHalf {
            self_destruct_rx: sd_rx,
            rx,
            actor,
        };

        let ctx = Context {
            immut_half,
            mut_half,
        };

        (ctx, addr)
    }

    pub(crate) fn addr(&self) -> Addr<A> {
        self.immut_half.addr()
    }

    pub(crate) fn actor_mut(&mut self) -> &mut A {
        &mut self.mut_half().actor
    }

    pub(crate) fn actor(&self) -> &A {
        &self.mut_half_immut().actor
    }

    pub(crate) fn halves_mut(
        &mut self,
    ) -> (&ContextImmutHalf<A>, &mut ContextMutHalf<A>) {
        (&self.immut_half, &mut self.mut_half)
    }

    pub(crate) fn immut_half(&self) -> &ContextImmutHalf<A> {
        &self.immut_half
    }

    pub(crate) fn mut_half(&mut self) -> &mut ContextMutHalf<A> {
        &mut self.mut_half
    }

    pub(crate) fn mut_half_immut(&self) -> &ContextMutHalf<A> {
        &self.mut_half
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
            BeAliveOrDead::*,
            Either::*,
        };

        match self.mut_half().poll_next(cx).unwrap() {
            NotReady => Ok(Pending),
            Ready(None) => unimplemented!(), // Ok(Ready(Some(Alive))),
            Ready(Some(Right(_))) => Ok(Ready(Some(Dead))),

            Ready(Some(Left(msg))) => {
                // unpack the packed message
                let (tx, msg) = msg.into_parts();

                // process the message and send the response back
                let response =
                    msg.handle(&mut self.mut_half.actor, &self.immut_half);
                tx.send(response);

                // send the status that this is still alive
                Ok(Ready(Some(Alive)))
            },
        }
    }
}

impl<A> Drop for Context<A>
where
    A: Actor,
{
    fn drop(&mut self) {
        self.actor().on_stop();
    }
}
