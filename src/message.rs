use actor::Actor;
use context::ContextImmutHalf;

///////////////////////////////////////////////////////////////////////////////

/// Trait for objects that can be sent to actors as messages
pub trait Message<A>
where
    A: Actor, {
    type Response: MessageResponse;

    fn handle(
        self,
        actor: &mut A,
        ctx: &ContextImmutHalf<A>,
    ) -> Self::Response;
}

pub trait MessageResponse {}

////////////////////////////////////////////////////////////////////////////////

impl<A, M> Message<A> for Box<M>
where
    A: Actor,
    M: Message<A>,
{
    type Response = M::Response;

    fn handle(
        self,
        actor: &mut A,
        ctx: &ContextImmutHalf<A>,
    ) -> Self::Response {
        (*self).handle(actor, ctx)
    }
}

impl<T> MessageResponse for T {
}
