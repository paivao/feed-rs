use actix_web::{
    HttpResponse, Result, delete, error, get,
    http::header::ContentType,
    post, put,
    web::{self, Bytes, Json},
};
use async_stream::stream;
use futures::{
    TryStreamExt,
    stream::{BoxStream, Stream, StreamExt},
};
use log;
use md5::{Digest, Md5};
use sqlx::{self, postgres::PgPool};
use std::fmt::Display;

use crate::model::{
    entry,
    feed::{self, Feed, InsertFeedData},
};

#[get("/feed/{name}")]
pub async fn serve_feed(pool: web::Data<PgPool>, name: web::Path<String>) -> Result<HttpResponse> {
    let pool_arc = pool.into_inner();
    let mut feed = feed::Feed::get(&pool_arc, &name).await.map_err(|err| {
        // TODO: logging
        if let sqlx::Error::RowNotFound = err {
            log::trace!(target: &format!("{}::app", crate::APP_NAME), "Error! Feed not found: {}.", &name);
            error::ErrorNotFound("feed not found")
        } else {
            log::warn!(target: &format!("{}::app", crate::APP_NAME), "Database error: {:?}", err);
            error::ErrorBadRequest("error in request")
        }
    })?;

    let mut data_stream = match feed.feed_type {
        feed::FeedType::IP => as_lines_bytes(entry::IPEntry::fetch_values(
            pool_arc.clone().as_ref(),
            &feed,
        )),
        feed::FeedType::URL => as_lines_bytes(entry::URLEntry::fetch_values(
            pool_arc.clone().as_ref(),
            &feed,
        )),
        feed::FeedType::Domain => as_lines_bytes(entry::DomainEntry::fetch_values(
            pool_arc.clone().as_ref(),
            &feed,
        )),
    };

    log::trace!(target: &format!("{}::app", crate::APP_NAME), "Fetch feed: {}", &name);

    // If MD5 is already there, doesn't need to recalculate
    if !feed.digest.is_empty() {
        return Ok(HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .streaming(data_stream));
    }

    let mut md5_ctx = md5::Md5::new();

    let data_stream = stream! {
        let this_pool = pool.clone();
        while let Some(item) = data_stream.next().await {
            match item {
                Ok(i) => {
                    md5_ctx.update(i);
                    yield Ok(i);
                },
                Err(e) => {
                    yield Err(e);
                    return;
                }
            }
        }
        // Update digest
        let digest = md5_ctx.finalize();
        feed.digest = digest.to_vec();
        feed.update_digest(&this_pool).await;
    };

    return Ok(HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .streaming(data_stream));
}

pub fn configure_feed_api(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/feed")
            .service(list_feeds)
            .service(get_feed)
            .service(create_feed)
            .service(update_feed)
            .service(delete_feed),
    );
}

#[get("/")]
async fn list_feeds(
    pool: web::Data<PgPool>,
    window: web::Query<Option<crate::model::Window>>,
) -> Result<Json<Vec<Feed>>> {
    let feeds = feed::Feed::list(&**pool, window.into_inner())
        .await
        .map_err(|err| {
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
async fn create_feed(
    pool: web::Data<PgPool>,
    info: web::Json<InsertFeedData>,
) -> Result<Json<Feed>> {
    let feed = feed::Feed::insert(&**pool, info.into_inner())
        .await
        .map_err(|err| {
            log::warn!(target: &format!("{}::app", crate::APP_NAME), "Database error: {:?}", err);
            error::ErrorBadRequest("error in request")
        })?;
    Ok(web::Json(feed))
}

#[put("/{id}")]
async fn update_feed(
    pool: web::Data<PgPool>,
    info: web::Json<Feed>,
    id: web::Path<i64>,
) -> Result<HttpResponse> {
    todo!()
}

#[delete("/{id}")]
async fn delete_feed(pool: web::Data<PgPool>) -> Result<Json<Feed>> {
    todo!()
}

// PRIVATE FUNCTIONS

fn as_lines_bytes<'a, S, T>(mut stream: S) -> BoxStream<'a, Result<Bytes, actix_web::Error>>
where
    S: Stream<Item = Result<T, sqlx::Error>> + Unpin + Send + 'a,
    T: Display + Send + Sized,
{
    return stream
        .map(|value| match value {
            Ok(val) => Ok(format!("{val}\n").into()),
            Err(e) => {
                log::warn!("Database error: {e}");
                Err(error::ErrorInternalServerError("database error"))
            }
        })
        .boxed();
}
