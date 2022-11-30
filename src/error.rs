use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub enum AppError {
    TooManyRequests,
    Anyhow(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Anyhow(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)).into_response()
            }
            Self::TooManyRequests => {
                (StatusCode::TOO_MANY_REQUESTS, "Too many requests").into_response()
            }
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
