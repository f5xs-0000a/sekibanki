use futures_executor::ThreadPool;
use futures_util::StreamExt;
use futures::{
    task::Context as FutContext,
};
use futures::executor::Executor;
use futures_util::FutureExt;

use address::Addr;
use context::Context;
use context::ContextImmutHalf;
use message::Message;

////////////////////////////////////////////////////////////////////////////////

/// Unit execution context that can send and process messages.
///
/// An actor is started once `start_actor()` is called. An actor is stopped if
/// all addresses pointing to it are dropped.
pub trait Actor: Sized + 'static + Send {
    fn start_actor(
        self,
        builder: ActorBuilder,
        mut pool: ThreadPool,
    ) -> Addr<Self> {
        use futures::future::ok;
        use futures_util::future::lazy;

        // create the context and the first address
        let (mut ctx, addr) = Context::new(self, builder, pool.clone());

        {
            // temporarily split the context
            let (immut_half, mut_half) = ctx.halves_mut();

            // call the `on_start()` method of the actor
            mut_half.actor_mut().on_start(immut_half);
        }

        // put the context into a waiting loop
        pool.spawn(
            Box::new(
                ctx.fuse()
                    .for_each(|_| ok(()))
                    .map(|_| ())
            )
        );

        addr
    }

    fn on_start(&mut self, ctx: &ContextImmutHalf<Self>) {
    }

    fn on_stop(&mut self, ctx: &ContextImmutHalf<Self>) {
    }
}

pub trait Handles<M>: Actor
where M: Message {
    type Response: Send;

    fn handle(&mut self, msg: M, ctx: &ContextImmutHalf<Self>) -> Self::Response;
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, PartialEq, Eq, Clone, Hash)]
/// Builder for the `Actor`.
pub struct ActorBuilder {
}
