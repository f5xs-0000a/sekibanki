#![feature(pin)]

extern crate either;
extern crate futures_channel;
extern crate futures_core as futures;
extern crate futures_executor;
extern crate futures_util;

pub mod actor;
pub mod address;
pub mod context;
pub mod message;
pub mod response;

pub use actor::{
    Actor,
    ActorBuilder,
    Handles,
};
pub use address::Addr;
pub use context::ContextImmutHalf;
pub use message::Message;
pub use response::ResponseFuture;
