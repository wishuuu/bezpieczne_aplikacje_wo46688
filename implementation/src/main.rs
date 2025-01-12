pub mod encryption;
mod signatures;

use aes_gcm::aead::OsRng;
use async_trait::async_trait;
use axum::extract::Host;
use axum::middleware::{self};
use axum::routing::post;
use axum::{Json, Router};
use axum_extra::extract::CookieJar;
use base64::prelude::*;
use encryption::{get_body_decryption, get_body_encryption};
use http::Method;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use openapi::apis::users::{
    CreateUserResponse, DeleteUserResponse, GetAllUsersResponse, GetUserByIdResponse,
    UpdateUserResponse,
};
use openapi::apis::ApiKeyAuthHeader;
use openapi::models::{
    CreateRequest, DeleteUserPathParams, Error, GetUserByIdPathParams, RequestHeader,
    ResponseHeader, UpdateRequest, UpdateUserPathParams, User, UserListResponse, UserResponse,
};
use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use signatures::{verify_hmac_signature, verify_jws_signature, JwtClaims, SecretsConfig};
use std::io::Write;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::RwLock;
use uuid::{uuid, Uuid};
use validator::Validate;

struct ServerImpl {
    users: Arc<RwLock<std::collections::HashMap<Uuid, User>>>,
    skip_auth: bool,
}

impl ServerImpl {
    fn new(skip_auth: bool) -> Self {
        let mut users = std::collections::HashMap::new();
        users.insert(
            uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            User {
                id: Some(uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8")),
                name: "Adam".into(),
                surname: "Mickiewicz".into(),
                email: Some("mickiewicz@o2.pl".into()),
                age: 37,
                personal_id: "12345678900".into(),
                citizenship: "PL".into(),
            },
        );

        ServerImpl {
            users: Arc::new(RwLock::new(users)),
            skip_auth,
        }
    }
}

fn build_response_header() -> ResponseHeader {
    ResponseHeader {
        request_id: Uuid::new_v4(),
        send_date: chrono::Utc::now(),
    }
}

fn build_request_header() -> RequestHeader {
    RequestHeader {
        request_id: Uuid::new_v4(),
        send_date: chrono::Utc::now(),
    }
}

#[allow(unused_variables)]
impl ServerImpl {
    async fn create_user_func(&self, mut body: CreateRequest) -> Option<User> {
        let uuid = Uuid::new_v4();
        body.user.id = Some(uuid);
        self.users.write().await.insert(uuid, body.user.clone())
    }
    async fn delete_user_func(&self, path_params: DeleteUserPathParams) -> Option<User> {
        self.users.write().await.remove(&path_params.id)
    }
    async fn get_all_users_func(&self) -> Vec<User> {
        self.users
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>()
    }
}

#[allow(unused_variables)]
#[async_trait]
impl openapi::apis::users::Users for ServerImpl {
    type Claims = ();

    async fn create_user(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        mut body: CreateRequest,
    ) -> Result<CreateUserResponse, ()> {
        let val = body.user.validate();
        if let Err(e) = val {
            return Ok(CreateUserResponse::Status400_BadRequest(Error::new(
                build_response_header(),
                e.to_string(),
            )));
        };
        let uuid = Uuid::new_v4();
        body.user.id = Some(uuid);
        self.users.write().await.insert(uuid, body.user.clone());
        Ok(CreateUserResponse::Status201_UserCreatedSuccessfully(
            UserResponse {
                response_header: body.request_header,
                user: body.user,
            },
        ))
    }

    async fn delete_user(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: DeleteUserPathParams,
    ) -> Result<DeleteUserResponse, ()> {
        match self.users.write().await.remove(&path_params.id) {
            None => Ok(DeleteUserResponse::Status404_UserNotFound(Error::new(
                build_response_header(),
                "404".into(),
            ))),
            Some(user) => Ok(DeleteUserResponse::Status204_NoContent),
        }
    }

    async fn get_all_users(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
    ) -> Result<GetAllUsersResponse, ()> {
        Ok(GetAllUsersResponse::Status200_Success(UserListResponse {
            response_header: build_request_header(),
            users_list: self.users.read().await.values().cloned().collect(),
        }))
    }

    async fn get_user_by_id(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: GetUserByIdPathParams,
    ) -> Result<GetUserByIdResponse, ()> {
        match self.users.read().await.get(&path_params.id) {
            None => Ok(GetUserByIdResponse::Status404_UserNotFound(Error::new(
                build_response_header(),
                "404".into(),
            ))),
            Some(user) => Ok(GetUserByIdResponse::Status200_Success(UserResponse {
                response_header: build_request_header(),
                user: user.clone(),
            })),
        }
    }

