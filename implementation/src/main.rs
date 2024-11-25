use async_trait::async_trait;
use axum::extract::Host;
use axum_extra::extract::CookieJar;
use http::Method;
use openapi::apis::users::{
    CreateUserResponse, DeleteUserResponse, GetAllUsersResponse, GetUserByIdResponse,
    UpdateUserResponse,
};
use openapi::apis::ApiKeyAuthHeader;
use openapi::models::{
    CreateRequest, DeleteUserPathParams, Error, GetUserByIdPathParams, RequestHeader,
    ResponseHeader, UpdateRequest, UpdateUserPathParams, User, UserListResponse, UserResponse,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::RwLock;
use uuid::Uuid;
use validator::Validate;

struct ServerImpl {
    // database: sea_orm::DbConn,
    users: Arc<RwLock<std::collections::HashMap<Uuid, User>>>,
}

impl ServerImpl {
    fn new() -> Self {
        ServerImpl {
            users: Arc::new(RwLock::new(std::collections::HashMap::new())),
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
#[async_trait]
impl openapi::apis::users::Users for ServerImpl {
    type Claims = ();

    async fn create_user(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        mut body: CreateRequest,
    ) -> Result<CreateUserResponse, ()> {
        let val = body.validate();
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
        body: UpdateRequest,
    ) -> Result<UpdateUserResponse, ()> {
        let val = body.validate();
        if let Err(e) = val {
            return Ok(UpdateUserResponse::Status400_BadRequest(Error::new(
                build_response_header(),
                e.to_string(),
            )));
        };
        let collection = self.users.read().await;
        match collection.get(&path_params.id) {
            None => Ok(UpdateUserResponse::Status404_UserNotFound(Error::new(
                build_response_header(),
                "404".into(),
            ))),
            Some(user) => {
                collection
                    .clone()
                    .insert(user.id.unwrap(), body.user.clone());
                Ok(UpdateUserResponse::Status200_Success(UserResponse {
                    response_header: build_request_header(),
                    user: body.user.clone(),
                }))
            }
        }
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
        Box::pin(async move { Some(()) })
    }
}

pub async fn start_server(addr: &str) {
    // Init Axum router
    let app = openapi::server::new(Arc::new(ServerImpl::new()));

    // Add layers to the router
    // let app = app.layer(...);

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
