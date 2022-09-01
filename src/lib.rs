#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod impls;

#[cfg(feature = "reactor")]
mod reactor;

use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

#[cfg(feature = "reactor")]
pub use reactor::Reactor;

pub use serde_json::{json, Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    UnexpectedIdVariant,
    UnexpectedParamsVariant,
    UnexpectedRequestVariant,
    InvalidNumberCast,
    JsonRpcVersionNotFound,
    InvalidJsonRpcVersion,
    ExpectedId,
    ExpectedMethod,
    InvalidMethodVariant,
    UnexpectedNotificationVariant,
    UnexpectedErrorVariant,
    ExpectedErrorCode,
    ExpectedErrorCodeAsInteger,
    ExpectedErrorMessage,
    ExpectedErrorCodeAsString,
    UnexpectedResponseVariant,
    ResponseExpectsResultOrError,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Id {
    String(String),
    Number(i64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Params {
    Array(Vec<Value>),
    Object(Map<String, Value>),
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: Id,
    pub method: String,
    pub params: Params,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub method: String,
    pub params: Params,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub id: Id,
    pub result: Result<Value, RpcError>,
}
