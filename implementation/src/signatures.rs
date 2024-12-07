use std::sync::Arc;

use axum::{
    async_trait, debug_handler,
    extract::{FromRequestParts, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose, Engine};
use hmac::{Hmac, Mac};
use http::{request::Parts, StatusCode};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[derive(Clone)]
pub struct SecretsConfig {
    pub hmac_secret: String,
    pub jws_secret: Vec<u8>,
    pub allowed_algorithms: Vec<Algorithm>,
}

#[derive(Debug)]
pub enum SignatureError {
    MissingSignature(String),
    InvalidSignature,
    JwsVerificationFailed,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JwtClaims {
    scope: Vec<String>,
    exp: i64,
    jti: String,
    client_id: String,
}

impl IntoResponse for SignatureError {
    fn into_response(self) -> Response {
        match self {
            SignatureError::MissingSignature(s) => {
                (StatusCode::BAD_REQUEST, format!("Missing {s} signature")).into_response()
            }
            SignatureError::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, "Invalid signature").into_response()
            }
            SignatureError::JwsVerificationFailed => {
                (StatusCode::UNAUTHORIZED, "JWS verification failed").into_response()
            }
        }
    }
}

pub async fn verify_hmac_signature(
    State(config): State<Arc<SecretsConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, SignatureError> {
    if request.method() != http::Method::POST {
        return Ok(next.run(request).await);
    }
    let signature = request
        .headers()
        .get("X-HMAC-SIGNATURE")
        .ok_or(SignatureError::MissingSignature("HMAC".into()))?
        .to_str()
        .map_err(|_| SignatureError::InvalidSignature)?
        .to_owned();
    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| SignatureError::InvalidSignature)?;

    let mut mac = Hmac::<Sha256>::new_from_slice(config.hmac_secret.as_bytes())
        .map_err(|_| SignatureError::InvalidSignature)?;
    mac.update(&body_bytes);

    let computed_signature = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    if computed_signature != signature {
        return Err(SignatureError::InvalidSignature);
    }

    let request = Request::from_parts(parts, axum::body::Body::from(body_bytes));

    Ok(next.run(request).await)
}

pub async fn verify_jws_signature(
    State(config): State<Arc<SecretsConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, SignatureError> {
    if request.method() != http::Method::PUT {
        return Ok(next.run(request).await);
    }
    let signature_header = request
        .headers()
        .get("X-JWS-SIGNATURE")
        .ok_or(SignatureError::MissingSignature("JWS".into()))?
        .to_str()
        .map_err(|_| SignatureError::InvalidSignature)?
        .to_owned();

    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| SignatureError::InvalidSignature)?;

    let token_data = decode::<()>(
        &signature_header,
        &DecodingKey::from_secret(&config.jws_secret),
        &Validation::default(),
    )
    .map_err(|_| SignatureError::JwsVerificationFailed)?;

    let request = Request::from_parts(parts, axum::body::Body::from(body_bytes));

    Ok(next.run(request).await)
}

pub fn create_hmac_signature(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}
