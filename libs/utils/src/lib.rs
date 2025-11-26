pub mod prelude {
    pub use crate::api_client_utils::*;
    pub use crate::errors::*;
}

pub mod errors {
    use snafu::{ResultExt, Whatever};

    pub type Res<T, E = snafu::Whatever> = std::result::Result<T, E>;

    pub trait AutoSnafu<T, E> {
        fn uw(self) -> Result<T, Whatever>;
    }
    impl<T, E: snafu::Error + std::fmt::Display + 'static> AutoSnafu<T, E> for Result<T, E> {
        fn uw(self) -> Result<T, Whatever> {
            // let location = std::line!();
            let caller = std::panic::Location::caller();
            self.with_whatever_context(|e| format!("{e:?} [{}:{}]", caller.file(), caller.line()))
        }
    }
}

pub mod api_client_utils {
    use reqwest::{RequestBuilder, StatusCode};
    use serde::de::DeserializeOwned;
    use std::time::Duration;

    pub trait IsApiClient {
        fn base_url(&self) -> &str;
        fn http_client(&self) -> &reqwest::Client;

        fn path(&self, url_path: &str) -> String {
            if url_path.starts_with("http") {
                return url_path.to_string();
            }

            let origin = self.base_url().trim().trim_end_matches('/');
            let path = url_path.trim().trim_start_matches('/');
            format!("{origin}/{path}")
        }
        fn default_params(&self, request_builder: RequestBuilder) -> RequestBuilder {
            request_builder.timeout(Duration::new(5, 0))
        }

        fn get(&self, url_path: &str) -> RequestBuilder {
            self.default_params(self.http_client().get(self.path(url_path)))
        }
        fn post(&self, url_path: &str) -> RequestBuilder {
            self.default_params(self.http_client().post(self.path(url_path)))
        }
    }

    #[async_trait::async_trait]
    pub trait RequestBuilderExt {
        async fn fetch_json<D: DeserializeOwned>(self) -> Result<D, FetchErr>;
        // async fn fetch_bytes(self) -> Result<Vec<u8>, FetchErr>;
        // fn try_build_split(self) -> Result<RequestClient, reqwest::Error>;
    }
    #[async_trait::async_trait]
    impl RequestBuilderExt for RequestBuilder {
        async fn fetch_json<D: DeserializeOwned>(self) -> Result<D, FetchErr> {
            let builder = self
                .header("Content-Type", "application/json")
                .header("Accept", "application/json");
            let (client, req) = builder.build_split();
            let req = req?;

            let method = req.method().clone();
            let resp = client.execute(req).await?;
            let status = resp.status();
            let url = resp.url().clone();
            let resp_text = resp.text().await?;

            if !status.is_success() {
                return Err(FetchErr::ErrResp {
                    method,
                    url,
                    status,
                    body_str: resp_text,
                });
            }
            let deserialized: D =
                serde_json::from_str(&resp_text).map_err(|e| FetchErr::DeserialErr {
                    body_str: resp_text,
                    source: e,
                })?;
            Ok(deserialized)
        }
        // async fn fetch_bytes(self) -> Result<Vec<u8>, FetchErr> {
        //     let builder = self
        //         .header("Content-Type", "application/json")
        //         .header("Accept", "application/json");
        //     let (client, req) = builder.build_split();
        //     let req = req?;

        //     let method = req.method().clone();
        //     let resp = client.execute(req).await?;
        //     let status = resp.status();
        //     let url = resp.url().clone();
        //     let resp_bytes = resp.bytes().await?;

        //     if !status.is_success() {
        //         return Err(FetchErr::ErrResp {
        //             method,
        //             url,
        //             status,
        //             body_str: String::from_utf8_lossy(&resp_bytes).to_string(),
        //         });
        //     }
        //     Ok(resp_bytes.to_vec())
        // }

        // fn try_build_split(self) -> Result<RequestClient, reqwest::Error> {
        //     let (client, request_result) = self.build_split();
        //     let request = request_result?;
        //     Ok(RequestClient { request, client })
        // }
        // fn request(self) ->
    }

    #[derive(thiserror::Error, Debug)]
    pub enum FetchErr {
        #[error("Failed sending request: {0}")]
        ReqwestErr(#[from] reqwest::Error),
        #[error("{method} {url} \nReceived {status} error response: {body_str}")]
        ErrResp {
            method: reqwest::Method,
            url: reqwest::Url,
            status: StatusCode,
            body_str: String,
        },
        #[error("Failed deserializing: body: {body_str}")]
        DeserialErr {
            body_str: String,
            source: serde_json::Error,
        },
    }

    // pub struct RequestClient {
    //     pub request: reqwest::Request,
    //     pub client: reqwest::Client,
    // }
}
