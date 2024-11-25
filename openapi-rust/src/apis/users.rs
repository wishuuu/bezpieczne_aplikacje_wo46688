use async_trait::async_trait;
use axum::extract::*;
use axum_extra::extract::{CookieJar, Multipart};
use bytes::Bytes;
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{models, types::*};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum CreateUserResponse {
    /// User created successfully
    Status201_UserCreatedSuccessfully
    (models::UserResponse)
    ,
    /// Bad request
    Status400_BadRequest
    (models::Error)
    ,
    /// Unauthorized
    Status401_Unauthorized
    (models::Error)
    ,
    /// Unprocessable entity. Codes: USER_ALREADY_EXISTS
    Status422_UnprocessableEntity
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteUserResponse {
    /// No content
    Status204_NoContent
    ,
    /// Bad request
    Status400_BadRequest
    (models::Error)
    ,
    /// Unauthorized
    Status401_Unauthorized
    (models::Error)
    ,
    /// User not found
    Status404_UserNotFound
    (models::Error)
    ,
    /// Unprocessable entity.
    Status422_UnprocessableEntity
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetAllUsersResponse {
    /// Success
    Status200_Success
    (models::UserListResponse)
    ,
    /// Bad request
    Status400_BadRequest
    (models::Error)
    ,
    /// Unauthorized
    Status401_Unauthorized
    (models::Error)
    ,
    /// Unprocessable entity. Codes: USER_ALREADY_EXISTS
    Status422_UnprocessableEntity
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetUserByIdResponse {
    /// Success
    Status200_Success
    (models::UserResponse)
    ,
    /// Bad request
    Status400_BadRequest
    (models::Error)
    ,
    /// Unauthorized
    Status401_Unauthorized
    (models::Error)
    ,
    /// User not found
    Status404_UserNotFound
    (models::Error)
    ,
    /// Unprocessable entity. Codes: USER_ALREADY_EXISTS
    Status422_UnprocessableEntity
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum UpdateUserResponse {
    /// Success
    Status200_Success
    (models::UserResponse)
    ,
    /// Bad request
    Status400_BadRequest
    (models::Error)
    ,
    /// Unauthorized
    Status401_Unauthorized
    (models::Error)
    ,
    /// User not found
    Status404_UserNotFound
    (models::Error)
    ,
    /// Unprocessable entity.
    Status422_UnprocessableEntity
    (models::Error)
}


/// Users
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Users {
  type Claims;

    /// Create.
    ///
    /// CreateUser - POST /api/users
    async fn create_user(
    &self,
    method: Method,
    host: Host,
    cookies: CookieJar,
            body: models::CreateRequest,
    ) -> Result<CreateUserResponse, ()>;

    /// Delete user.
    ///
    /// DeleteUser - DELETE /api/users/{id}
    async fn delete_user(
    &self,
    method: Method,
    host: Host,
    cookies: CookieJar,
        claims: Self::Claims,
      path_params: models::DeleteUserPathParams,
    ) -> Result<DeleteUserResponse, ()>;

    /// Get users list.
    ///
    /// GetAllUsers - GET /api/users
    async fn get_all_users(
    &self,
    method: Method,
    host: Host,
    cookies: CookieJar,
    ) -> Result<GetAllUsersResponse, ()>;

    /// Get user.
    ///
    /// GetUserById - GET /api/users/{id}
    async fn get_user_by_id(
    &self,
    method: Method,
    host: Host,
    cookies: CookieJar,
      path_params: models::GetUserByIdPathParams,
    ) -> Result<GetUserByIdResponse, ()>;

    /// Update user.
    ///
    /// UpdateUser - PUT /api/users/{id}
    async fn update_user(
    &self,
    method: Method,
    host: Host,
    cookies: CookieJar,
        claims: Self::Claims,
      path_params: models::UpdateUserPathParams,
            body: models::UpdateRequest,
    ) -> Result<UpdateUserResponse, ()>;
}
