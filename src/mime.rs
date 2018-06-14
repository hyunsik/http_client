use std::ops::Deref;
use super::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use error::HError;

pub struct Json<V: Send>(pub V);

impl<V: Send> Json<V> {
    pub fn into_inner(self) -> V {
        self.0
    }
}

impl<'a, V: Send> MimeType for Json<V>
    where
        V: Serialize + DeserializeOwned,
{
    fn mime_type() -> &'static str {
        "application/json"
    }

    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        match serde_json::to_vec(&self.0) {
            Ok(v) => Ok(v),
            Err(e) => Err(HError::InvalidDataFormat(format!("{}", e)))
        }
    }

    fn from_bytes(_: StatusCode, body: Vec<u8>) -> Result<Self, HError> {
        match serde_json::from_slice(&body) {
            Ok(v) => Ok(Json(v)),
            Err(e) => Err(HError::InvalidDataFormat(format!("{}", e))),
        }
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

impl MimeType for TextPlain {
    fn mime_type() -> &'static str {
        "text/plain"
    }

    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        Ok(self.0.into_bytes())
    }

    fn from_bytes(_: StatusCode, body: Vec<u8>) -> Result<Self, HError> {
        match String::from_utf8(body) {
            Ok(s) => Ok(TextPlain(s)),
            Err(e) => return Err(HError::InvalidDataFormat(format!("{}", e))),
        }
    }
}

impl MimeType for () {
    fn mime_type() -> &'static str {
        "text/plain"
    }

    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        Ok(vec![])
    }

    fn from_bytes(_: StatusCode, _: Vec<u8>) -> Result<Self, HError> {
        Ok(())
    }
}

pub trait MimeType: Sized + Send {
    fn mime_type() -> &'static str;

    fn to_bytes(self) -> Result<Vec<u8>, HError>;

    fn from_bytes(status: StatusCode, body: Vec<u8>) -> Result<Self, HError>;
}