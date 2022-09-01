use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::{fmt, iter};

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Error as fmt::Debug>::fmt(self, f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl From<Id> for Value {
    fn from(id: Id) -> Self {
        match id {
            Id::String(s) => Value::String(s),
            Id::Number(n) => Value::Number(n.into()),
        }
    }
}

impl From<&Id> for Value {
    fn from(id: &Id) -> Self {
        match id {
            Id::String(s) => Value::String(s.clone()),
            Id::Number(n) => Value::Number((*n).into()),
        }
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<i64> for Id {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl TryFrom<Value> for Id {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) => Ok(n.as_i64().ok_or(Error::InvalidNumberCast)?.into()),
            Value::String(s) => Ok(s.into()),
            Value::Null | Value::Array(_) | Value::Object(_) | Value::Bool(_) => {
                Err(Error::UnexpectedIdVariant)
            }
        }
    }
}

impl TryFrom<&Value> for Id {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) => Ok(n.as_i64().ok_or(Error::InvalidNumberCast)?.into()),
            Value::String(s) => Ok(s.clone().into()),
            Value::Null | Value::Array(_) | Value::Object(_) | Value::Bool(_) => {
                Err(Error::UnexpectedIdVariant)
            }
        }
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Value::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Value::deserialize(deserializer).and_then(|v| Self::try_from(v).map_err(D::Error::custom))
    }
}

impl Params {
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Params::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Map<String, Value>> {
        match self {
            Params::Object(m) => Some(m),
            _ => None,
        }
    }
}

impl From<Params> for Value {
    fn from(params: Params) -> Self {
        match params {
            Params::Array(a) => Value::Array(a),
            Params::Object(m) => Value::Object(m),
            Params::Null => Value::Null,
        }
    }
}

impl From<&Params> for Value {
    fn from(params: &Params) -> Self {
        match params {
            Params::Array(a) => Value::Array(a.clone()),
            Params::Object(m) => Value::Object(m.clone()),
            Params::Null => Value::Null,
        }
    }
}

impl From<Vec<Value>> for Params {
    fn from(a: Vec<Value>) -> Self {
        Self::Array(a)
    }
}

impl From<Map<String, Value>> for Params {
    fn from(m: Map<String, Value>) -> Self {
        Self::Object(m)
    }
}

impl FromIterator<Value> for Params {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        iter.into_iter().collect::<Vec<_>>().into()
    }
}

impl FromIterator<(String, Value)> for Params {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        iter.into_iter().collect::<Map<_, _>>().into()
    }
}

impl TryFrom<Value> for Params {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::Array(a) => Ok(a.into()),
            Value::Object(o) => Ok(o.into()),

            Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                Err(Error::UnexpectedParamsVariant)
            }
        }
    }
}

impl TryFrom<&Value> for Params {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::Array(a) => Ok(a.clone().into()),
            Value::Object(o) => Ok(o.clone().into()),

            Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                Err(Error::UnexpectedParamsVariant)
            }
        }
    }
}

impl Serialize for Params {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Value::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Params {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Value::deserialize(deserializer).and_then(|v| Self::try_from(v).map_err(D::Error::custom))
    }
}

impl From<Request> for Value {
    fn from(req: Request) -> Self {
        let Request { id, method, params } = req;
        let params = Value::from(params);

        let map = iter::once(Some((
            "jsonrpc".to_string(),
            Value::String("2.0".to_string()),
        )))
        .chain(iter::once(Some(("method".to_string(), method.into()))))
        .chain(iter::once(
            (!params.is_null()).then(|| ("params".to_string(), params)),
        ))
        .chain(iter::once(Some(("id".to_string(), id.into()))))
        .flatten()
        .collect();

        Value::Object(map)
    }
}

impl TryFrom<&Value> for Request {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_object().ok_or(Error::UnexpectedRequestVariant)?;

        map.get("jsonrpc")
            .ok_or(Error::JsonRpcVersionNotFound)?
            .as_str()
            .filter(|version| version == &"2.0")
            .ok_or(Error::InvalidJsonRpcVersion)?;

        let id = map.get("id").ok_or(Error::ExpectedId)?.try_into()?;

        let method = map
            .get("method")
            .ok_or(Error::ExpectedMethod)?
            .as_str()
            .ok_or(Error::InvalidMethodVariant)?
            .to_string();

        let params = map.get("params").unwrap_or(&Value::Null).try_into()?;

        Ok(Self { id, method, params })
    }
}

impl From<Notification> for Value {
    fn from(notification: Notification) -> Self {
        let Notification { method, params } = notification;

        let method = Value::String(method);
        let params = Value::from(params);

        json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        })
    }
}

impl TryFrom<&Value> for Notification {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_object().ok_or(Error::UnexpectedRequestVariant)?;

        map.get("jsonrpc")
            .ok_or(Error::JsonRpcVersionNotFound)?
            .as_str()
            .filter(|version| version == &"2.0")
            .ok_or(Error::InvalidJsonRpcVersion)?;

