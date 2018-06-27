use super::StatusCode;
use error::HError;
use mime::{self, Mime};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use std::ops::Deref;

pub trait RequestBody {
    const MIME: Mime;

    fn to_bytes(self) -> Result<Vec<u8>, HError>;
}

pub trait ResponseBody: Sized {
    fn accept_types() -> &'static str;

    fn from_bytes(status: StatusCode, body: Vec<u8>) -> Result<Self, HError>;
}

impl RequestBody for () {
    const MIME: Mime = mime::TEXT_PLAIN;

    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        Ok(vec![])
    }
}

impl ResponseBody for () {
    fn accept_types() -> &'static str {
        "*/*"
    }

    fn from_bytes(_: StatusCode, _: Vec<u8>) -> Result<Self, HError> {
        Ok(())
    }
}

pub struct Json<V>(pub V);

impl<V> Json<V> {
    pub fn into_inner(self) -> V {
        self.0
    }

    pub fn inner(&self) -> &V {
        &self.0
    }
}

pub fn decode_json<T: DeserializeOwned>(slice: &[u8]) -> Result<T, HError> {
    match serde_json::from_slice(slice) {
        Ok(v) => Ok(v),
        Err(e) => Err(HError::InvalidDataFormat(format!(
            "invalid data format: {}",
            e
        ))),
    }
}

impl<'a, V> RequestBody for Json<V>
where
    V: Serialize,
{
    const MIME: Mime = mime::APPLICATION_JSON;

    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        match serde_json::to_vec(&self.0) {
            Ok(v) => Ok(v),
            Err(e) => Err(HError::InvalidDataFormat(format!("{}", e))),
        }
    }
}

impl<V: DeserializeOwned> ResponseBody for Json<V> {
    fn accept_types() -> &'static str {
        "application/json"
    }

    fn from_bytes(_: StatusCode, body: Vec<u8>) -> Result<Self, HError> {
        Ok(Json(decode_json(&body)?))
    }
}

impl<V: Send> Deref for Json<V> {
    type Target = V;

    fn deref(&self) -> &V {
        &self.0
    }
}

pub struct TextPlain(String);

impl TextPlain {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl RequestBody for TextPlain {
    const MIME: Mime = mime::TEXT_PLAIN;

    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        Ok(self.0.into_bytes())
    }
}

impl ResponseBody for TextPlain {
    fn accept_types() -> &'static str {
        "text/plain"
    }

    fn from_bytes(_: StatusCode, body: Vec<u8>) -> Result<Self, HError> {
        match String::from_utf8(body) {
            Ok(s) => Ok(TextPlain(s)),
            Err(e) => return Err(HError::InvalidDataFormat(format!("{}", e))),
        }
    }
}
