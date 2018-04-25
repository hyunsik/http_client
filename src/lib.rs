//! Asynchronous Http Client for JSON body

extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate tokio_core;

use std::convert::From;
use std::error::Error;
use std::fmt;

use futures::sync::oneshot;
use futures::{Async, Future, Poll, Stream};
use hyper::client::HttpConnector;
use hyper::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_core::reactor::Handle;

pub use hyper::Uri;
pub use hyper::client::Request;
pub use hyper::header::{ContentLength, ContentType, Header};
pub use hyper::{Method, StatusCode};

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

pub type HResult<T> = Result<T, HError>;
pub type RFuture<T> = futures::Future<Item = T, Error = HError>;

pub static EMPTY: () = ();

pub const DEFAULT_THREAD_NUM: usize = 2;

pub struct HttpClient {
    client: Client<HttpConnector>,
    handle: Handle,
}

#[derive(Debug, PartialEq)]
pub enum Status<T: DeserializeOwned + 'static> {
    /// 200 OK
    Ok(T),
    /// 201 Created
    Created,
    /// 202 Accepted
    Accepted,

    /// 404 Not Found
    NotFound,

    /// 500 Internal Server Error
    InternalServerError,
    /// 501 Not Implemented
    NotImplemented,

    Unregistered(u16),
}

pub struct FutureResponse<T: DeserializeOwned + 'static>(
    oneshot::Receiver<Result<Status<T>, HError>>,
);

impl<T: DeserializeOwned + 'static> Future for FutureResponse<T> {
    type Item = Status<T>;
    type Error = HError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.poll() {
            Ok(Async::Ready(Ok(t))) => Ok(t.into()),
            Ok(Async::Ready(Err(e))) => Err(e),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(_) => Err(HError::CanceledSend),
        }
    }
}

impl HttpClient {
    pub fn new(handle: &Handle) -> HttpClient {
        let client = Client::configure()
            .connector(HttpConnector::new(DEFAULT_THREAD_NUM, handle))
            .keep_alive(true)
            .build(handle);

        HttpClient {
            client: client,
            handle: handle.clone(),
        }
    }

    pub fn get<R>(&mut self, uri: &hyper::Uri) -> FutureResponse<R>
        where
            R: DeserializeOwned + 'static,
    {
        let req = self.build_request(Method::Get, uri, &EMPTY)
            .ok()
            .expect("HttpClient::build_request failed..");
        self.handle_response(req)
    }

    pub fn request<P, R>(
        &mut self,
        method: Method,
        uri: &hyper::Uri,
        value: &P,
    ) -> HResult<FutureResponse<R>>
        where
            P: Serialize,
            R: DeserializeOwned + 'static,
    {
        let req = self.build_request(method, uri, value)?;
        Ok(self.handle_response(req))
    }

    pub fn build_request<P>(
        &mut self,
        method: Method,
        uri: &hyper::Uri,
        value: &P,
    ) -> HResult<Request>
        where
            P: Serialize,
    {
        let json_str = serde_json::to_string(value)?;
        let mut req = Request::new(method, uri.clone());
        req.headers_mut().set(ContentType::json());
        req.headers_mut().set(ContentLength(json_str.len() as u64));
        req.set_body(json_str);
        Ok(req)
    }

    fn handle_response<R>(&mut self, req: Request) -> FutureResponse<R>
        where
            R: DeserializeOwned + 'static,
    {
        let (tx, rx) = oneshot::channel::<HResult<Status<R>>>();
        let task = self.client
            .request(req)
            .then(|result| -> HResult<Box<RFuture<Status<R>>>> {
                match result {
                    Ok(r) => {
                        let future: Box<RFuture<Status<R>>> = match r.status() {
                            StatusCode::Ok => Box::new(
                                r.body()
                                    .map_err(|e| {
                                        HError::InvalidHttpResponse(e.description().to_owned())
                                    })
                                    .fold(Vec::new(), |mut acc, chunk| {
                                        acc.extend_from_slice(&*chunk);
                                        futures::future::ok::<_, HError>(acc)
                                    })
                                    .and_then(|chunk| match serde_json::from_slice(&chunk) {
                                        Ok(json) => Ok(Status::Ok(json)),
                                        Err(e) => Err(HError::InvalidHttpResponse(
                                            e.description().to_owned(),
                                        )),
                                    }),
                            ),
                            _ => Box::new(futures::future::ok(r.status().into())),
                        };

                        Ok(future)
                    }
                    Err(e) => Err(HError::InvalidHttpResponse(e.description().to_owned())),
                }
            })
            .and_then(|c| c)
            .then(|res| -> Result<(), ()> {
                let _ = tx.send(res);
                Ok(())
            });

        let _ = self.handle.spawn(task);
        FutureResponse(rx)
    }
}

impl<T: DeserializeOwned + 'static> From<StatusCode> for Status<T> {
    fn from(s: StatusCode) -> Self {
        match s {
            StatusCode::Created => Status::Created,
            StatusCode::Accepted => Status::Accepted,
            StatusCode::NotFound => Status::NotFound,
            StatusCode::InternalServerError => Status::InternalServerError,
            StatusCode::NotImplemented => Status::NotImplemented,
            _ => Status::Unregistered(s.as_u16()),
        }
    }
}