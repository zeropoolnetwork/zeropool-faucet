use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorResp {
    error: String,
}

pub enum AppError {
    LimitExceeded,
    Anyhow(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Anyhow(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResp {
                    error: err.to_string(),
                }),
            )
                .into_response(),
            Self::LimitExceeded => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResp {
                    error: "Limit exceeded".to_owned(),
                }),
            )
                .into_response(),
        }
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Anyhow(err.into())
    }
}
