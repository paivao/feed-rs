use actix_web::http::StatusCode;
use actix_web::{
    HttpRequest, HttpResponse, Result, delete, get, post, put,
    web::{self, Json},
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::auth::{self, SessionStore};
use crate::error::{ApiError, json_error};
use crate::model::user::User;
use crate::utils::{db_error, not_found_or_db_error};

/// Unauthenticated: session-issuing login and its counterpart logout.
pub fn configure_auth_api(cfg: &mut web::ServiceConfig) {
    cfg.service(login).service(logout);
}

pub fn configure_user_api(cfg: &mut web::ServiceConfig) {
    cfg.service(me).service(
        web::scope("/user")
            .service(list_users)
            .service(get_user)
            .service(create_user)
            .service(update_user)
            .service(delete_user),
    );
}

/// The currently logged-in user, as resolved by `auth::session_validator` from the
/// session cookie. 401s (via the middleware) if there's no valid session.
#[get("/me")]
async fn me(user: web::ReqData<User>) -> Result<Json<User>> {
    Ok(Json(user.into_inner()))
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
    match User::verify_credentials(&pool, &info.username, &info.password).await {
        None => Err(json_error(StatusCode::UNAUTHORIZED, "invalid credentials")),
        Some(user) => {
            let cookie = auth::issue_session_cookie(&sessions, user.clone());
            Ok(HttpResponse::Ok().cookie(cookie).json(user))
        }
    }
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
    search: Option<String>,
}

#[get("/")]
async fn list_users(
    pool: web::Data<PgPool>,
    query: web::Query<ListQuery>,
) -> Result<Json<Vec<User>>> {
    let query = query.into_inner();
    let users = User::list_all(
        &pool,
        query.limit.unwrap_or(50),
        query.offset.unwrap_or(0),
        query.search.as_deref(),
    )
    .await
    .map_err(|err| db_error("users", 0, err))?;
    Ok(Json(users))
}

#[get("/{id}")]
async fn get_user(pool: web::Data<PgPool>, id: web::Path<i64>) -> Result<Json<User>> {
    let user = User::get_by_id(&pool, *id)
        .await
        .map_err(|err| not_found_or_db_error("user", *id, err))?;
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
        .map_err(|err| api_error("new user", 0, err))?;
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
        .map_err(|err| not_found_or_db_error("user", *id, err))?;
    user.name = info.name;
    user.email = info.email;
    user.update(&pool)
        .await
        .map_err(|err| not_found_or_db_error("user", *id, err))?;
    Ok(Json(user))
}

#[delete("/{id}")]
async fn delete_user(pool: web::Data<PgPool>, id: web::Path<i64>) -> Result<HttpResponse> {
    let deleted = User::delete(&pool, *id)
        .await
        .map_err(|err| db_error("user", *id, err))?;
    if deleted {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(json_error(StatusCode::NOT_FOUND, "user not found"))
    }
}

// PRIVATE FUNCTIONS

fn api_error(entity: &'static str, id: i64, err: ApiError) -> actix_web::Error {
    match err {
        ApiError::DB(e) => db_error(entity, id, e),
        ApiError::InvalidPassword => json_error(StatusCode::UNAUTHORIZED, "invalid credentials"),
        ApiError::NotFound => json_error(StatusCode::NOT_FOUND, format!("{entity} not found")),
        other => {
            crate::app_warn!("Error on {} {}: {:?}", entity, id, other);
            json_error(StatusCode::BAD_REQUEST, "error in request")
        }
    }
}
