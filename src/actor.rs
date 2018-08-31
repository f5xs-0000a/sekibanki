use futures_executor::ThreadPool;
use futures_util::StreamExt;

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
        use futures::future::ok;
        use futures_util::future::lazy;

        // create the context and the first address
        let (ctx, addr) = Context::new(self, builder);

        // create a closure wrapped in `ok()` future that will call the function
        // to poll into ThreadPool.
        //
        // the child thread will only be created so the function blocks on
        // itself but the actual thread that will perform the logic would be
        // the threadpool
        let other_thread = lazy(move |_| {
            // call the `on_start()` method of the actor
            ctx.actor_mut().on_start();

            // put the context in an infinite loop of waiting
            // just so you know, this idiom is a do-while loop
            let loop_future = ctx.fuse().for_each(|_| ok(()));
            // TODO: because the context is fused, it must return a None when
            // it's ready to die.

            // the `on_stop()` isn't called here, it's called on the destructor
            // of the context.

            loop_future
        });

        // put the `other_thread` into a waiting loop
        pool.run(other_thread);

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
        use self::BeAliveOrDead::*;

        match self {
            &Alive => true,
            _ => false,
        }
    }

    pub fn is_dead(&self) -> bool {
        use self::BeAliveOrDead::*;

        match self {
            &Dead => true,
            _ => false,
        }
    }
}
