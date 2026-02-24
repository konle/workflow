use axum::{http::StatusCode, response::IntoResponse, Json};
use crate::response::response::Response;

pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn internal(message: impl ToString) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.to_string(),
        }
    }

    pub fn bad_request(message: impl ToString) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            message: message.to_string(),
        }
    }

    pub fn not_found(message: impl ToString) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            message: message.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Response::<()>::error(self.status.as_u16() as i32, self.message);
        (self.status, Json(body)).into_response()
    }
}

// 从 Box<dyn Error> 自动转换为 ApiError
impl From<Box<dyn std::error::Error + Send + Sync>> for ApiError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ApiError::internal(e.to_string())
    }
}
