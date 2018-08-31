use actor::Actor;
use message::{
    Message,
    MessageResponse,
    PackedMessage,
};

pub type PMUnboxedType<A: Actor> =
    PackedMessage<A, dyn Message<A, Response = dyn MessageResponse>>;
pub type PMChannelType<A: Actor> = Box<PMUnboxedType<A>>;

// tx: Sender<PackedMessage<A, Box<Message<A, Response = MessageResponse>>>>,
// rx: Receiver<Box<dyn Message<A, Response = MessageResponse>>>,
// type Item = Either<Box<dyn Message<A, Response = MessageResponse>>, ()>;
// sender: Sender<Box<dyn Message<A, Response = MessageResponse>>>,
