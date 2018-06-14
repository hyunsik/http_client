//! Asynchronous Http Client for JSON body

extern crate futures;
extern crate http;
extern crate hyper;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio;

use std::convert::From;
use std::error::Error;
use std::fmt;

use futures::{Future, Stream};
use http::{Method, Request, Response};
use hyper::client::HttpConnector;
use hyper::header::CONTENT_TYPE;
use hyper::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub use http::StatusCode;
pub use http::Uri;

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

pub type HResult<T> = Result<T, HError>;
pub type RFuture<T> = futures::Future<Item = T, Error = HError> + Send;

pub static EMPTY: () = ();

pub const DEFAULT_THREAD_NUM: usize = 2;

pub struct HttpClient {
    client: Client<HttpConnector>,
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

impl HttpClient {
    pub fn new() -> HttpClient {
        HttpClient {
            client: Client::new(),
        }
    }

    pub fn get<R>(&self, uri: &Uri) -> impl Future<Item = Status<R>, Error = HError> + 'static
    where
        R: DeserializeOwned + 'static + Send
    {
        let req = self.build_request(Method::GET, uri, &EMPTY)
            .ok()
            .expect("HttpClient::build_request failed..");
        self.handle_response(req)
    }

    pub fn request<P, R>(
        &self,
        method: Method,
        uri: &Uri,
        value: &P,
    ) -> HResult<impl Future<Item = Status<R>, Error = HError>>
    where
        P: Serialize,
        R: DeserializeOwned + 'static + Send
    {
        let req = self.build_request(method, uri, value)?;
        Ok(self.handle_response(req))
    }

    pub fn request_raw<P, R>(
        &self,
        method: Method,
        uri: &Uri,
        value: P,
    ) -> HResult<impl Future<Item = Status<R>, Error = HError>>
    where
        P: AsRef<[u8]>,
        R: DeserializeOwned + 'static + Send
    {
        let req = self.build_request_raw(method, uri, value)?;
        Ok(self.handle_response(req))
    }

    pub fn build_request<P>(
        &self,
        method: Method,
        uri: &Uri,
        value: &P,
    ) -> HResult<Request<hyper::Body>>
    where
        P: Serialize,
    {
        let body = serde_json::to_vec(value)?;
        let mut req = Request::builder();
        match req.uri(uri.clone())
            .method(method)
            .header(CONTENT_TYPE, "application/json")
            .body(body.into())
        {
            Ok(req) => Ok(req),
            Err(e) => Err(HError::InvalidHttpRequest(format!("{}", e))),
        }
    }

    pub fn build_request_raw<P>(
        &self,
        method: Method,
        uri: &Uri,
        value: P,
    ) -> HResult<Request<hyper::Body>>
    where
        P: AsRef<[u8]>,
    {
        let body = value.as_ref();
        let mut req = Request::builder();
        match req.uri(uri.clone())
            .method(method)
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_owned().into())
        {
            Ok(req) => Ok(req),
            Err(e) => Err(HError::InvalidHttpRequest(format!("{}", e))),
        }
    }

    fn handle_response<R>(
        &self,
        req: Request<hyper::Body>,
    ) -> impl Future<Item = Status<R>, Error = HError> + 'static
    where
        R: Sized + DeserializeOwned + 'static + Send
    {
        self.client
            .request(req)
            .then(|result: Result<Response<hyper::Body>, hyper::Error>|
                    -> HResult<Box<RFuture<Status<R>>>> {
                match result {
                    Ok(r) => {
                        let future: Box<RFuture<Status<R>>> = match r.status() {
                            StatusCode::OK => Box::new(
                                r.into_body()
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
    }
}

impl<T: DeserializeOwned + 'static> From<StatusCode> for Status<T> {
    fn from(s: StatusCode) -> Self {
        match s {
            StatusCode::CREATED => Status::Created,
            StatusCode::ACCEPTED => Status::Accepted,
            StatusCode::NOT_FOUND => Status::NotFound,
            StatusCode::INTERNAL_SERVER_ERROR => Status::InternalServerError,
            StatusCode::NOT_IMPLEMENTED => Status::NotImplemented,
            _ => Status::Unregistered(s.as_u16()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum RunnerStatus {
        New,
        InitFailed(String),
        Idle,
        Running,
        Unhealthy(String),
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct RunnerState {
        start_time: u64,
        status: RunnerStatus,
        succeeded_tasks: u64,
        failed_tasks: u64,
    }

    #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
    pub struct TaskRunReq {
        pub id: String,
        pub sandbox: String,
        pub input_fname: String,
        pub run_script: String,
        pub run_expr: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ErrorResponse {
        code: String,
        message: String,
    }

    #[test]
    fn test() {
        use std::thread;
        let uri: Uri = "http://127.0.0.1:58921/tasks".parse().unwrap();
        let client = HttpClient::new();
        let (tx, rx) = oneshot::channel();
        use futures::sync::oneshot;

        use serde_json;
        let req = TaskRunReq {
            id: "task_123".to_string(),
            sandbox: "/tmp/rdist-test".to_string(),
            input_fname: "test.csv".to_string(),
            run_script: "run.R".to_string(),
            run_expr: "run()".to_string(),
        };

        thread::spawn(move || {
            let task = client.request(Method::POST, &uri, &req).unwrap()
                .then(move |r: Result<Status<()>, HError>| -> Result<(), ()> {
//                    match r {
//                        Ok(Status::Ok(r)) => {
//                            //let r: RunnerState = r;
//                            //tx.send(r).unwrap();
//                        }
//                        Ok(_) => panic!("{}"),
//                        Err(e) => eprintln!("{:?}", e),
//                    };
                    tx.send(());
                    Ok(())
                });
            tokio::run(task);
        });

        let state = rx.wait().unwrap();
        eprintln!("{:?}", state);
    }
}
