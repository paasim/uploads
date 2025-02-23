use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Redirect, Response};
use axum::{extract::Multipart, response::IntoResponse};
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{SqlitePool, query, query_as};
use tracing::Level;

#[derive(Debug)]
pub struct FileInfo {
    id: i64,
    name: String,
    data_len: Option<i64>,
    modified: Option<String>,
}

impl FileInfo {
    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn modified(&self) -> &str {
        self.modified.as_ref().map_or("", |s| s)
    }

    pub fn size_formatted(&self) -> String {
        let Some(len) = self.data_len else {
            return String::new();
        };
        let giga = 1_000_000_000;
        let mega = 1_000_000;
        let kilo = 1_000;
        let (div, unit) = match len {
            n if n > giga => (giga, "GB"),
            n if n > mega => (mega, "MB"),
            n if n > kilo => (kilo, "kB"),
            _ => (1, "B"),
        };
        let size = (len as f64 / div as f64 * 10.0).round() / 10.0;
        format!("{} {}", size, unit)
    }

    pub async fn get_all(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        let rows = query_as!(
            Self,
            r#"SELECT
                id,
                name,
                -- somehow sqlx does not understand the length-function without + 0
                length(data) + 0 AS data_len,
                datetime(modified, 'unixepoch') AS modified
            FROM file"#
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Debug)]
pub struct FileData {
    name: String,
    content_type: Option<String>,
    data: Vec<u8>,
}

impl FileData {
    #[allow(dead_code)] // testing
    pub fn name(&self) -> &str {
        &self.name
    }

    #[allow(dead_code)] // testing
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    async fn insert(&self, pool: &SqlitePool) -> sqlx::Result<SqliteQueryResult> {
        let q = query!(
            r#"INSERT INTO file (name, content_type, data) VALUES (?, ?, ?)"#,
            self.name,
            self.content_type,
            self.data
        );
        q.execute(pool).await
    }

    pub async fn get(pool: &SqlitePool, id: i64) -> sqlx::Result<Option<Self>> {
        query_as!(
            Self,
            r#"SELECT name, content_type, data FROM file WHERE id = ?"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    fn into_download(self) -> Response {
        let content_disp = (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename={}", self.name),
        );
        match self.content_type {
            Some(content_type) => (
                [(header::CONTENT_TYPE, content_type), content_disp],
                self.data,
            )
                .into_response(),
            None => ([content_disp], self.data).into_response(),
        }
    }
}

pub async fn upload_file(State(pool): State<SqlitePool>, mut mp: Multipart) -> Response {
    while let Some(field) = mp.next_field().await.transpose() {
        let field = match field {
            Ok(f) => f,
            Err(e) => return (e.status(), e.body_text()).into_response(),
        };
        if field.name().unwrap_or("") != "file" {
            tracing::event!(Level::INFO, "Form field name not equal to 'file', skipping");
            continue;
        };
        let Some(file_name) = field.file_name().map(|s| s.to_string()) else {
            tracing::event!(Level::INFO, "Got file name missing, skipping");
            continue;
        };
        let content_type = field.content_type().map(|s| s.to_string());
        let file = match field.bytes().await {
            Ok(data) => FileData {
                name: file_name,
                content_type,
                data: data.to_vec(),
            },
            Err(e) => return (e.status(), e.body_text()).into_response(),
        };
        if let Err(e) = file.insert(&pool).await {
            let s = format!("Inserting `{}` to db failed", file.name);
            tracing::event!(Level::ERROR, s, " -- {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, s).into_response();
        }
    }
    Redirect::to("/").into_response()
}

pub async fn delete_file(State(pool): State<SqlitePool>, Path(file_id): Path<i64>) -> Response {
    let q = query!(r#"DELETE FROM file WHERE id = ?"#, file_id);
    match q.execute(&pool).await {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => {
            let s = format!("Deleting `{}` from db failed", file_id);
            tracing::event!(Level::ERROR, s, " -- {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, s).into_response()
        }
    }
}

pub async fn get_file(State(pool): State<SqlitePool>, Path(file_id): Path<i64>) -> Response {
    match FileData::get(&pool, file_id).await {
        Ok(Some(f)) => f.into_download(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            let s = format!("Querying for `{}` from db failed", file_id);
            tracing::event!(Level::ERROR, s, " -- {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, s).into_response()
        }
    }
}
