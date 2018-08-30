use futures_channel::mpsc::channel;
use futures_executor::ThreadPool;
use futures_util::future::lazy;

use address::Addr;
use context::Context;

////////////////////////////////////////////////////////////////////////////////

/// Unit execution context that can send and process messages.
///
/// An actor is started once `start_actor()` is called. An actor is stopped if
/// all addresses pointing to it are dropped.
pub trait Actor: Sized {
    fn start_actor(
        self,
        builder: ActorBuilder,
        pool: ThreadPool,
    ) -> Addr<Self> {
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
        let other_thread = lazy(move |_| {
            // call the `on_start()` method of the actor
            ctx.actor.on_start();

            // put the context in an infinite loop of waiting
            // just so you know, this idiom is a do-while loop
            while { (&mut ctx.mut_half).next() == BeAliveOrDead::Alive } {}

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

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
/// Builder for the `Actor`.
pub struct ActorBuilder {
    pub buffer_size: usize,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum BeAliveOrDead {
    Alive,
    Dead,
}

////////////////////////////////////////////////////////////////////////////////

impl BeAliveOrDead {
    pub fn is_alive(&self) -> bool {
        *self == *BeAliveOrDead::Alive
    }

    pub fn is_dead(&self) -> bool {
        *self == *BeAliveOrDead::Dead
    }
}
