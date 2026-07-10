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
use std::{fmt::Display, ops::Deref};

use crate::model::{
    entry,
    feed::{self, Feed, InsertFeedData},
};

#[get("/feed/{name}")]
pub async fn serve_feed(pool: web::Data<PgPool>, name: web::Path<String>) -> HttpResponse {
    let mut feed = match feed::Feed::get(pool.get_ref(), &name).await {
        Ok(feed) => feed,
        Err(err) => {
            if let sqlx::Error::RowNotFound = err {
                log::trace!(target: &format!("{}::app", crate::APP_NAME), "Error! Feed not found: {}.", &name);
                return HttpResponse::NotFound()
                    .content_type(ContentType::plaintext())
                    .body("feed not found");
            } else {
                log::warn!(target: &format!("{}::app", crate::APP_NAME), "Database error: {:?}", err);
                return HttpResponse::InternalServerError()
                    .content_type(ContentType::plaintext())
                    .body("error in request");
            }
        }
    };

    log::trace!(target: &format!("{}::app", crate::APP_NAME), "Fetch feed: {}", &name);

    let feed_name = name.clone();

    // If MD5 is already there, doesn't need to recalculate
    let must_recalculate = feed.digest.is_empty();
    let data_stream = stream! {
        let inner_pool = pool.clone();
        let mut data_stream = match feed.feed_type {
            feed::FeedType::IP => entry::IPEntry::fetch_values(&inner_pool, &feed),
            feed::FeedType::Domain => entry::DomainEntry::fetch_values(&inner_pool, &feed),
            feed::FeedType::URL => entry::URLEntry::fetch_values(&pool, &feed),
        };
        let mut md5_ctx = md5::Md5::new();
        while let Some(item) = data_stream.next().await {
            let item = match item {
                Ok(i) => i,
                Err(e) => {
                    log::warn!(target: &format!("{}::app", crate::APP_NAME), "Error fetching feed {}. Database error: {:?}", feed_name, e);
                    yield Err(error::ErrorInternalServerError("database error"));
                    return;
                },
            };
            if must_recalculate {
                md5_ctx.update(&item);
            }
            yield Ok(item);
        };
        if must_recalculate {
            feed.digest = md5_ctx.finalize().to_vec();
            if let Err(e) = feed.update_digest(&inner_pool).await {
                log::warn!(target: &format!("{}::app", crate::APP_NAME), "Error updating digest for feed {}: {:?}", &name, e);
            }
        }
    };

    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .streaming(data_stream)
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

fn as_lines_bytes<'a, S, T>(stream: S) -> BoxStream<'a, Result<Bytes, actix_web::Error>>
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
