use futures;
use std::error::Error;
use std::fmt;

pub type HResult<T> = Result<T, HError>;
pub type RFuture<T> = futures::Future<Item = T, Error = HError> + Send;

pub enum HError {
    CanceledSend,
    InvalidHttpRequest(String),
    InvalidHttpResponse(String),
    InvalidDataFormat(String),
}

impl Error for HError {
    fn description(&self) -> &str {
        match *self {
            HError::CanceledSend => "canceled send in channel",
            HError::InvalidHttpRequest(ref m) => m,
            HError::InvalidHttpResponse(ref m) => m,
            HError::InvalidDataFormat(ref m) => m,
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            _ => None,
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
