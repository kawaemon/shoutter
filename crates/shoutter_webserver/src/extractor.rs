use std::error::Error;

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::FromRequest;
use axum::http::Request;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, http};
use prost::DecodeError;

pub struct Proto<T: prost::Message + Default>(pub T);

#[derive(thiserror::Error, Debug)]
pub enum ProtoRequestFailure {
    #[error("The request could not be handled as bytes.")]
    ByteParseError(Response),

    #[error("The request is malformed (likely unexpected type for the endpoint, or the request body is not Protobuf payload?)")]
    MalformedRequest(DecodeError),
}

impl IntoResponse for ProtoRequestFailure {
    fn into_response(self) -> Response {
        let mut base = self.to_string().into_response();
        (*base.status_mut()) = http::StatusCode::BAD_REQUEST;

        base
    }
}

#[async_trait]
impl<S, B, T> FromRequest<S, B> for Proto<T>
where
    Bytes: FromRequest<S, B>,
    B: Send + 'static,
    S: Send + Sync,
    T: prost::Message + Default,
{
    type Rejection = ProtoRequestFailure;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)
            .map_err(ProtoRequestFailure::ByteParseError)?;

        let decoded = T::decode(bytes).map_err(ProtoRequestFailure::MalformedRequest)?;

        Ok(Proto(decoded))
    }
}
