use futures::{
    task::Context as FutContext,
    Future,
    Never,
    Poll,
};
use futures_channel::oneshot::Receiver as OShReceiver;

use actor::Actor;
use message::Message;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct ResponseFuture<A, M>
where
    A: Actor,
    M: Message<A>, {
    rx:     OShReceiver<M::Response>,
    polled: bool,
}

////////////////////////////////////////////////////////////////////////////////

impl<A, M> ResponseFuture<A, M>
where
    A: Actor,
    M: Message<A>,
{
    pub(crate) fn with_receiver(
        rx: OShReceiver<M::Response>,
    ) -> ResponseFuture<A, M> {
        ResponseFuture {
            rx,
            polled: false,
        }
    }
}

impl<A, M> Future for ResponseFuture<A, M>
where
    A: Actor,
    M: Message<A>,
{
    type Error = Never;
    type Item = M::Response;

    fn poll(
        &mut self,
        cx: &mut FutContext,
    ) -> Poll<Self::Item, Self::Error> {
        self.rx.poll(cx).map_err(|_| unreachable!())
    }
}
