use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose, Engine};
use hmac::{Hmac, Mac};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct SecretsConfig {
    pub hmac_secret: String,
    pub jws_secret: Vec<u8>,
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

    if !signature_header.contains("..") {
        return Err(SignatureError::InvalidSignature);
    }

    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| SignatureError::InvalidSignature)?;

    println!("Body: {:?}", body_bytes);

    let mut hasher = Sha256::new();
    hasher.update(body_bytes.clone());
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &hasher.finalize()[..].to_vec(),
        &jsonwebtoken::EncodingKey::from_secret(&config.jws_secret),
    )
    .map_err(|_| SignatureError::InvalidSignature)?;

    let jws_parts: Vec<&str> = token.split('.').collect();
    let _header = jws_parts[0];
    let _payload = jws_parts[1];
    let signature = jws_parts[2];

    if signature != signature_header.split("..").collect::<Vec<&str>>()[1] {
        println!("Signature: {}", signature);
        println!("Signature header: {}", signature_header);
        return Err(SignatureError::JwsVerificationFailed);
    }

    let request = Request::from_parts(parts, axum::body::Body::from(body_bytes));

    Ok(next.run(request).await)
}

pub fn create_hmac_signature(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

pub fn create_jws_signature(secret: &[u8], body: &[u8]) -> String {
    // create signed JWT of body and remove payload part
    let mut hasher = Sha256::new();
    hasher.update(body);
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &hasher.finalize()[..].to_vec(),
        &jsonwebtoken::EncodingKey::from_secret(secret),
    )
    .unwrap();
    let parts: Vec<&str> = token.split('.').collect();
    let header = parts[0];
    let _payload = parts[1];
    let signature = parts[2];

    format!("{}..{}", header, signature)
}
