use actix_web::{delete, error, get, http::header::ContentType, post, put, web::{self, Json}, HttpResponse, Result};
use futures::stream::{StreamExt, Stream};
use std::fmt::Display;
use sqlx::postgres::PgPool;
use md5::{Digest, Md5};
use log;

use crate::model::{entry, feed::{self, Feed, InsertFeedData}};

#[get("/feed/{name}")]
pub async fn serve_feed(pool: web::Data<PgPool>, name: web::Path<String>) -> Result<HttpResponse> {
    let mut feed = feed::Feed::get(&**pool, &name).await.map_err(|err| {
        // TODO: logging
        if let sqlx::Error::RowNotFound = err {
            log::trace!(target: &format!("{}::app", crate::APP_NAME), "Error! Feed not found: {}.", &name);
            error::ErrorNotFound("feed not found")
        } else {
            log::warn!(target: &format!("{}::app", crate::APP_NAME), "Database error: {:?}", err);
            error::ErrorBadRequest("error in request")
        }
    })?;

    let entries = match feed.feed_type {
        feed::FeedType::IP => into_string(entry::IPEntry::fetch_values(&**pool, &feed)).await?,
        feed::FeedType::URL => into_string(entry::URLEntry::fetch_values(&**pool, &feed)).await?,
        feed::FeedType::Domain => into_string(entry::DomainEntry::fetch_values(&**pool, &feed)).await?,
    };

    // If digest is empty, it means that it should be calculated
    if feed.digest.is_empty() {
        let mut feed_hash = Md5::new();
        feed_hash.update(entries.as_bytes());
        feed.digest = feed_hash.finalize().as_slice().to_vec();
        let _ = feed.update_digest(&**pool).await;
    }

    log::trace!(target: &format!("{}::app", crate::APP_NAME), "Fetch feed: {}", &name);
    Ok(HttpResponse::Ok().content_type(ContentType::plaintext()).body(entries))
}

pub fn configure_feed_api(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/feed")
        .service(list_feeds)
        .service(get_feed)
        .service(create_feed)
        .service(update_feed)
        .service(delete_feed)
    );
}

#[get("/")]
async fn list_feeds(pool: web::Data<PgPool>, window: web::Query<Option<crate::model::Window>>) -> Result<Json<Vec<Feed>>> {
    let feeds = feed::Feed::list(&**pool, window.into_inner()).await.map_err(|err| {
        log::warn!(target: &format!("{}::app", crate::APP_NAME), "Database error: {:?}", err);
        error::ErrorBadRequest("error in request")
    })?;
    Ok(web::Json(feeds))
}

#[get("/{id}")]
async fn get_feed(pool: web::Data<PgPool>, id: web::Path<i64>) -> Result<Json<Feed>> {
    let feed = feed::Feed::get_by_id(&**pool, *id).await.map_err(|err| {
        // TODO: logging
        if let sqlx::Error::RowNotFound = err {
            log::trace!(target: &format!("{}::app", crate::APP_NAME), "Error! Feed not found: {}.", id);
            error::ErrorNotFound("feed not found")
        } else {
            log::warn!(target: &format!("{}::app", crate::APP_NAME), "Database error: {:?}", err);
            error::ErrorBadRequest("error in request")
        }
    })?;
    Ok(web::Json(feed))
}

#[post("/")]
async fn create_feed(pool: web::Data<PgPool>, info: web::Json<InsertFeedData>) -> Result<Json<Feed>> {
    let feed = feed::Feed::insert(&**pool, info.into_inner()).await.map_err(|err| {
        log::warn!(target: &format!("{}::app", crate::APP_NAME), "Database error: {:?}", err);
        error::ErrorBadRequest("error in request")
    })?;
    Ok(web::Json(feed))
}

#[put("/{id}")]
async fn update_feed(pool: web::Data<PgPool>, info: web::Json<Feed> , id: web::Path<i64>) -> Result<HttpResponse> {
    todo!()
}

#[delete("/{id}")]
async fn delete_feed(pool: web::Data<PgPool>) -> Result<Json<Feed>> {
    todo!()
}

// PRIVATE FUNCTIONS

async fn into_string<S, T>(mut stream: S) -> actix_web::Result<String> where 
    S: Stream<Item = Result<T, sqlx::Error>> + Unpin,
    T: Display + Send + Sized,
    {
    let mut joined = String::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(entry) => {joined.push_str(&entry.to_string()); joined.push('\n')},
            Err(_) => return Err(error::ErrorBadRequest("unable to fetch feed")),
        }
    };
    Ok(joined)
}