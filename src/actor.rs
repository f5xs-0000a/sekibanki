use futures::{
    Future,
    Stream,
};
use tokio_threadpool::Sender as TPSender;

use address::Addr;
use context::{
    Context,
    ContextImmutHalf,
};
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
        pool: TPSender,
    ) -> Addr<Self> {
        use futures::future::ok;

        // create the context and the first address
        let (mut ctx, addr) = Context::new(self, builder, pool.clone());

        {
            // temporarily split the context
            let (immut_half, mut_half) = ctx.halves_mut();

            // call the `on_start()` method of the actor
            mut_half.actor_mut().on_start(immut_half);
        }

        // put the context into a waiting loop
        pool.spawn(Box::new(ctx.fuse().for_each(|_| ok(())).map(|_| ())))
            .expect("Unable to spawn new context...");

        addr
    }

    fn on_start(
        &mut self,
        _ctx: &ContextImmutHalf<Self>,
    ) {
    }

    fn on_stop(
        &mut self,
        _ctx: &ContextImmutHalf<Self>,
    ) {
    }
}

pub trait Handles<M>: Actor
where
    M: Message, {
    type Response: Send;

    fn handle(
        &mut self,
        msg: M,
        ctx: &ContextImmutHalf<Self>,
    ) -> Self::Response;
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, PartialEq, Eq, Clone, Hash)]
/// Builder for the `Actor`.
pub struct ActorBuilder {}
