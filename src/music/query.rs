use std::process::{Command, Stdio};

use serde::de::Error;
use songbird::input::Input;

type QueryResult<T> = Result<T, QueryError>;

#[derive(Debug)]
pub enum QueryError {
    NotFound,
}

pub async fn query_video(uri: String) -> QueryResult<Input> {
    if !uri.starts_with("http") {
        let source = songbird::ytdl(format!("ytsearch1:{}", uri)).await;

        if source.is_err() {
            return Err(QueryError::NotFound);
        }

        return Ok(source.unwrap());
    }

    let source = songbird::ytdl(uri).await;

    if let Err(_) = source {
        return Err(QueryError::NotFound);
    }

    return Ok(source.unwrap());
}
