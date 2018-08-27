extern crate futures_core as futures;
extern crate futures_executor;
extern crate futures_channel;

////////////////////////////////////////////////////////////////////////////////

use futures_channel::mpsc::channel;
use futures_channel::mpsc::Sender;
use futures_channel::mpsc::Receiver;
use futures_channel::oneshot::channel as osh_channel;
use futures_channel::oneshot::Sender as OShSender;
use futures_channel::oneshot::Receiver as OShReceiver;
use futures_executor::ThreadPool;
use futures::task::Context as FutContext;
use futures::Stream;
use futures::Future;
use std::sync::Arc;
use futures::future::lazy;
use std::sync::Weak;
use std::mem::PinMut;
use futures::Poll;
use either::Either;
use futures::never::Never;

////////////////////////////////////////////////////////////////////////////////

/// Unit execution context that can send and process messages.
///
/// An actor is started once `start_actor()` is called. An actor is stopped if
/// all addresses pointing to it are dropped.
pub trait Actor {
    fn start_actor(self, builder: ActorBuilder, pool: ThreadPool) -> Addr<Actor> {
        // create the channel to send messages to the actor
        let (rx, tx) = channel(builder.buffer_size);

        let ctx = Context::new(self);

        // create the address
        let addr = ctx.addr();

        // create a closure wrapped in `ok()` future that will call the function
        // to poll into ThreadPool.
        //
        // the child thread will only be created so the function blocks on
        // itself but the actual thread that will perform the logic would be
        // the threadpool
        let other_thread = lazy(move || {
            // call the `on_start()` method of the actor
            ctx.actor.on_start();

            // put the context in an infinite loop of waiting
            // just so you know, this idiom is a do-while loop
            while {
                (&mut ctx.mut_half).next() == BeAliveOrDead::Alive
            } { }

            // call the `on_stop()` method of the actor
            ctx.actor.on_stop();
        });

        // put the `other_thread` into a waiting loop
        unimplemented!();

        addr
    }

    fn on_start(&mut self) {
    }

