use std::pin::Pin;
use std::task::Poll;

use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use mysql_async::binlog::events::{
    DeleteRowsEvent, Event, EventData, GtidEvent, IncidentEvent, QueryEvent, RotateEvent,
    RowsEventData, RowsQueryEvent, UpdateRowsEvent, WriteRowsEvent,
};
use mysql_async::BinlogStream;
use pin_project::pin_project;

use crate::connection::get_binlog_stream;
use crate::record::MysqlTableEvent;
use crate::source::{MysqlSource, SourceContext};

#[pin_project]
pub struct MysqlCdcStream {
    #[pin]
    binlog_stream: BinlogStream,
    #[pin]
    handler: MysqlEventHandler,
}

impl MysqlCdcStream {
    pub async fn new(source: &MysqlSource) -> Result<Self> {
        let binlog_stream = get_binlog_stream(&source.pool, &source.cfg).await?;

        Ok(Self {
            binlog_stream,
            handler: MysqlEventHandler::new(),
        })
    }
}

pub struct MysqlEventHandler {
    ctx: SourceContext,
}

impl MysqlEventHandler {
    pub fn new() -> Self {
        Self {
            ctx: SourceContext::default(),
        }
    }

    fn handle_event(&mut self, event: Event) -> Result<Option<MysqlTableEvent>> {
        let event_data = match event.read_data()? {
            Some(data) => data,
            None => return Ok(None), // Skip empty event.
        };
        self.ctx.server_id = event.header().server_id();

        match event_data {
            EventData::QueryEvent(e) => self.handle_query(e),
            EventData::RotateEvent(e) => self.handle_rotate(e),
            EventData::HeartbeatEvent => self.handle_server_heartbeat(),
            EventData::RowsQueryEvent(e) => self.handle_rows_query(e),
            EventData::GtidEvent(e) => self.handle_gtid_event(e),
            EventData::RowsEvent(e) => {
             return  Ok(Some(  self.handle_rows_event(e)?));
            }
            _ => {
                // Unhandled events.
            }
            // EventData::UnknownEvent => todo!(),
            // EventData::StartEventV3(_) => todo!(),
            // EventData::StopEvent => todo!(),
            // EventData::IntvarEvent(_) => todo!(),
            // EventData::LoadEvent(_) => todo!(),
            // EventData::SlaveEvent => todo!(),
            // EventData::CreateFileEvent(_) => todo!(),
            // EventData::AppendBlockEvent(_) => todo!(),
            // EventData::ExecLoadEvent(_) => todo!(),
            // EventData::DeleteFileEvent(_) => todo!(),
            // EventData::NewLoadEvent(_) => todo!(),
            // EventData::RandEvent(_) => todo!(),
            // EventData::UserVarEvent(_) => todo!(),
            // EventData::FormatDescriptionEvent(_) => todo!(),
            // EventData::XidEvent(_) => todo!(),
            // EventData::BeginLoadQueryEvent(_) => todo!(),
            // EventData::ExecuteLoadQueryEvent(_) => todo!(),
            // EventData::TableMapEvent(_) => todo!(),
            // EventData::PreGaWriteRowsEvent(_) => todo!(),
            // EventData::PreGaUpdateRowsEvent(_) => todo!(),
            // EventData::PreGaDeleteRowsEvent(_) => todo!(),
            // EventData::IncidentEvent(_) => todo!(),
            // EventData::IgnorableEvent(_) => todo!(),
            // EventData::AnonymousGtidEvent(_) => todo!(),
            // EventData::PreviousGtidsEvent(_) => todo!(),
            // EventData::TransactionContextEvent(_) => todo!(),
            // EventData::ViewChangeEvent(_) => todo!(),
            // EventData::XaPrepareLogEvent(_) => todo!(),
        };
        Ok(None)
    }

    /// Handle a [mysql_async::binlog::events::XidEvent] or a COMMIT statement.
    fn handle_txn_completion(&self) {}

    /// Handle the supplied event that signals that mysqld has stopped.
    fn handle_server_stop(&self) {
        info!("Server stopped")
    }

    /// Handle the supplied event that is sent by a primary to a replica to let the replica
    /// know that the primary is still alive. Not written to a binary log.
    fn handle_server_heartbeat(&self) {}

    /// Handle the supplied event that signals that an out of the ordinary event that occurred on
    /// the master. It notifies the replica that something happened on the primary that might
    /// cause data to be in an inconsistent state.
    fn handle_server_incident(&self, e: IncidentEvent) {
        // TODO: Failure handling: https://github.com/neverchanje/chgcap-rs/issues/2.
        error!("server incident: {}", e.message())
    }

