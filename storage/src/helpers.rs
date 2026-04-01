use chrono::{DateTime, Utc};
use uuid::Uuid;

pub use scylla::frame::value::CqlTimestamp;

use crate::error::ApiError;

pub fn ts_to_iso(ts: CqlTimestamp) -> String {
    DateTime::<Utc>::from_timestamp_millis(ts.0)
        .unwrap_or_default()
        .to_rfc3339()
}

pub fn now_ts() -> CqlTimestamp {
    CqlTimestamp(Utc::now().timestamp_millis())
}

pub fn parse_uuid(s: &str, label: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(s).map_err(|_| ApiError::BadRequest(format!("invalid {label} UUID: {s}")))
}