    async fn update_user(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: UpdateUserPathParams,
        mut body: UpdateRequest,
    ) -> Result<UpdateUserResponse, ()> {
        let val = body.user.validate();
        body.user.id = Some(path_params.id);
        if let Err(e) = val {
            return Ok(UpdateUserResponse::Status400_BadRequest(Error::new(
                build_response_header(),
                e.to_string(),
            )));
        };
        let mut collection = self.users.write().await;
        match collection.get(&path_params.id) {
            None => Ok(UpdateUserResponse::Status404_UserNotFound(Error::new(
                build_response_header(),
                "404".into(),
            ))),
            Some(user) => {
                collection.remove(&path_params.id);
                collection.insert(path_params.id, body.user.clone());
                Ok(UpdateUserResponse::Status200_Success(UserResponse {
                    response_header: build_request_header(),
                    user: body.user.clone(),
                }))
            }
        }
    }
}

fn validate_jwt_token(token: &str, public_key: &str) -> Result<JwtClaims, String> {
    let validation = Validation::new(Algorithm::RS256);
    let decoding_key = match DecodingKey::from_rsa_pem(public_key.as_bytes()) {
        Ok(key) => key,
        Err(_) => return Err("Failed to parse public key".to_string()),
    };

    // Decode and validate the token
    match jsonwebtoken::decode::<JwtClaims>(token, &decoding_key, &validation) {
        Ok(token_data) => Ok(token_data.claims),
        Err(err) => Err(format!("Token validation failed: {}", err)),
    }
}

impl ApiKeyAuthHeader for ServerImpl {
    type Claims = ();

