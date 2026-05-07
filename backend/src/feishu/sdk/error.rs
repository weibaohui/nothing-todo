use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum LarkAPIError {
    #[error("IO error: {0}")]
    IOErr(String),
    #[error("Invalid parameter: {0}")]
    IllegalParamError(String),
    #[error("JSON deserialization error: {0}")]
    DeserializeError(String),
    #[error("HTTP request failed: {0}")]
    RequestError(String),
    #[error("URL parse error: {0}")]
    UrlParseError(String),
    #[error("API error: {message} (code: {code}, request_id: {request_id:?})")]
    ApiError {
        code: i32,
        message: String,
        request_id: Option<String>,
    },
    #[error("Missing access token")]
    MissingAccessToken,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Data error: {0}")]
    DataError(String),
    #[error("API error: {msg} (code: {code})")]
    APIError {
        code: i32,
        msg: String,
        error: Option<String>,
    },
}

impl From<std::io::Error> for LarkAPIError {
    fn from(err: std::io::Error) -> Self {
        Self::IOErr(err.to_string())
    }
}

impl From<serde_json::Error> for LarkAPIError {
    fn from(err: serde_json::Error) -> Self {
        Self::DeserializeError(err.to_string())
    }
}

impl From<reqwest::Error> for LarkAPIError {
    fn from(err: reqwest::Error) -> Self {
        Self::RequestError(err.to_string())
    }
}

impl From<url::ParseError> for LarkAPIError {
    fn from(err: url::ParseError) -> Self {
        Self::UrlParseError(err.to_string())
    }
}

pub type SDKResult<T> = Result<T, LarkAPIError>;