        let method = map
            .get("method")
            .ok_or(Error::ExpectedMethod)?
            .as_str()
            .ok_or(Error::InvalidMethodVariant)?
            .to_string();

        let params = map.get("params").unwrap_or(&Value::Null).try_into()?;

        Ok(Self { method, params })
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RpcError {}

impl From<RpcError> for Value {
    fn from(re: RpcError) -> Self {
        json!({
            "code": re.code,
            "message": re.message,
            "data": re.data,
        })
    }
}

impl From<&RpcError> for Value {
    fn from(re: &RpcError) -> Self {
        json!({
            "code": re.code,
            "message": re.message,
            "data": re.data,
        })
    }
}

impl TryFrom<Value> for RpcError {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let map = value.as_object().ok_or(Error::UnexpectedErrorVariant)?;

        let code = map
            .get("code")
            .ok_or(Error::ExpectedErrorCode)?
            .as_i64()
            .ok_or(Error::ExpectedErrorCodeAsInteger)?;

        let message = map
            .get("message")
            .ok_or(Error::ExpectedErrorMessage)?
            .as_str()
            .ok_or(Error::ExpectedErrorCodeAsString)?
            .to_string();

        let data = map.get("data").unwrap_or(&Value::Null).clone();

        Ok(Self {
            code,
            message,
            data,
        })
    }
}

impl TryFrom<&Value> for RpcError {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_object().ok_or(Error::UnexpectedErrorVariant)?;

        let code = map
            .get("code")
            .ok_or(Error::ExpectedErrorCode)?
            .as_i64()
            .ok_or(Error::ExpectedErrorCodeAsInteger)?;

        let message = map
            .get("message")
            .ok_or(Error::ExpectedErrorMessage)?
            .as_str()
            .ok_or(Error::ExpectedErrorCodeAsString)?
            .to_string();

        let data = map.get("data").unwrap_or(&Value::Null).clone();

        Ok(Self {
            code,
            message,
            data,
        })
    }
}

impl Serialize for RpcError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Value::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RpcError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Value::deserialize(deserializer).and_then(|v| Self::try_from(v).map_err(D::Error::custom))
    }
}

impl From<Response> for Value {
    fn from(response: Response) -> Self {
        let Response { id, result } = response;

        let result = result
            .map(|v| Some(("result".to_string(), v)))
            .unwrap_or_else(|e| Some(("error".to_string(), Value::from(e))));

        let map = iter::once(Some((
            "jsonrpc".to_string(),
            Value::String("2.0".to_string()),
        )))
        .chain(iter::once(Some(("id".to_string(), id.into()))))
        .chain(iter::once(result))
        .flatten()
        .collect();

        Value::Object(map)
    }
}

impl From<&Response> for Value {
    fn from(response: &Response) -> Self {
        let Response { id, result } = response;

        let result = result
            .as_ref()
            .map(|v| Some(("result".to_string(), v.clone())))
            .unwrap_or_else(|e| Some(("error".to_string(), Value::from(e.clone()))));

        let map = iter::once(Some((
            "jsonrpc".to_string(),
            Value::String("2.0".to_string()),
        )))
        .chain(iter::once(Some(("id".to_string(), id.into()))))
        .chain(iter::once(result))
        .flatten()
        .collect();

        Value::Object(map)
    }
}

impl TryFrom<Value> for Response {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let map = value.as_object().ok_or(Error::UnexpectedResponseVariant)?;

        map.get("jsonrpc")
            .ok_or(Error::JsonRpcVersionNotFound)?
            .as_str()
            .filter(|version| version == &"2.0")
            .ok_or(Error::InvalidJsonRpcVersion)?;

        let result = map.get("result").cloned();
        let error = map.get("error").map(RpcError::try_from).transpose()?;

        let result = match (result, error) {
            (None, Some(e)) => Err(e),
            (Some(v), None) => Ok(v),
            (Some(_), Some(_)) | (None, None) => return Err(Error::ResponseExpectsResultOrError),
        };

        let id = map.get("id").ok_or(Error::ExpectedId)?.try_into()?;

        Ok(Self { id, result })
    }
}

impl TryFrom<&Value> for Response {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_object().ok_or(Error::UnexpectedResponseVariant)?;

        map.get("jsonrpc")
            .ok_or(Error::JsonRpcVersionNotFound)?
            .as_str()
            .filter(|version| version == &"2.0")
            .ok_or(Error::InvalidJsonRpcVersion)?;

        let result = map.get("result").cloned();
        let error = map.get("error").map(RpcError::try_from).transpose()?;

        let result = match (result, error) {
            (None, Some(e)) => Err(e),
            (Some(v), None) => Ok(v),
            (Some(_), Some(_)) | (None, None) => return Err(Error::ResponseExpectsResultOrError),
        };

        let id = map.get("id").ok_or(Error::ExpectedId)?.try_into()?;

        Ok(Self { id, result })
    }
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Value::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Value::deserialize(deserializer).and_then(|v| Self::try_from(v).map_err(D::Error::custom))
    }
}