    fn on_stop(&mut self) {
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Builder for the `Actor`.
pub struct ActorBuilder {
    pub buffer_size: usize,
}

////////////////////////////////////////////////////////////////////////////////

/// The half of the context that is immutable.
pub struct ContextImmutHalf<A>
where A: Actor {
    tx: Sender<Box<dyn Message<A>>>,

    // this attribute is needed so he can make an exact clone of the address
    // without the need of the address
    // and this is guaranteed to be alive for most intents and purposes
    sd: Weak<ActorSelfDestructor>,
}

impl<A> ContextImmutHalf<A>
where A: Actor {
    pub fn addr(&self) -> Addr<A> {
        // as a consequence of the initialization of the `sd` attribute, the
        // immutable half cannot be accessed when the actor is stopping.
        // TODO: you need your own method
        Addr {
            tx: self.tx.clone(),

            // it's got to be alive
            sd: Arc::upgrade(&self.sd).unwrap(),
        }
    }
}

pub enum BeAliveOrDead {
    Alive,
    Dead,
}

/// The half of the context that is mutable.
struct ContextMutHalf<A>
where A: Actor {
    rx: Receiver<Box<dyn Message<A>>>,
    self_destruct_rx: Receiver<()>,
    actor: A,
}

impl<A> Stream for ContextMutHalf<A> {
    type Item = Either<Box<dyn Message<A>>, ()>;

    fn poll_next(self: PinMut<Self>, cx: &mut FutContext) -> Poll<Option<Self::Item>, Never> {
        use futures::Async::Ready;
        use futures::Async::Pending;
        use Either::*;

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

////////////////////////////////////////////////////////////////////////////////

/// The context of the actor.
pub struct Context<A>
where A: Actor {
    immut_half: ContextImmutHalf<A>,
    mut_half: ContextMutHalf<A>,
}

impl<A> Context<A> {
    pub (crate) fn new(actor: A) -> Context<A> {
        unimplemented!() // TODO: this is your last edit
    }
}

impl<A> Stream for Context<A>
where A: Actor {
    type Item = BeAliveOrDead;

    fn poll_next(self: PinMut<Self>, cx: &mut FutContext) -> Poll<Option<Self::Item>, Never> {
        let message = match (&mut self.mut_half).next() {
            NotReady => return NotReady,
            Ready(Right(_)) => return Ready(BeAliveOrDead::Dead),
            Ready(Left(msg)) => msg
        };

        message.handle(&mut self.mut_half.actor, &self.immut_half);
        Ready(BeAliveOrDead::Alive);
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Trait for objects that can be sent to actors as messages
pub trait Message<A>
where A: Actor {
    type Response;

    fn handle(self, actor: &mut A, ctx: &ContextImmutHalf<A>) -> Self::Response;
}

////////////////////////////////////////////////////////////////////////////////

pub struct ResponseFuture<A, M>
where A: Actor,
      M: Message<A> {
    rx: OShReceiver<M::Response>,
    polled: bool,
}

impl<A, M> ResponseFuture<A, M>
where A: Actor,
      M: Message<A> {
    pub (crate) fn with_receiver(rx: OShReceiver<M::Response>) -> ResponseFuture<A, M> {
        ResponseFuture {
            rx,
            polled: false,
        }
    }
}

impl<A, M> Future for ResponseFuture<A, M>
where A: Actor,
      M: Message<A> {
    type Item = M::Response;
    type Error = Never;

    fn poll(&mut self, cx: &mut FutContext) -> Poll<Self::Item, Self::Error> {
        self.rx.poll()
            .map_err(|_| unreachable!())
    }
}

////////////////////////////////////////////////////////////////////////////////

/// When this object is dropped, it sends a message to the actor to kill itself.
///
/// This is meant to be wrapped in an `Arc` and a clone is held by an address.
/// If all addresses pointing to a specific actor were dropped, the `drop()`
/// method of this struct gets called, sending the signal to the actor to kill
/// itself.
struct ActorSelfDestructor {
    tx: Option<OShSender<()>>,
}

impl Drop for ActorSelfDestructor {
    fn drop(&mut self) {
        self.tx.take().unwrap().send(());
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct PackedMessage<A, M>
where A: Actor,
      M: Message<A> {
    msg: M,
    tx: OShSender<M::Response>,
}

impl<A, M> PackedMessage<A, M>
where A: Actor,
      M: Message<A> {
    pub fn new_with_response_channel(msg: M) -> (PackedMessage<A, M>, OShReceiver<M::Response>) {
        let (rx, tx) = osh_channel();

        let pm = PackedMessage {
            msg,
            tx,
        };

        (pm, rx)
    }

    fn handle(self, actor: &mut Actor, immut_half: &ContextImmutHalf<A>) {
        // process the message and get the response
        let response = self.msg.handle(actor, immut_half);

        // send the response back to the sender
        self.tx.send(response);
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Address pointing to an actor. Used to send messages to an actor.
pub struct Addr<A>
where A: Actor {
    tx: Sender<Box<dyn Message<A>>>,
    sd: Arc<ActorSelfDestructor>,
}

impl<A> Addr<A>
where A: Actor {
    /// Called during the creation of the actor
    pub (crate) fn on_new_actor(
        sender: Sender<Box<dyn Message<A>>>,
        destructor: Sender<()>,
    ) -> Addr<A> {
        // create the self-destructor
        let sd = Arc::new(ActorSelfDestructor{ tx: destructor });
        
        Addr {
            tx: sender,
            sd,
        }
    }

    /// Send a message to the actor
    pub fn send(&mut self, msg: Box<PackedMessage<A, dyn Message<A>>>) -> ResponseFuture {
        // wrap the message and receive the response channel
        let (pm, rx) = PackedMessage::new_with_response_channel(msg);

        // send the packed message
        self.tx.send(Box::new(pm));

        ResponseFuture::with_receiver(rx)
    }
}
