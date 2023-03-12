/// This class represents a look-ahead buffer that allows Debezium to accumulate binlog events and
/// decide if the last event in transaction is either `ROLLBACK` or `COMMIT`. The
/// incoming events are either supposed to be in transaction or out-of-transaction. When
/// out-of-transaction they are sent directly into the destination handler. When in transaction the
/// event goes through the buffering.
///
/// The reason for the buffering is that the binlog contains rolled back transactions in some cases.
/// E.g. that's the case when a temporary table is dropped (see DBZ-390). For rolled back
/// transactions we may not propagate any of the contained events, hence the buffering is applied.
///
/// The transaction start is identified by a `BEGIN` event. Transaction is ended either by
/// `COMMIT` event or by `XID` an event.
///
/// If there are more events that can fit to the buffer then:
///
/// * Binlog position is recorded for the first event not fitting into the buffer
/// * Binlog position is recorded for the commit event
/// * Buffer content is sent to the final handler
/// * Binlog position is rewound and all events between the above recorded positions are sent to the
///   final handler
pub struct EventBuffer {}
