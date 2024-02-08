use crate::connection::BinlogPosition;

pub struct SnapshotReader {
    low_watermark: BinlogPosition,
    high_watermark: BinlogPosition,
}
