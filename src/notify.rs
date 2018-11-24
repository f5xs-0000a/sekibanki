use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use actor::Handles;
use message::Message;

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct NotifyHandle<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    inner: Arc<()>,
    phantom_a: PhantomData<A>,
    phantom_m: PhantomData<M>,
}

#[derive(Debug)]
pub(crate) struct CrateNotifyHandle<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    inner: Weak<()>,
    phantom_a: PhantomData<A>,
    phantom_m: PhantomData<M>,
}

////////////////////////////////////////////////////////////////////////////////

impl<A, M> CrateNotifyHandle<A, M>
where
    A: Handles<M>,
    M: Message<A>,
{
    pub fn is_alive(&self) -> bool {
        Weak::upgrade(&self.inner).is_some()
    }
}

pub(crate) fn new_notify<A, M>() -> (NotifyHandle<A, M>, CrateNotifyHandle<A, M>)
where
    A: Handles<M>,
    M: Message<A>,
{
    let inner = Arc::new(());

    let cnh = CrateNotifyHandle {
        inner: Arc::downgrade(&inner),
        phantom_a: PhantomData,
        phantom_m: PhantomData,
    };

    let nh = NotifyHandle {
        inner,
        phantom_a: PhantomData,
        phantom_m: PhantomData,
    };

    (nh, cnh)
}
