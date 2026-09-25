use actix_web::web::Bytes;
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use serde::Serialize;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::{Error, FromRow, PgPool};

use crate::model::feed::Feed;

macro_rules! make_entry_type {
    ($name: ident, $field_type: ty, $table_name: literal) => {
        #[derive(Debug, Clone, FromRow, Serialize)]
        pub struct $name {
            pub id: i64,
            pub value: $field_type,
            pub enabled: bool,
            pub description: String,
            pub valid_until: Option<DateTime<Utc>>,
            //feed_id: i64,
        }

        impl $name {
            const INSERT_QUERY: &'static str = concat!("INSERT INTO ", $table_name, " (value, enabled, feed_id, description, valid_until) VALUES ($1, $2, $3, $4, $5) RETURNING id;");
            const FETCH_QUERY: &'static str = concat!("SELECT value from ", $table_name, " WHERE feed_id = $1 AND enabled = TRUE AND (valid_until IS NULL OR valid_until >= NOW())");
            const GET_QUERY: &'static str = concat!("SELECT id, value, enabled, description, valid_until FROM ", $table_name, " WHERE id = $1 AND feed_id = $2");
            const GET_SOME_QUERY: &'static str = concat!("SELECT id, value, enabled, description, valid_until FROM ", $table_name, " WHERE feed_id = ");
            const UPDATE_QUERY: &'static str = concat!("UPDATE ", $table_name, " SET enabled = $1, description = $2, valid_until = $3 WHERE id = $4 AND feed_id = $5;");
            const DELETE_QUERY: &'static str = concat!("DELETE FROM ", $table_name, " WHERE id = $1 AND feed_id = $2;");

            pub async fn insert(conn: &PgPool, feed: &Feed, value: $field_type, description: Option<String>, valid_until: Option<DateTime<Utc>>) -> Result<Self, Error> {
                let descr = description.unwrap_or(String::new());
                let id:i64 = sqlx::query_scalar(Self::INSERT_QUERY)
                    .bind(&value)
                    .bind(true)
                    .bind(feed.id)
                    .bind(&descr)
                    .bind(&valid_until)
                    .fetch_one(conn).await?;
                Ok(Self{id: id, value: value, enabled: true, description: descr, valid_until: valid_until/*, feed_id: feed.id*/})
            }

            pub fn fetch_values<'q>(conn: &'q PgPool, feed: &Feed) -> BoxStream<'q, Result<Bytes, Error>>
            {
                sqlx::query_scalar(Self::FETCH_QUERY).bind(feed.id).fetch(conn).map_ok(|value: $field_type| format!("{value}\n").into()).boxed()
            }

            /// Fetch a single entry by id, scoped to the given feed.
            pub async fn get(conn: &PgPool, feed: &Feed, id: i64) -> Result<Self, Error> {
                sqlx::query_as(Self::GET_QUERY).bind(id).bind(feed.id).fetch_one(conn).await
            }

            pub async fn fetch_some(conn: &PgPool, feed: &Feed, quantity: i64, last_id: Option<i64>, enabled: Option<bool>, valid_until: Option<Option<DateTime<Utc>>>) -> Result<Vec<Self>, Error> {
                let mut builder = sqlx::QueryBuilder::new(Self::GET_SOME_QUERY);
                builder.push_bind(feed.id);
                if let Some(enabled_cond) = enabled {
                    builder.push(" AND enabled = ").push_bind(enabled_cond);
                }
                if let Some(valid_conf) = valid_until {
                    builder.push(" AND valid_until");
                    match valid_conf {
                        Some(times) => builder.push(" >= ").push_bind(times),
                        None => builder.push(" IS NULL")
                    };
                }
                builder.push(" AND id > ").push_bind(last_id.unwrap_or(0));
                builder.push(" LIMIT ").push_bind(quantity);
                builder.build_query_as().fetch_all(conn).await
            }

            /// Update an entry's mutable fields (value is immutable), scoped to the given feed,
            /// returning the row as it now stands.
            pub async fn update(conn: &PgPool, feed: &Feed, id: i64, enabled: bool, description: Option<String>, valid_until: Option<DateTime<Utc>>) -> Result<Self, Error> {
                let descr = description.unwrap_or(String::new());
                sqlx::query(Self::UPDATE_QUERY)
                    .bind(enabled)
                    .bind(&descr)
                    .bind(&valid_until)
                    .bind(id)
                    .bind(feed.id)
                    .execute(conn).await?;
                Self::get(conn, feed, id).await
            }

            /// Delete an entry by id, scoped to the given feed. Returns whether a row was deleted.
            pub async fn delete(conn: &PgPool, feed: &Feed, id: i64) -> Result<bool, Error> {
                let result = sqlx::query(Self::DELETE_QUERY)
                    .bind(id)
                    .bind(feed.id)
                    .execute(conn).await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }
}

make_entry_type!(IPEntry, IpNetwork, "ip_entries");
make_entry_type!(URLEntry, String, "url_entries");
make_entry_type!(DomainEntry, String, "domain_entries");

/*
pub trait TEntry where
    for <'r> Self: Sized + AsRef<BaseEntry> + AsMut<BaseEntry> + Unpin + FromRow<'r, PgRow> + Send,
    for <'q> Self::ValueType: Encode<'q, Postgres> + Type<Postgres> + Sync + Unpin + Decode<'q, Postgres> + Send
{
    type ValueType;
    const INSERT_QUERY: &'static str = ;
    const FETCH_QUERY: &'static str = "SELECT value from {} WHERE feed_id = $1 AND enabled = TRUE AND (valid_until IS NULL OR valid_until >= NOW())";
    const GET_SOME_QUERY: &'static str = "SELECT (id, value, enabled, description, valid_until) from {} WHERE feed_id = ";

    fn mut_value(&mut self) -> &mut Self::ValueType;
    fn value(&self) -> &Self::ValueType;
    fn new(value: Self::ValueType, inner: BaseEntry) -> Self;



    fn fetch_values<'q>(conn: &'q PgPool, feed: &Feed) -> BoxStream<'q, Result<Self::ValueType, Error>> where Self::ValueType: 'q
    {
        sqlx::quer
        sqlx::query_scalar(Self::FETCH_QUERY).bind(feed.id).fetch(conn)
    }

    async fn fetch_some(conn: &PgPool, feed: &Feed, quantity: i64, last_id: Option<i64>, enabled: Option<bool>, valid_until: Option<Option<DateTime<Utc>>>) -> Result<Vec<Self>, Error> {
        let mut builder = sqlx::QueryBuilder::new(Self::GET_SOME_QUERY);
        builder.push_bind(feed.id);
        if let Some(enabled_cond) = enabled {
            builder.push(" AND enabled = ").push_bind(enabled_cond);
        }
        if let Some(valid_conf) = valid_until {
            builder.push(" AND valid_until");
            match valid_conf {
                Some(times) => builder.push(" >= ").push_bind(times),
                None => builder.push(" IS NULL")
            };
        }
        builder.push(" AND id > ").push_bind(last_id.unwrap_or(0));
        builder.push(" LIMIT ").push_bind(quantity);
        builder.build_query_as().fetch_all(conn).await
    }

    async fn update(&self, conn: &PgPool) -> Result<(), Error> {
        sqlx::query("UPDATE")
    }
}

impl AsRef<BaseEntry> for IPEntry {
    fn as_ref(&self) -> &BaseEntry {
        &self.inner
    }
}

impl AsMut<BaseEntry> for IPEntry  {
    fn as_mut(&mut self) -> &mut BaseEntry {
        &mut self.inner
    }
}

impl TEntry for IPEntry {
    type ValueType = IpNetwork;

    fn mut_value(&mut self) -> &mut Self::ValueType {
        &mut self.value
    }

    fn value(&self) -> &Self::ValueType {
        &self.value
    }

    fn new(value: Self::ValueType, inner: BaseEntry) -> Self {
        Self{value: value, inner: inner}
    }
}
    */