    /// Handle the supplied event that signals the logs are being rotated. This means that either
    /// the server was restarted, or the binlog has transitioned to a new file.
    /// In either case, subsequent table numbers will be different than those seen to this point.
    fn handle_rotate(&mut self, e: RotateEvent) {
        self.ctx.current_binlog_pos = e.position();
        self.ctx.current_binlog_filename = e.name().to_string();
    }

    /// Handle the supplied event that signals the beginning of a GTID transaction.
    /// We don't yet know whether this transaction contains any events we're interested in,
    /// but we have to record it so that we know the position of this event and know we've
    /// processed the binlog to this point.
    ///
    /// Note that this captures the current GTID and complete GTID set, regardless of whether
    /// the connector is filtering the GTID set upon connection. We do this because we actually
    /// want to capture all GTID set values found in the binlog, whether or not we process them.
    /// However, only when we connect do we actually want to pass to MySQL only those GTID ranges
    /// that are applicable per the configuration.
    fn handle_gtid_event(&mut self, e: GtidEvent) {
        self.ctx.current_gtid = Some(e);
    }

    /// Handle the supplied event [RowsQueryEvent] by recording the original SQL query that
    /// generated the event.
    fn handle_rows_query(&mut self, e: RowsQueryEvent) {
        self.ctx.current_query = e.query().to_string();
    }

    /// Handle the supplied event with an [QueryEvent] by possibly recording the DDL statements
    /// as changes in the MySQL schemas.
    fn handle_query(&mut self, e: QueryEvent) {
        debug!("Received query command: {:?}", e);

        let query = e.query().to_string().trim().to_string();
        if query.eq_ignore_ascii_case("BEGIN") {
            self.ctx.thread_id = Some(e.thread_id());
            return;
        }
        if query.eq_ignore_ascii_case("COMMIT") {
            self.handle_txn_completion();
            return;
        }

        let stmt_prefix = query[..7].to_uppercase();
        if stmt_prefix.starts_with("XA ") {
            info!("Ignore XA transaction");
            return;
        } else if stmt_prefix == "INSERT " || stmt_prefix == "UPDATE " || stmt_prefix == "DELETE " {
            warn!("Received DML '{}' for processing, binlog probably contains events generated with statement or mixed based replication format", query);
            return;
        }
        if query.eq_ignore_ascii_case("ROLLBACK") {
            unimplemented!("{}", query)
        }
    }

    /// Generate source records for the supplied event.
    fn handle_rows_event(&self, e: RowsEventData) -> Result<MysqlTableEvent> {
        match e {
            RowsEventData::WriteRowsEvent(e) => self.handle_write_rows(e),
            RowsEventData::UpdateRowsEvent(e) => self.handle_update_rows(e),
            RowsEventData::DeleteRowsEvent(e) => self.handle_delete_rows(e),
            RowsEventData::PartialUpdateRowsEvent(_) => todo!(),

            RowsEventData::DeleteRowsEventV1(_)
            | RowsEventData::WriteRowsEventV1(_)
            | RowsEventData::UpdateRowsEventV1(_) => Err(anyhow!(
                "Received an unsupported V1 event, which is for mariadb and mysql 5.1.15-5.6.x."
            )), // TODO: may mark it as an unrecoverable error.
        }
    }

    fn handle_write_rows(&self, _e: WriteRowsEvent) -> Result<MysqlTableEvent> {
        todo!()
    }

    fn handle_update_rows(&self, _e: UpdateRowsEvent) -> Result<MysqlTableEvent> {
        todo!()
    }

    fn handle_delete_rows(&self, _e: DeleteRowsEvent) -> Result<MysqlTableEvent> {
        todo!()
    }
}

impl futures_core::stream::Stream for MysqlCdcStream {
    type Item = Result<MysqlTableEvent>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            return match this.binlog_stream.as_mut().poll_next(cx) {
                Poll::Ready(t) => match t {
                    Some(event_result) => match event_result {
                        Ok(event) => match this.handler.handle_event(event) {
                            Ok(change) => match change {
                                Some(c) => Poll::Ready(Some(Ok(c))),
                                None => continue, // Skip this event.
                            },
                            Err(e) => Poll::Ready(Some(Err(e))),
                        },
                        Err(err) => Poll::Ready(Some(Err(anyhow!(err)))),
                    },
                    None => Poll::Ready(None), // Completed.
                },
                Poll::Pending => Poll::Pending,
            };
        }
    }
}
