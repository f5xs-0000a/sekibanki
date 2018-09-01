use futures_executor::ThreadPool;
use futures_util::StreamExt;
use futures::{
    task::Context as FutContext,
};

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
        mut pool: ThreadPool,
    ) -> Addr<Self> {
        use futures::future::ok;
        use futures_util::future::lazy;

        // create the context and the first address
        let (mut ctx, addr) = Context::new(self, builder);

        // create a closure wrapped in `ok()` future that will call the function
        // to poll into ThreadPool.
        //
        // the child thread will only be created so the function blocks on
        // itself but the actual thread that will perform the logic would be
        // the threadpool
        let other_thread = lazy(move |_fut_ctx: &mut FutContext| {
            // call the `on_start()` method of the actor
            ctx.actor_mut().on_start();

            // put the context in an infinite loop of waiting
            let loop_future = ctx.fuse().for_each(|_| ok(()));

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
