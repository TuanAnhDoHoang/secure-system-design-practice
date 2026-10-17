use getset::{Getters, Setters};
use surrealdb::types::{RecordId, SurrealValue};

#[derive(Getters, Setters, Debug, SurrealValue, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct Record {
    id: RecordId,
}
