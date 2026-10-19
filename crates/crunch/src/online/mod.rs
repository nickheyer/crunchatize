// Two player play, the host simulates and streams its ticks while the helper's input flows back
pub mod dialog;
pub mod hash;
pub mod session;
pub mod wire;

pub use session::{Hand, Notice, Phase, Role, Session, Who};
