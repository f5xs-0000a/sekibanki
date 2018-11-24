use actor::Handles;
use either::Either;
use futures::{
    sync::{
        mpsc::{unbounded, UnboundedReceiver as Receiver, UnboundedSender as Sender},
        oneshot::Receiver as OneShotReceiver,
    },
    Async, Future, Stream,
};
use message::Message;
use std::{
    sync::{Arc, Weak},
    time::Duration,
};
use tokio_threadpool::Sender as TPSender;

use actor::{Actor, ActorBuilder};
use address::{ActorSelfDestructor, Addr};
use message::Envelope;
use notify::{new_notify, NotifyHandle};

////////////////////////////////////////////////////////////////////////////////

/// The half of the context that is immutable.
pub struct ContextImmutHalf<A>
where
    A: Actor,
{
    // this attribute is needed so he can make an exact clone of the address
    // without the need of the address
    // and this is guaranteed to be alive for most intents and purposes
    sd: Weak<ActorSelfDestructor>,

    tx: Sender<Envelope<A>>,

    // tx: Sender<Box<dyn Message<A, Response = MessageResponse>>>,
    //
    // a copy of the threadpool, in case another actor needs to be started
    pool: TPSender,
}

/// The half of the context that is mutable.
pub(crate) struct ContextMutHalf<A>
where
    A: Actor,
{
    self_destruct_rx: OneShotReceiver<()>,
    rx: Receiver<Envelope<A>>,
    actor: A,
}

/// The context of the actor.
pub(crate) struct Context<A>
where
    A: Actor,
{
    immut_half: ContextImmutHalf<A>,
    mut_half: ContextMutHalf<A>,
}

////////////////////////////////////////////////////////////////////////////////

impl<A> ContextImmutHalf<A>
where
    A: Actor + 'static,
{
    pub fn addr(&self) -> Addr<A> {
        // as a consequence of the initialization of the `sd` attribute, the
        // immutable half cannot be accessed when the actor is stopping.
        let sd = Weak::upgrade(&self.sd)
            .expect("Attempted to upgrade an already dead weak shared pointer.");

        Addr::new(self.tx.clone(), sd)
    }

    pub fn has_addr(&self, addr: &Addr<A>) -> bool {
        let sd = Weak::upgrade(&self.sd)
            .expect("Attempted to upgrade an already dead weak shared pointer.");

        Arc::ptr_eq(&sd, &addr.sd)
    }

    pub fn threadpool(&self) -> &TPSender {
        &self.pool
    }

    pub fn notify<M>(&self, msg: M)
    where
        A: Handles<M>,
        M: Message<A> + 'static,
    {
        // NOTE: the implementation is the same as `Addr::send()`
        self.tx
            .unbounded_send(Envelope::new_without_response(msg))
            .expect(
                "Message sending unexpectedly failed. Perhaps the receiver \
                 address was also unexpectedly dropped?",
            );
    }

    pub fn notify_later<M>(&self, msg: M, sleep: Duration) -> NotifyHandle<A, M>
    where
        A: Handles<M>,
        M: Message<A> + Sync + 'static,
    {
        // NOTE: the implementation is the same as `Addr::send_later()`

        let new_tx = self.tx.clone();
        let (nh, cnh) = new_notify();

        // spawn a thread that will sleep for a given duration then forget about
        // it
        ::std::thread::spawn(move || {
            ::std::thread::sleep(sleep);

            // if the handle is still not dropped...
            if cnh.is_alive() {
                // send the message but don't care if the send failed
                new_tx.unbounded_send(Envelope::new_without_response(msg));
            }
        });

        nh
    }
}

impl<A> ContextMutHalf<A>
where
    A: Actor,
{
    pub fn actor(&self) -> &A {
        &self.actor
    }

    pub fn actor_mut(&mut self) -> &mut A {
        &mut self.actor
    }

    fn next(&mut self, ctx: &ContextImmutHalf<A>) -> Option<Envelope<A>> {
        use tokio_threadpool::blocking;

        // the error value of receivers is Never so they will never error, i.e.
        // they can be safely unwrapped.

        // check if rx has a message ready to be processed
        // unwrap since it never fails
        match self.rx.poll().unwrap() {
            // the values caught by this branch would either be a message (Some)
            // or a None
            // since msg becoming `None` would mean that all senders are
            // dropped, miraculously including the one in the Context, it would
            // mean that the actor is ready to be dropped
            Async::Ready(msg) => return msg,

            _ => {}
        }

        // check if the self-desturctor is ready to be processed
        if let Async::Ready(_) = self.self_destruct_rx.poll().unwrap() {
            // if the self-destructor is mysteriously dropped without sending
            // its message, it is considered dropped.
            // therefore, either way, if the receiver is ready, this struct is
            // ready to be dropped
            return None;
        }

        self.actor.on_message_exhaust(ctx);

        // create a waiting stream of the two receivers
        let left_stream = self.rx.by_ref().map(|msg| Either::Left(msg));

        let right_stream = (&mut self.self_destruct_rx)
            .into_stream()
            .map(|_| Either::Right(()))
            .map_err(|_| ());

        let select = left_stream.select(right_stream).take(1);

        match blocking(|| select.wait().next()) {
            // the threadpool is dropped. what do we do now?
            Err(e) => panic!("{:?}", e),

            // the threadpool has reached its maximum blocking threads. what do
            // we do now?
            Ok(Async::NotReady) => unimplemented!(),

            Ok(Async::Ready(msg)) => {
                // since the msg came from an iterator's `next()`, it's an
                // option. we have to match it. and it may be an error too
                match msg {
                    // either the left stream's senders have been dropped or
                    // the right stream's sender failed to send. either way, the
                    // actor must be dropped.
                    None | Some(Err(_)) | Some(Ok(Either::Right(_))) => None,

                    Some(Ok(Either::Left(msg))) => Some(msg),
                }
            }
        }
    }
}

impl<A> Context<A>
where
    A: Actor + 'static,
{
    pub(crate) fn new(actor: A, _builder: ActorBuilder, pool: TPSender) -> (Context<A>, Addr<A>) {
        // create the self-destructor, the message towards the self-desturcto,
        // and the weak pointer to the self-destructor
        let (self_destructor, sd_rx) = ActorSelfDestructor::new();
        let self_destructor = Arc::new(self_destructor);
        let sd_weak = Arc::downgrade(&self_destructor);

        // create the message channel
        let (tx, rx) = unbounded();

        // create the address
        let addr = Addr::new(tx.clone(), self_destructor);

        let immut_half = ContextImmutHalf {
            sd: sd_weak,
            tx,
            pool,
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

    pub(crate) fn halves_mut(&mut self) -> (&ContextImmutHalf<A>, &mut ContextMutHalf<A>) {
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

    pub(crate) fn iter_through(&mut self) {
        // iterate throug the underlying iterator
        loop {
            let msg = self.mut_half.next(&self.immut_half);

            match msg {
                // no more messages or the self-destruct sequence has initiated
                None => break,

                // a message was received; handle it
                Some(msg) => msg.handle(&mut self.mut_half.actor, &self.immut_half),
            }
        }
    }
}
