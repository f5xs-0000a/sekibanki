use futures::{
    task::Context as FutContext,
    Future,
    Never,
    Poll,
};
use futures_channel::oneshot::Receiver as OneShotReceiver;

use actor::Handles;
use message::Message;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct ResponseFuture<A, M>
where
    A: Handles<M>,
    M: Message, {
    rx:     OneShotReceiver<A::Response>,
    polled: bool,
}

////////////////////////////////////////////////////////////////////////////////

impl<A, M> ResponseFuture<A, M>
where
    A: Handles<M>,
    M: Message,
{
    pub(crate) fn with_receiver(
        rx: OneShotReceiver<A::Response>,
    ) -> ResponseFuture<A, M> {
        ResponseFuture {
            rx,
            polled: false,
        }
    }
}

impl<A, M> Future for ResponseFuture<A, M>
where
    A: Handles<M>,
    M: Message,
{
    type Error = Never;
    type Item = A::Response;

    fn poll(
        &mut self,
        cx: &mut FutContext,
    ) -> Poll<Self::Item, Self::Error> {
        self.rx.poll(cx).map_err(|_| unreachable!())
    }
}
