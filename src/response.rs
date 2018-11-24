use futures::{
    sync::oneshot::{Canceled, Receiver as OneShotReceiver},
    Future, Poll,
};

use actor::Handles;
use message::Message;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct ResponseFuture<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    rx: OneShotReceiver<A::Response>,
    polled: bool,
}

////////////////////////////////////////////////////////////////////////////////

impl<A, M> ResponseFuture<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    pub(crate) fn with_receiver(rx: OneShotReceiver<A::Response>) -> ResponseFuture<A, M> {
        ResponseFuture { rx, polled: false }
    }
}

impl<A, M> Future for ResponseFuture<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    type Error = Canceled;
    type Item = A::Response;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.rx.poll()
    }
}
