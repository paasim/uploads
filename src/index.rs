use crate::file::FileInfo;
use askama::Template;
use axum::extract::State;
use axum::http::status::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use sqlx::SqlitePool;
use tracing::Level;

pub struct HtmlTemplate<T>(pub T);

impl<T: askama::Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(s) => Html(s).into_response(),
            Err(e) => {
                tracing::event!(Level::ERROR, "Rendering error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct RootPage {
    files: Vec<FileInfo>,
}

pub async fn index(State(pool): State<SqlitePool>) -> HtmlTemplate<RootPage> {
    let files = match FileInfo::get_all(&pool).await {
        Ok(files) => files,
        Err(e) => {
            tracing::event!(Level::ERROR, "Cant read files from db: {e}");
            vec![]
        }
    };
    HtmlTemplate(RootPage { files })
}
