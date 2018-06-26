use super::StatusCode;
use error::HError;
use mime;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use std::ops::Deref;

pub trait MimeType {
    fn mime_type() -> &'static str;
}

pub trait MimeSerialize: MimeType {
    fn to_bytes(self) -> Result<Vec<u8>, HError>;
}

pub trait MimeDeserialize: MimeType + Sized {
    fn from_bytes(status: StatusCode, body: Vec<u8>) -> Result<Self, HError>;
}

impl MimeType for () {
    fn mime_type() -> &'static str {
        mime::TEXT_PLAIN.as_ref()
    }
}

impl MimeSerialize for () {
    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        Ok(vec![])
    }
}

impl MimeDeserialize for () {
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

impl<'a, V> MimeType for Json<V> {
    fn mime_type() -> &'static str {
        "application/json"
    }
}

impl<'a, V> MimeSerialize for Json<V>
where
    V: Serialize,
{
    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        match serde_json::to_vec(&self.0) {
            Ok(v) => Ok(v),
            Err(e) => Err(HError::InvalidDataFormat(format!("{}", e))),
        }
    }
}

impl<V: DeserializeOwned> MimeDeserialize for Json<V> {
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

impl MimeType for TextPlain {
    fn mime_type() -> &'static str {
        "text/plain"
    }
}

impl MimeSerialize for TextPlain {
    fn to_bytes(self) -> Result<Vec<u8>, HError> {
        Ok(self.0.into_bytes())
    }
}

impl MimeDeserialize for TextPlain {
    fn from_bytes(_: StatusCode, body: Vec<u8>) -> Result<Self, HError> {
        match String::from_utf8(body) {
            Ok(s) => Ok(TextPlain(s)),
            Err(e) => return Err(HError::InvalidDataFormat(format!("{}", e))),
        }
    }
}
