use actor::Actor;
use context::ContextImmutHalf;
use message::Envelope;

pub (crate) type PMChannelType<A: Actor> = Envelope<A>;

//Box<dyn FnOnce(&mut A, &ContextImmutHalf<A>)>;

// tx: Sender<PackedMessage<A, Box<Message<A, Response = MessageResponse>>>>,
// rx: Receiver<Box<dyn Message<A, Response = MessageResponse>>>,
// type Item = Either<Box<dyn Message<A, Response = MessageResponse>>, ()>;
// sender: Sender<Box<dyn Message<A, Response = MessageResponse>>>,
