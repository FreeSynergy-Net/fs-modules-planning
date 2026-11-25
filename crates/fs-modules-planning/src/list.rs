//! The lanes as records of the form `rows@1.0.0` (type `Lane`, scope `lanes:<axis>[:<board>]`, see `lanes.rs`).
//!
//! Pattern: Projection (the lanes of the same tasks the cards show, as rows); the scope decides the type, as the form `rows@1.0.0`
//! leaves its meaning to the module.

use crate::formsource::FormSourceImpl;
use crate::lanes::LANES;
use crate::write::{UNKNOWN_SCOPE, problem};

/// One record: its id (`uid`) first, then the values by the field names of the catalog.
pub(crate) type Row = Vec<(&'static str, String)>;

impl FormSourceImpl {
    /// One page of `rows@1.0.0` in `scope`: the lanes of `lanes:...`. Returns the total and the rows from `skip`, at most `limit`.
    pub(crate) async fn list_page(
        &self,
        scope: &str,
        skip: usize,
        limit: usize,
    ) -> capnp::Result<(usize, Vec<Row>)> {
        let Some(lanes) = scope.strip_prefix(LANES) else {
            let why = format!("rows@1.0.0 lists the lanes: the scope is `{LANES}<axis>[:<board>]`");
            return Err(problem(UNKNOWN_SCOPE, why, 404).into_error());
        };
        let lanes = self.lanes(lanes).await?;
        let rows = lanes.iter().enumerate().skip(skip).take(limit);
        Ok((lanes.len(), rows.map(|(at, lane)| lane.row(at)).collect()))
    }
}
