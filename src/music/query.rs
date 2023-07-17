use std::process::{Command, Stdio};

use serde::de::Error;
use songbird::input::Input;

type QueryResult<T> = Result<T, QueryError>;

#[derive(Debug)]
pub enum QueryError {
    NotFound,
}

pub async fn query_video(uri: String) -> QueryResult<Input> {
    let uri = if !uri.starts_with("http") {
        format!("ytsearch1:{}", uri)
    } else {
        uri
    };

    let source = songbird::ytdl(uri).await;

    if let Err(_) = source {
        return Err(QueryError::NotFound);
    }

    return Ok(source.unwrap());
}
