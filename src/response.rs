use axum::Json;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Debug, Serialize)]
pub enum SuccessResult {
    #[serde(rename = "success")]
    Success,
}

#[derive(Debug, Serialize)]
pub enum ErrorResult {
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Serialize)]
pub enum EmptyMsg {
    #[serde(rename = "")]
    Empty,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    BadRequest,
    BadEventQueueId,
}

#[derive(Debug, Serialize)]
pub struct JsonSuccess<T> {
    result: SuccessResult,
    msg: EmptyMsg,
    #[serde(flatten)]
    inner: T,
}

#[derive(Debug, Serialize)]
pub struct JsonError<'a> {
    result: ErrorResult,
    msg: Cow<'a, str>,
    code: ErrorCode,
}

pub fn json_success<T>(inner: T) -> Json<JsonSuccess<T>> {
    Json(JsonSuccess {
        result: SuccessResult::Success,
        msg: EmptyMsg::Empty,
        inner,
    })
}

pub fn json_error<'a>(message: impl Into<Cow<'a, str>>) -> Json<JsonError<'a>> {
    Json(JsonError {
        result: ErrorResult::Error,
        msg: message.into(),
        code: ErrorCode::BadRequest,
    })
}

pub fn json_error_code<'a>(
    message: impl Into<Cow<'a, str>>,
    code: ErrorCode,
) -> Json<JsonError<'a>> {
    Json(JsonError {
        result: ErrorResult::Error,
        msg: message.into(),
        code,
    })
}
