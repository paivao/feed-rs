use actix_web::{error, get, web, HttpResponse, Result};
use futures::stream::{StreamExt, Stream};
use std::fmt::Display;
use sqlx::postgres::PgPool;
use md5::{Digest, Md5};

use crate::model::{feed, entry};

#[get("/feed/{name}")]
pub async fn serve_feed(pool: web::Data<PgPool>, name: web::Path<String>) -> Result<HttpResponse> {
    let mut feed = feed::Feed::get(&**pool, name.as_str(), None).await.map_err(|err| {
        // TODO: logging
        if let sqlx::Error::RowNotFound = err {
            error::ErrorNotFound("feed not found")
        } else {
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

    Ok(HttpResponse::Ok().content_type("plain/text").body(entries))
}

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
