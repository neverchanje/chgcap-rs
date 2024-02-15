#[allow(dead_code)]
mod binlog_stream;
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod connection;
#[allow(dead_code)]
mod event;
#[allow(dead_code)]
mod metrics;
#[allow(dead_code)]
mod schema;
#[allow(dead_code)]
mod snapshot;
#[allow(dead_code)]
mod source;
#[allow(dead_code)]
mod state;

#[macro_use]
extern crate derive_builder;

pub use binlog_stream::BinlogStream;
pub use config::{SourceConfig, SourceConfigBuilder};
pub use event::{Event, EventData};
pub use source::Source;
