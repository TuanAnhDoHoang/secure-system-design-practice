use getset::{Getters, Setters};
use surrealdb::types::{RecordId, SurrealValue};

pub mod connect;
pub mod schema;

#[derive(Getters, Setters, Debug, SurrealValue, Clone)]
#[getset(get = "pub", set = "pub")]

pub struct Record {
    id: RecordId,
}
