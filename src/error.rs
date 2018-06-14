use std::error::Error;
use std::fmt;
use http;
use serde_json;

pub enum HError {
    CanceledSend,
    InvalidHttpRequest(String),
    InvalidHttpResponse(String),
}

impl Error for HError {
    fn description(&self) -> &str {
        match *self {
            HError::CanceledSend => "canceled send in channel",
            HError::InvalidHttpRequest(ref m) => m,
            HError::InvalidHttpResponse(ref m) => m,
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            HError::CanceledSend => None,
            HError::InvalidHttpRequest(_) => None,
            HError::InvalidHttpResponse(_) => None,
        }
    }
}

impl fmt::Display for HError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _ => write!(f, "{}", self.description()),
        }
    }
}

impl fmt::Debug for HError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _ => write!(f, "{}", self),
        }
    }
}

impl From<serde_json::Error> for HError {
    fn from(e: serde_json::Error) -> Self {
        HError::InvalidHttpResponse(e.description().to_owned())
    }
}

impl From<http::Error> for HError {
    fn from(e: http::Error) -> Self {
        HError::InvalidHttpResponse(e.description().to_owned())
    }
}