    #[doc = " Extracting Claims from Header. Return None if the Claims is invalid."]
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn extract_claims_from_header<'a, 'b, 'c, 'async_trait>(
        &'a self,
        _headers: &'b axum::http::header::HeaderMap,
        _key: &'c str,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Option<Self::Claims>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'a: 'async_trait,
        'b: 'async_trait,
        'c: 'async_trait,
        Self: 'async_trait,
    {
        if self.skip_auth {
            return Box::pin(async move { Some(()) });
        }
        match _key {
            "Bearer" => {
                let key = "-----BEGIN CERTIFICATE-----MIIDQDCCAiigAwIBAgIEX8EtRzANBgkqhkiG9w0BAQsFADBiMQswCQYDVQQGEwJQTDELMAkGA1UECAwCWlMxETAPBgNVBAcMCFN6Y3plY2luMQswCQYDVQQKDAJXSTEMMAoGA1UECwwDWlVUMRgwFgYDVQQDDA9QQkEgQVVUSCBTRVJWRVIwHhcNMjAxMTI3MTY0NTU5WhcNMjExMTI3MTY0NTU5WjBiMQswCQYDVQQGEwJQTDELMAkGA1UECAwCWlMxETAPBgNVBAcMCFN6Y3plY2luMQswCQYDVQQKDAJXSTEMMAoGA1UECwwDWlVUMRgwFgYDVQQDDA9QQkEgQVVUSCBTRVJWRVIwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDEFcp+Uic4iKcGvZjSsQH1WQOn/5vNcwHRw+v3jAtSxXa5jzAjSPYmiuYmZTYmU1aIiCckVU0HMWG85NPp55Evvb54odYKJnPYUoRyNNM+3XkF2Pvwd7lYvPcHl7MK9kylgdszz41DXAKRC3cb9ku3FnvWGPrRXT9HFc/WW0VJxncgYXM2kjWfDXV+hBPN47GaBi7SK6ohBdgFroilsFHZUpwpdr1rgzh7aMHoWKx+cRp7vTUqGaMcw+jelTDNG2txJ6AFOa0QJBpbrrImJtexoSsvPhHSUSXKMCDy4PghkuueLbpXXYeot6tVjeC5GblTaz1TYcEMpWiEP99NMnQzAgMBAAEwDQYJKoZIhvcNAQELBQADggEBAC1Re3Fh6BmMuX+rdu3OWbX9WONw7xYTWaXDvGtg/qczTIp4DA6YlxpTCMLANnepHpk4O9b1ml2ukWzymq+YuT4XzBZU2RtHwtHqaal/KTHGYsVY9t8W6aUEArPdrUeQ3bIzj19KZbRawlA9o6tWRDBDnF8fPAxNLz0YjWHAhZC5TgPbmgWcTOQ5ddrJ5vrQWI9spRtWCuAXLz1dBgqujtBgTls5eU1nYWkH7Wy42TePWKIJDbIwQrb8wJWih/7BS2O0Skpa3T8Z3mryIfoaLZLrY9tn5sBXl3fILwce+Or6NDTV0toBb2gNTBJNNei+0jKD9yoAl8ffxN+o8x4uzYg=-----END CERTIFICATE-----";
                let value = _headers
                    .get("Authorization")
                    .and_then(|value| value.to_str().ok());
                if let Some(value) = value {
                    let token_str = value.replace("Bearer ", "");
                    println!("Flaga 1");
                    println!("{:?}", token_str);
                    let token = validate_jwt_token(&token_str, key);
                    if let Ok(_token) = token {
                        Box::pin(async move { Some(()) })
                    } else {
                        token.err().map(|e| println!("{:?}", e));
                        Box::pin(async move { None })
                    }
                } else {
                    Box::pin(async move { None })
                }
            }
            "Basic" => {
                let value = _headers
                    .get("Authorization")
                    .and_then(|value| value.to_str().ok());
                if let Some(value) = value {
                    let value = value.to_string();
                    let value = value.split(' ').last().unwrap();
                    let value = base64::engine::general_purpose::STANDARD.decode(value);
                    let value = String::from_utf8(value.unwrap()).unwrap();

                    if value == "wo46688:123456" {
                        Box::pin(async move { Some(()) })
                    } else {
                        Box::pin(async move { None })
                    }
                } else {
                    Box::pin(async move { None })
                }
            }
            _ => Box::pin(async move { None }),
        }
    }
}

pub async fn get_body_hash_hmac(body: String) -> Json<String> {
    let body = body.as_bytes();
    let hmac_signature = signatures::create_hmac_signature("123456", body);
    Json(hmac_signature)
}

pub async fn get_body_hash_jws(body: String) -> Json<String> {
    let body = body.as_bytes();
    let jws_signature = signatures::create_jws_signature(b"123456", body);
    Json(jws_signature)
}

pub async fn get_app(skip_singnatures: bool) -> Router {
    let config = Arc::new(SecretsConfig {
        hmac_secret: "123456".into(),
        jws_secret: b"123456".to_vec(),
    });

    let mut rng = OsRng;
    let bits = 2048;
    let private = &RsaPrivateKey::new(&mut rng, bits).unwrap();
    let public = &RsaPublicKey::from(private);
    let private_pem = private
        .to_pkcs1_pem(rsa::pkcs8::LineEnding::LF)
        .unwrap()
        .to_string();
    let public_pem = public.to_pkcs1_pem(rsa::pkcs8::LineEnding::LF).unwrap();

    let mut file = std::fs::File::create("private.pem").unwrap();
    file.write_all(private_pem.as_bytes()).unwrap();
    let mut file = std::fs::File::create("public.pem").unwrap();
    file.write_all(public_pem.as_bytes()).unwrap();

    let app = openapi::server::new(Arc::new(ServerImpl::new(skip_singnatures)));

    if !skip_singnatures {
        app.layer(middleware::from_fn_with_state(
            config.clone(),
            verify_hmac_signature,
        ))
        .layer(middleware::from_fn_with_state(
            config.clone(),
            verify_jws_signature,
        ))
        .route("/hash/hmac", post(get_body_hash_hmac))
        .route("/hash/jws", post(get_body_hash_jws))
        .route("/encData", post(get_body_encryption))
        .route("/decData", post(get_body_decryption))
    } else {
        app
    }
}

pub async fn start_server(addr: &str) {
    let app = get_app(false).await;

    // Run the server with graceful shutdown
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() {
    start_server("0.0.0.0:8080").await;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn add_user() {
        let server = ServerImpl::new(true);

        let users = server.get_all_users_func().await;
        assert_eq!(users.len(), 1);

        let request_body = json!({
            "requestHeader": {
                "requestId": "123e4567-e89b-12d3-a456-426614174000",
                "sendDate": "2024-11-23T12:00:00Z"
            },
            "user": {
                "name": "John",
                "surname": "Doe",
                "age": 30,
                "personalId": "12345678900",
                "citizenship": "PL"
            }
        });

        let user = serde_json::from_value::<CreateRequest>(request_body).unwrap();

        server.create_user_func(user).await;

        let users = server.get_all_users_func().await;
        assert_eq!(users.len(), 2);

        let deletion = server
            .delete_user_func(DeleteUserPathParams {
                id: uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            })
            .await;
        assert!(deletion.is_some());
        let deletion = server
            .delete_user_func(DeleteUserPathParams {
                id: uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            })
            .await;
        assert!(deletion.is_none());
    }
}
