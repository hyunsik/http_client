//! Asynchronous Http Client for JSON body

extern crate futures;
extern crate http;
extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate tokio;

pub use error::*;
use futures::{Future, Stream};
use http::Request;
use hyper::client::HttpConnector;
use hyper::header::CONTENT_TYPE;
use hyper::Client;
use std::convert::From;
use std::error::Error;

mod error;
pub mod mime;

use mime::MimeType;
pub use http::{Method, StatusCode};
pub use http::Uri;

pub const DEFAULT_THREAD_NUM: usize = 2;

pub struct HttpClient {
    client: Client<HttpConnector>,
}

pub struct Response<T>
where
    T: MimeType,
{
    pub status: http::StatusCode,
    pub value: T,
}

impl<T> Response<T>
where
    T: MimeType,
{
    pub fn new(status: StatusCode, value: T) -> Self {
        Response { status, value }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl HttpClient {
    pub fn new() -> HttpClient {
        HttpClient {
            client: Client::new(),
        }
    }

    pub fn get<R: MimeType>(
        &self,
        uri: Uri,
    ) -> impl Future<Item = Response<R>, Error = HError> + 'static
    where
        R: MimeType + 'static,
    {
        let mut req = Request::new(hyper::Body::default());
        *req.uri_mut() = uri;
        self.handle_response(req)
    }

    pub fn post<S, R>(
        &self,
        uri: Uri,
        value: S,
    ) -> Result<impl Future<Item = Response<R>, Error = HError> + 'static, HError>
    where
        S: MimeType + 'static,
        R: MimeType + 'static,
    {
        self.request(Method::POST, uri, value)
    }

    pub fn put<S, R>(
        &self,
        uri: Uri,
        value: S,
    ) -> Result<impl Future<Item = Response<R>, Error = HError> + 'static, HError>
    where
        S: MimeType + 'static,
        R: MimeType + 'static,
    {
        self.request(Method::PUT, uri, value)
    }

    pub fn delete<S, R>(
        &self,
        uri: Uri,
        value: S,
    ) -> Result<impl Future<Item = Response<R>, Error = HError> + 'static, HError>
    where
        S: MimeType + 'static,
        R: MimeType + 'static,
    {
        self.request(Method::DELETE, uri, value)
    }

    pub fn request<S, R>(
        &self,
        method: Method,
        uri: Uri,
        value: S,
    ) -> Result<impl Future<Item = Response<R>, Error = HError> + 'static, HError>
    where
        S: MimeType + 'static,
        R: MimeType + 'static,
    {
        let mut builder = Request::builder();
        let req = match builder
            .uri(uri.clone())
            .method(method)
            .header(CONTENT_TYPE, S::mime_type())
            .body(hyper::Body::from(value.to_bytes()?))
        {
            Ok(req) => req,
            Err(e) => return Err(HError::InvalidHttpRequest(format!("{}", e))),
        };
        Ok(self.handle_response(req))
    }

    fn handle_response<R>(
        &self,
        req: Request<hyper::Body>,
    ) -> impl Future<Item = Response<R>, Error = HError> + 'static
    where
        R: MimeType + 'static,
    {
        self.client
            .request(req)
            .then(
                |result: Result<http::Response<hyper::Body>, hyper::Error>| -> Result<
                    Box<Future<Item = Response<R>, Error = HError> + 'static + Send>,
                    HError,
                > {
                    match result {
                        Ok(r) => {
                            let status_code = r.status();
                            let future = Box::new(
                                r.into_body()
                                    .map_err(|e| {
                                        HError::InvalidHttpResponse(e.description().to_owned())
                                    })
                                    .fold(Vec::new(), |mut acc, chunk| {
                                        acc.extend_from_slice(&*chunk);
                                        futures::future::ok::<_, HError>(acc)
                                    })
                                    .and_then(move |body| {
                                        R::from_bytes(status_code, body)
                                            .map(|payload| Response::new(status_code, payload))
                                    }),
                            );
                            Ok(future)
                        }
                        Err(e) => Err(HError::InvalidHttpResponse(format!("{}", e))),
                    }
                },
            )
            .and_then(|res| res)
    }
}
