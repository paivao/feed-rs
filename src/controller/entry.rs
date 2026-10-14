use actix_web::http::StatusCode;
use actix_web::{
    HttpResponse, Result, delete, get, post, put,
    web::{self, Json},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::types::chrono::{DateTime, Utc};

use crate::error::json_error;
use crate::model::{entry, feed};
use crate::utils::{db_error, not_found_or_db_error};

pub fn configure_entry_api(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/entries/{feed_id}")
            .service(list_entries)
            .service(get_entry)
            .service(create_entry)
            .service(update_entry)
            .service(delete_entry),
    );
}

#[derive(Serialize)]
#[serde(untagged)]
enum EntryResponse {
    IP(entry::IPEntry),
    Domain(entry::DomainEntry),
    URL(entry::URLEntry),
}

#[derive(Deserialize)]
struct CreateEntryData {
    value: String,
    description: Option<String>,
    valid_until: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct UpdateEntryData {
    enabled: bool,
    description: Option<String>,
    valid_until: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct ListQuery {
    quantity: Option<i64>,
    last_id: Option<i64>,
    enabled: Option<bool>,
}

#[get("/")]
async fn list_entries(
    pool: web::Data<PgPool>,
    feed_id: web::Path<i64>,
    query: web::Query<ListQuery>,
) -> Result<Json<Vec<EntryResponse>>> {
    let feed = get_feed_or_404(&pool, *feed_id).await?;
    let query = query.into_inner();
    let quantity = query.quantity.unwrap_or(100);

    let entries = match feed.feed_type {
        feed::FeedType::IP => {
            entry::IPEntry::fetch_some(&pool, &feed, quantity, query.last_id, query.enabled, None)
                .await
                .map(|v| v.into_iter().map(EntryResponse::IP).collect())
        }
        feed::FeedType::Domain => entry::DomainEntry::fetch_some(
            &pool,
            &feed,
            quantity,
            query.last_id,
            query.enabled,
            None,
        )
        .await
        .map(|v| v.into_iter().map(EntryResponse::Domain).collect()),
        feed::FeedType::URL => {
            entry::URLEntry::fetch_some(&pool, &feed, quantity, query.last_id, query.enabled, None)
                .await
                .map(|v| v.into_iter().map(EntryResponse::URL).collect())
        }
    }
    .map_err(|err| db_error("entries of feed", feed.id, err))?;

    Ok(Json(entries))
}

#[get("/{id}")]
async fn get_entry(
    pool: web::Data<PgPool>,
    path: web::Path<(i64, i64)>,
) -> Result<Json<EntryResponse>> {
    let (feed_id, id) = path.into_inner();
    let feed = get_feed_or_404(&pool, feed_id).await?;

    let entry = match feed.feed_type {
        feed::FeedType::IP => entry::IPEntry::get(&pool, &feed, id)
            .await
            .map(EntryResponse::IP),
        feed::FeedType::Domain => entry::DomainEntry::get(&pool, &feed, id)
            .await
            .map(EntryResponse::Domain),
        feed::FeedType::URL => entry::URLEntry::get(&pool, &feed, id)
            .await
            .map(EntryResponse::URL),
    }
    .map_err(|err| not_found_or_db_error("entry", id, err))?;

    Ok(Json(entry))
}

#[post("/")]
async fn create_entry(
    pool: web::Data<PgPool>,
    feed_id: web::Path<i64>,
    info: web::Json<CreateEntryData>,
) -> Result<Json<EntryResponse>> {
    let feed = get_feed_or_404(&pool, *feed_id).await?;
    let info = info.into_inner();

    let entry = match feed.feed_type {
        feed::FeedType::IP => {
            let value = info
                .value
                .parse()
                .map_err(|_| json_error(StatusCode::BAD_REQUEST, "invalid IP/CIDR value"))?;
            entry::IPEntry::insert(&pool, &feed, value, info.description, info.valid_until)
                .await
                .map(EntryResponse::IP)
        }
        feed::FeedType::Domain => {
            entry::DomainEntry::insert(&pool, &feed, info.value, info.description, info.valid_until)
                .await
                .map(EntryResponse::Domain)
        }
        feed::FeedType::URL => {
            entry::URLEntry::insert(&pool, &feed, info.value, info.description, info.valid_until)
                .await
                .map(EntryResponse::URL)
        }
    }
    .map_err(|err| db_error("new entry in feed", feed.id, err))?;

    Ok(Json(entry))
}

#[put("/{id}")]
async fn update_entry(
    pool: web::Data<PgPool>,
    path: web::Path<(i64, i64)>,
    info: web::Json<UpdateEntryData>,
) -> Result<Json<EntryResponse>> {
    let (feed_id, id) = path.into_inner();
    let feed = get_feed_or_404(&pool, feed_id).await?;
    let info = info.into_inner();

    let entry = match feed.feed_type {
        feed::FeedType::IP => entry::IPEntry::update(
            &pool,
            &feed,
            id,
            info.enabled,
            info.description,
            info.valid_until,
        )
        .await
        .map(EntryResponse::IP),
        feed::FeedType::Domain => entry::DomainEntry::update(
            &pool,
            &feed,
            id,
            info.enabled,
            info.description,
            info.valid_until,
        )
        .await
        .map(EntryResponse::Domain),
        feed::FeedType::URL => entry::URLEntry::update(
            &pool,
            &feed,
            id,
            info.enabled,
            info.description,
            info.valid_until,
        )
        .await
        .map(EntryResponse::URL),
    }
    .map_err(|err| not_found_or_db_error("entry", id, err))?;

    Ok(Json(entry))
}

#[delete("/{id}")]
async fn delete_entry(
    pool: web::Data<PgPool>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse> {
    let (feed_id, id) = path.into_inner();
    let feed = get_feed_or_404(&pool, feed_id).await?;

    let deleted = match feed.feed_type {
        feed::FeedType::IP => entry::IPEntry::delete(&pool, &feed, id).await,
        feed::FeedType::Domain => entry::DomainEntry::delete(&pool, &feed, id).await,
        feed::FeedType::URL => entry::URLEntry::delete(&pool, &feed, id).await,
    }
    .map_err(|err| db_error("entry", id, err))?;

    if deleted {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(json_error(StatusCode::NOT_FOUND, "entry not found"))
    }
}

// PRIVATE FUNCTIONS

async fn get_feed_or_404(pool: &PgPool, feed_id: i64) -> Result<feed::Feed> {
    feed::Feed::get_by_id(pool, feed_id)
        .await
        .map_err(|err| not_found_or_db_error("feed", feed_id, err))
}
