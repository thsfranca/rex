//! Internal stream event consumer for the Rex desktop client.
//!
//! Parses `docs/STREAM_EVENTS.md` events into a [`TurnState`] machine and operator-facing messages.

mod consumer;
mod event;
#[cfg(feature = "grpc")]
mod grpc;
mod messaging;
mod truncate;
mod turn_state;

pub use consumer::StreamConsumer;
pub use event::{parse_stream_line, StreamEvent, StreamEventKind};
#[cfg(feature = "grpc")]
pub use grpc::stream_event_from_grpc;
pub use messaging::{LifecycleContext, LifecyclePhase, OperatorMessaging};
pub use truncate::{truncate_display, TRUNCATION_MARKER};
pub use turn_state::{ToolCard, TurnPhase, TurnState, UiEffect};
