use self::context::{OkRespWithContext, RespContext};
use self::error::ClientErr;
use self::serialization_formats::{ApiFormat, JsonFormat, SerialFormat, XmlFormat};
use reqwest::{RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use std::future::Future;
use std::time::Duration;

pub mod re_exports {
    pub use reqwest;
}

pub mod prelude {
    pub use crate::error::aliases::{
        ApiResult, JsonApiErr, JsonClientResult, XmlApiErr, XmlApiResult,
    };
    pub use crate::error::{ClientErr, ResultExt};
    pub use crate::serialization_formats::{ApiFormat, JsonFormat, SerialFormat};
    pub use crate::{ApiClient, ExpectResp, JsonApiClient, ReceiveJson};
}

// Goals
// - directly implement recv_json() / recv_xml() (one trait for each, dedup common logic)
// one ApiClient trait for xml, one for json, dedup common logic

// - can just await the ok version (no special syntax)
// - it deserializes to ok type
// - can specify expected status
// - don't have to specify expected status
// - if specify expected status, specify in callee not caller

// - can expect error
// - can specify expected status
// - don’t have to specify expected status
// - caller can specify expected status because it depends on the error
// - can get status and details if not expected status / error / type

pub trait ApiClient<Format: ApiFormat> {
    fn base_url(&self) -> &str;
    fn http_client(&self) -> &reqwest::Client;

    fn path(&self, url_path: &str) -> String {
        let origin = self.base_url().trim().trim_end_matches('/');
        let path = url_path.trim().trim_start_matches('/');
        format!("{origin}/{path}")
    }

    fn default_params(&self, request_builder: RequestBuilder) -> RequestBuilder {
        Format::with_accept_header(request_builder.timeout(Duration::new(5, 0)))
    }
    fn get(&self, url_path: &str) -> RequestBuilder {
        self.default_params(self.http_client().get(self.path(url_path)))
    }
    fn post(&self, url_path: &str) -> RequestBuilder {
        self.default_params(Format::with_content_type_header(
            self.http_client().post(self.path(url_path)),
        ))
    }
}

/// Convenience alias trait for ApiClient<JsonFormat> since JSON is most common
pub trait JsonApiClient {
    fn base_url(&self) -> &str;
    fn http_client(&self) -> &reqwest::Client;
}
impl<T: JsonApiClient> ApiClient<JsonFormat> for T {
    fn base_url(&self) -> &str {
        <Self as JsonApiClient>::base_url(self)
    }
    fn http_client(&self) -> &reqwest::Client {
        <Self as JsonApiClient>::http_client(self)
    }
}

pub mod serialization_formats {
    use reqwest::RequestBuilder;
    use serde::Deserialize;

    pub trait SerialFormat {
        type Error: std::fmt::Debug;
        fn from_str<T: for<'a> Deserialize<'a>>(input: &str) -> Result<T, Self::Error>;
    }
    #[derive(Debug)]
    pub struct JsonFormat;
    impl SerialFormat for JsonFormat {
        type Error = serde_json::Error;
        fn from_str<T: for<'a> Deserialize<'a>>(input: &str) -> Result<T, Self::Error> {
            serde_json::from_str(input)
        }
    }
    #[derive(Debug)]
    pub struct XmlFormat;
    impl SerialFormat for XmlFormat {
        type Error = serde_xml_rs::Error;
        fn from_str<T: for<'a> Deserialize<'a>>(input: &str) -> Result<T, Self::Error> {
            serde_xml_rs::de::from_str(input)
        }
    }

    pub trait ApiFormat: SerialFormat {
        fn with_accept_header(builder: RequestBuilder) -> RequestBuilder;
        fn with_content_type_header(builder: RequestBuilder) -> RequestBuilder;
    }
    impl ApiFormat for JsonFormat {
        fn with_accept_header(builder: RequestBuilder) -> RequestBuilder {
            builder.header("Accept", "application/json")
        }
        fn with_content_type_header(builder: RequestBuilder) -> RequestBuilder {
            builder.header("Content-Type", "application/json")
        }
    }
    impl ApiFormat for XmlFormat {
        fn with_accept_header(builder: RequestBuilder) -> RequestBuilder {
            builder.header("Accept", "application/xml")
        }
        fn with_content_type_header(builder: RequestBuilder) -> RequestBuilder {
            builder.header("Content-Type", "application/xml")
        }
    }
}

impl<T: Sized + Into<RequestBuilder>> ExpectResp<JsonFormat> for T {} // auto-implement for RequestBuilder and more
#[allow(async_fn_in_trait)]
pub trait ExpectResp<F: SerialFormat>: Sized + Into<RequestBuilder> {
    async fn expect_ok<Ok: DeserializeOwned, ErrResp: DeserializeOwned>(
        self,
    ) -> Result<Ok, ClientErr<ErrResp, F>> {
        self.partial_expect().await.map(|ok| ok.ok_body)
    }
    async fn expect_err_resp<Ok: DeserializeOwned, ErrResp: DeserializeOwned>(
        self,
        expect_status: StatusCode,
    ) -> Result<ErrResp, ClientErr<ErrResp, F>> {
        match self.partial_expect::<Ok, ErrResp>().await {
            Ok(ok) => Err(ClientErr::ExpectedErrorResponse {
                context: Some(ok.context),
            }),
            Err(err) => err.try_into_err_resp(expect_status),
        }
    }
    // async fn expect_status<Ok: DeserializeOwned, ErrResp: DeserializeOwned>(
    //     self,
    //     expect_status: StatusCode,
    // ) -> Result<Ok, RequestErr<ErrResp, F>> {
    //     match Self::partial_expect(self.into()).await {
    //         Ok(ok) => {
    //             // TODO check status
    //             todo!()
    //         }
    //         Err(err) => {
    //             // TODO check status
    //             todo!()
    //         }
    //     }
    //     // TODO
    // }
    fn partial_expect<Ok: DeserializeOwned, ErrResp: DeserializeOwned>(
        self,
    ) -> impl Future<Output = Result<OkRespWithContext<Ok>, ClientErr<ErrResp, F>>> {
        async move {
            let request_builder: RequestBuilder = self.into();
            let (client, if_ok_request) = request_builder.build_split();
            let request = if_ok_request.map_err(ClientErr::BuildRequest)?;
            let (method, url) = { (request.method().clone(), request.url().clone()) };

            let response = client
                .execute(request)
                .await
                .map_err(ClientErr::ExecuteRequest)?;
            let got_status = response.status();
            let context = RespContext {
                method,
                url,
                got_status: response.status(),
                response_text: response.text().await.map_err(ClientErr::ReadRespBodyText)?,
            };

            // if let Some(expected_status) = expect_status {
            //     if got_status.is_success() && !expected_status.is_success() {
            //         return Err(RequestErr::ExpectedErrorResponse { context });
            //     }
            //     // if !got_status.is_success() && expected_status.is_success() {
            //     //     return Err(RequestErr::ExpectedSuccessResponse { context });
            //     // }
            //     if got_status != expected_status {
            //         return Err(RequestErr::ExpectedStatus {
            //             context,
            //             expected_status,
            //         });
            //     }
            // }

            // if err, try to deserialize error body into ErrResp type
            if !got_status.is_success() {
                match F::from_str::<ErrResp>(&context.response_text) {
                    Ok(source) => {
                        return Err(ClientErr::ErrorResponse {
                            context,
                            err_body: source,
                        })
                    }
                    Err(deserialize_error) => {
                        return Err(ClientErr::DeserializeError {
                            context,
                            deserialize_error,
                        })
                    }
                }
            }

            // try to deserialize ok response
            match F::from_str(&context.response_text) {
                Ok(v) => Ok(OkRespWithContext {
                    ok_body: v,
                    context,
                }),
                Err(deserialize_error) => Err(ClientErr::DeserializeError {
                    context,
                    deserialize_error,
                }),
            }
        }
    }
}
pub trait ReceiveJson {
    fn recv_json<Ok: DeserializeOwned, ErrResp: DeserializeOwned>(
        self,
    ) -> impl Future<Output = Result<Ok, ClientErr<ErrResp, JsonFormat>>>;
}
// auto-impl ReceiveJson for all ExpectResp
impl<T: ExpectResp<JsonFormat>> ReceiveJson for T {
    fn recv_json<Ok: DeserializeOwned, ErrResp: DeserializeOwned>(
        self,
    ) -> impl Future<Output = Result<Ok, ClientErr<ErrResp, JsonFormat>>> {
        self.expect_ok()
    }
}

pub mod context {
    use super::prelude::*;
    use reqwest::{Method, StatusCode, Url};
    use serde::de::DeserializeOwned;

    #[derive(Debug, Clone)]
    pub struct RespContext {
        pub method: Method,
        pub url: Url,
        pub got_status: StatusCode,
        pub response_text: String,
    }
    impl RespContext {
        pub fn body_from_json<B: DeserializeOwned>(&self) -> anyhow::Result<B> {
            serde_json::from_str(&self.response_text).map_err(anyhow::Error::from)
        }
        pub fn expect_status<ErrResp, F: SerialFormat>(
            &self,
            expect_status: StatusCode,
        ) -> Result<(), ClientErr<ErrResp, F>> {
            if self.got_status != expect_status {
                return Err(ClientErr::ExpectedStatus {
                    context: Box::new(self.clone()),
                    expected_status: expect_status,
                });
            }

            Ok(())
        }
    }

    #[derive(Debug)]
    pub struct OkRespWithContext<Body> {
        pub ok_body: Body,
        pub context: RespContext,
    }

    // TODO use
    #[derive(Debug)]
    pub struct ErrRespWithContext<Body> {
        pub err_body: Body,
        pub context: RespContext,
    }
}

pub mod error {
    use self::aliases::ApiResult;
    use super::*;

    pub mod aliases {
        use super::*;
        pub type JsonApiErr<Ok> = ClientErr<Ok, JsonFormat>;
        pub type XmlApiErr<Ok> = ClientErr<Ok, XmlFormat>;

        pub type ApiResult<Ok, ErrResp, F> = Result<Ok, ClientErr<ErrResp, F>>;
        pub type JsonClientResult<Ok, ErrResp> = Result<Ok, JsonApiErr<ErrResp>>;
        pub type XmlApiResult<Ok, ErrResp> = Result<Ok, XmlApiErr<ErrResp>>;
    }

    #[derive(thiserror::Error, Debug)]
    pub enum ClientErr<ErrResp, F: SerialFormat> {
        BuildRequest(reqwest::Error),
        ExecuteRequest(reqwest::Error),
        ReadRespBodyText(reqwest::Error),
        ExpectedErrorResponse {
            context: Option<RespContext>,
        },
        ExpectedStatus {
            context: Box<RespContext>,
            expected_status: StatusCode,
        },
        DeserializeError {
            context: RespContext,
            deserialize_error: F::Error,
        },
        ErrorResponse {
            context: RespContext,
            err_body: ErrResp,
        },
    }
    impl<ErrResp, F: SerialFormat> ClientErr<ErrResp, F> {
        pub fn context(&self) -> Option<&RespContext> {
            match self {
                ClientErr::BuildRequest(_) => None,
                ClientErr::ExecuteRequest(_) => None,
                ClientErr::ReadRespBodyText(_) => None,
                ClientErr::ExpectedErrorResponse { context } => context.as_ref(),
                ClientErr::ExpectedStatus { context, .. } => Some(context),
                ClientErr::DeserializeError { context, .. } => Some(context),
                ClientErr::ErrorResponse { context, .. } => Some(context),
            }
        }
        pub fn response_text(&self) -> Option<&str> {
            self.context().map(|ctx| ctx.response_text.as_str())
        }
    }
    impl<ErrResp: DeserializeOwned, F: SerialFormat> ClientErr<ErrResp, F> {
        pub fn try_into_err_resp(
            self,
            expect_status: StatusCode,
        ) -> Result<ErrResp, ClientErr<ErrResp, F>> {
            if let Some(context) = self.context() {
                context.expect_status(expect_status)?;
            }
            match self {
                ClientErr::ErrorResponse {
                    err_body: source, ..
                } => Ok(source),
                _ => Err(self),
            }
        }
    }
    impl<ErrResp: std::fmt::Display, F: SerialFormat> std::fmt::Display for ClientErr<ErrResp, F>
    where
        F::Error: std::fmt::Display,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(RespContext { method, url, .. }) = self.context() {
                writeln!(f, "{method} {url}")?;
            }

            let error_msg_core = match self {
                    ClientErr::BuildRequest(e) => format!("Failed building request: {e}"),
                    ClientErr::ExecuteRequest(e) => format!("Failed executing request: {e}"),
                    ClientErr::ReadRespBodyText(e) => format!("Failed reading response text: {e}"),
                    ClientErr::ExpectedErrorResponse { .. } => {
"Expected error response, got success".to_string()
                    }
                    ClientErr::ExpectedStatus {
                        context,
                        expected_status,
                    } => {
let got_status = context.got_status;
                        format!("Expected status: {expected_status}, got: {got_status}")
                    },
                    ClientErr::DeserializeError {
                        context: RespContext { response_text, .. },
                        deserialize_error,
                    } => format!(
                        "Failed deserializing JSON response: {deserialize_error}, response_body: {response_text}"
                    ),
                    ClientErr::ErrorResponse { err_body: source, .. } => {
                        format!("Got API error response: {source}")
                    }
                };
            writeln!(f, "{error_msg_core}")?;

            if let Some(response_text) = self.response_text() {
                writeln!(f, "{response_text}")?;
            }

            Ok(())
        }
    }

    pub trait ResultExt<F: SerialFormat> {
        type ErrResp;
        fn try_into_err_resp(
            self,
            expect_status: StatusCode,
        ) -> Result<Self::ErrResp, ClientErr<Self::ErrResp, F>>;
    }
    impl<Ok, ErrResp: DeserializeOwned, F: SerialFormat> ResultExt<F> for ApiResult<Ok, ErrResp, F> {
        type ErrResp = ErrResp;
        fn try_into_err_resp(
            self,
            expect_status: StatusCode,
        ) -> Result<ErrResp, ClientErr<ErrResp, F>> {
            match self {
                Ok(_) => Err(ClientErr::ExpectedErrorResponse { context: None }),
                Err(err) => err.try_into_err_resp(expect_status),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    use crate::context::RespContext;
    use crate::prelude::*;
    use crate::serialization_formats::JsonFormat;
    use crate::{ApiClient, JsonApiClient, ReceiveJson};
    use reqwest::{Method, StatusCode};
    use serde::Deserialize;
    use serde_json::Value;

    #[derive(Default)]
    pub struct ExampleApi {
        http_client: reqwest::Client,
    }
    impl JsonApiClient for ExampleApi {
        fn base_url(&self) -> &str {
            "https://petstore.swagger.io/v2"
        }
        fn http_client(&self) -> &reqwest::Client {
            &self.http_client
        }
    }
    impl ExampleApi {
        pub async fn call_something(&self) -> ClientResult<Value> {
            let req = self
                .get("/pet/findByStatus")
                .query(&[("status", "available")]);
            req.recv_json::<Value, CustomApiError>().await
        }
        pub async fn call_something_broken(&self) -> ClientResult<Value> {
            let req = self.get("/endpoint-doesnt-exist");
            req.recv_json::<Value, CustomApiError>().await
        }
    }

    #[derive(thiserror::Error, Deserialize, Debug, Clone)]
    #[error("Pet API err response: {message}")]
    pub struct CustomApiError {
        pub message: String,
    }
    pub type ClientResult<T> = JsonClientResult<T, CustomApiError>;

    #[tokio::test]
    async fn api_ergonomics() -> anyhow::Result<()> {
        let client = ExampleApi::default();

        // normal case: expect ok, don't specify Ok type or status
        let _got = client.call_something().await?;

        // error case: don't specify error type, specify status
        let _err = client
            .call_something_broken()
            .await
            .try_into_err_resp(StatusCode::NOT_FOUND)?;

        Ok(())
    }

    #[test]
    fn test_expect_err() -> anyhow::Result<()> {
        const ERR_MSG: &str = "some error message";
        let err_context = RespContext {
            method: Method::GET,
            url: "http://hello.com".parse()?,
            got_status: StatusCode::BAD_REQUEST,
            response_text: format!("{{\"message\":\"{ERR_MSG}\"}}"),
        };

        // with inner err
        let inner_err = ClientErr::<CustomApiError, JsonFormat>::ErrorResponse {
            context: err_context.clone(),
            err_body: err_context.body_from_json()?,
        };
        let parsed_err = inner_err.try_into_err_resp(StatusCode::BAD_REQUEST)?;
        assert_eq!(parsed_err.message, ERR_MSG);

        // with err Result

        let err_result =
            ClientResult::<CustomApiError>::Err(ClientErr::<CustomApiError, _>::ErrorResponse {
                context: err_context.clone(),
                err_body: err_context.body_from_json()?,
            });
        let parsed_err = err_result.try_into_err_resp(StatusCode::BAD_REQUEST)?;
        assert_eq!(parsed_err.message, ERR_MSG);

        Ok(())
    }

    #[test]
    fn test_expect_err__wrong_status() -> anyhow::Result<()> {
        const ERR_MSG: &str = "some error message";
        let err_context = RespContext {
            method: Method::GET,
            url: "http://hello.com".parse()?,
            got_status: StatusCode::BAD_REQUEST,
            response_text: format!("{{\"message\":\"{ERR_MSG}\"}}"),
        };

        // with inner err
        let inner_err = ClientErr::<CustomApiError, JsonFormat>::ErrorResponse {
            context: err_context.clone(),
            err_body: err_context.body_from_json()?,
        };

        let should_err = inner_err.try_into_err_resp(StatusCode::NOT_FOUND);
        let err = should_err.expect_err("should be error with wrong status");
        assert!(err
            .to_string()
            .contains("Expected status: 404 Not Found, got: 400 Bad Request"));

        Ok(())
    }
}
