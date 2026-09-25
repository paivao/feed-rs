use actix_web::{
    HttpRequest, HttpResponse, Result, delete, error, get, post, put,
    web::{self, Json},
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::auth::{self, SessionStore};
use crate::error::ApiError;
use crate::model::user::User;

/// Unauthenticated: session-issuing login and its counterpart logout.
pub fn configure_auth_api(cfg: &mut web::ServiceConfig) {
    cfg.service(login).service(logout);
}

pub fn configure_user_api(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .service(list_users)
            .service(get_user)
            .service(create_user)
            .service(update_user)
            .service(delete_user),
    );
}

#[derive(Deserialize)]
struct LoginData {
    username: String,
    password: String,
}

#[post("/login")]
async fn login(
    pool: web::Data<PgPool>,
    sessions: web::Data<SessionStore>,
    info: web::Json<LoginData>,
) -> Result<HttpResponse> {
    let info = info.into_inner();
    let user = User::verify_credentials(&pool, &info.username, &info.password)
        .await
        .map_err(|_| error::ErrorUnauthorized("invalid credentials"))?;

    let cookie = auth::issue_session_cookie(&sessions, user.clone());
    Ok(HttpResponse::Ok().cookie(cookie).json(user))
}

#[post("/logout")]
async fn logout(sessions: web::Data<SessionStore>, req: HttpRequest) -> Result<HttpResponse> {
    let token = req
        .cookie(auth::SESSION_COOKIE)
        .map(|c| c.value().to_string());
    let cookie = auth::clear_session_cookie(&sessions, token.as_deref());
    Ok(HttpResponse::Ok().cookie(cookie).finish())
}

#[derive(Deserialize)]
struct CreateUserData {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct UpdateUserData {
    name: String,
    email: String,
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[get("/")]
async fn list_users(
    pool: web::Data<PgPool>,
    query: web::Query<ListQuery>,
) -> Result<Json<Vec<User>>> {
    let query = query.into_inner();
    let users = User::list_all(&pool, query.limit.unwrap_or(50), query.offset.unwrap_or(0))
        .await
        .map_err(db_error)?;
    Ok(Json(users))
}

#[get("/{id}")]
async fn get_user(pool: web::Data<PgPool>, id: web::Path<i64>) -> Result<Json<User>> {
    let user = User::get_by_id(&pool, *id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| error::ErrorNotFound("user not found"))?;
    Ok(Json(user))
}

#[post("/")]
async fn create_user(
    pool: web::Data<PgPool>,
    info: web::Json<CreateUserData>,
) -> Result<Json<User>> {
    let info = info.into_inner();
    let user = User::create(&pool, &info.name, &info.email, &info.password)
        .await
        .map_err(api_error)?;
    Ok(Json(user))
}

#[put("/{id}")]
async fn update_user(
    pool: web::Data<PgPool>,
    id: web::Path<i64>,
    info: web::Json<UpdateUserData>,
) -> Result<Json<User>> {
    let info = info.into_inner();
    let mut user = User::get_by_id(&pool, *id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| error::ErrorNotFound("user not found"))?;
    user.name = info.name;
    user.email = info.email;
    user.update(&pool).await.map_err(api_error)?;
    Ok(Json(user))
}

#[delete("/{id}")]
async fn delete_user(pool: web::Data<PgPool>, id: web::Path<i64>) -> Result<HttpResponse> {
    let deleted = User::delete(&pool, *id).await.map_err(db_error)?;
    if deleted {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(error::ErrorNotFound("user not found"))
    }
}

// PRIVATE FUNCTIONS

fn db_error(err: sqlx::Error) -> actix_web::Error {
    log::warn!(target: &format!("{}::app", crate::APP_NAME), "Database error: {:?}", err);
    error::ErrorBadRequest("error in request")
}

fn api_error(err: ApiError) -> actix_web::Error {
    log::warn!(target: &format!("{}::app", crate::APP_NAME), "Error: {:?}", err);
    error::ErrorBadRequest("error in request")
}
