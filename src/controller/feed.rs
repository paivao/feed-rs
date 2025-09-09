use actix_web::{error, get, web, FromRequest, HttpResponse, Result};
use futures::StreamExt;
use sqlx::postgres::PgPool;
use md5::{Md5, Digest};

use crate::model::{feed, entry2};

#[get("/feed/{name}")]
async fn serve_feed(pool: web::Data<PgPool>, name: web::Path<String>) -> Result<HttpResponse> {
    let mut feed = feed::Feed::get(&**pool, name.as_str(), None).await.map_err(|err| {
        // TODO: logging
        if err == sqlx::Error::RowNotFound {
            error::ErrorNotFound("feed not found")
        } else {
            error::ErrorBadRequest("error in request")
        }
    })?;

    let mut feed_hash = Md5::new();
    let mut feed_hash_is_valid = true;
    let map_stream = |elem| {
        web::Bytes::copy_from_slice(elem.to_string())
    };
    let entries = match feed.feed_type {
        feed::FeedType::IP => entry2::IPEntry::fetch_values(&**pool, &feed).map(map_stream),
        feed::FeedType::URL => entry2::URLEntry::fetch_values(&**pool, &feed).map(map_stream),
        feed::FeedType::Domain => entry2::DomainEntry::fetch_values(&**pool, &feed).map(map_stream),
    };


    Ok(HttpResponse::Ok().content_type("plain/text").streaming(entries))
}
