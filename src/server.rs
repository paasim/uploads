use axum::extract::{DefaultBodyLimit, Request};
use axum::routing;
use axum::{Router, response::Response};
use sqlx::{Pool, Sqlite};
use std::{io, time};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{Level, Span};

use crate::config::Config;
use crate::file::{delete_file, get_file, upload_file};
use crate::index::index;

#[tokio::main]
pub async fn run(config: Config) -> io::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::INFO)
        .init();
    let trace = TraceLayer::new_for_http()
        .make_span_with(default_span)
        .on_response(log_status);

    let listener = config.listener().await?;
    let pool = config.connection_pool().await.map_err(io::Error::other)?;
    let app = mk_router(pool, config.max_upload_size()).await.layer(trace);

    tracing::info!("serving on {}", listener.local_addr()?);
    axum::serve(listener, app).await
}

async fn mk_router(pool: Pool<Sqlite>, max_upload_size: usize) -> Router {
    Router::new()
        .route("/", routing::get(index))
        .route("/file", routing::post(upload_file))
        .route("/file/{file_id}", routing::get(get_file))
        .route("/file/{file_id}/delete", routing::post(delete_file))
        .with_state(pool)
        .fallback_service(ServeDir::new("assets"))
        .layer(DefaultBodyLimit::max(max_upload_size))
}

fn default_span(request: &Request) -> Span {
    tracing::info_span!("request", "{} {}", request.method(), request.uri())
}

fn log_status(response: &Response, latency: time::Duration, _span: &Span) {
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        tracing::event!(Level::ERROR, %status, ?latency)
    } else {
        tracing::event!(Level::INFO, %status, ?latency)
    }
}

#[cfg(test)]
mod tests {
    use crate::file::{FileData, FileInfo};

    use super::*;
    use askama::Template;
    use axum::body::{Body, to_bytes};
    use axum::extract::State;
    use axum::http::{Request, header::CONTENT_TYPE};
    use std::io::Write;
    use tower::{Service, ServiceExt};

    fn form_data(file_name: &str, contents: &[u8], boundary: &str) -> io::Result<Vec<u8>> {
        let mut data: Vec<u8> = Vec::new();

        write!(data, "--{}\r\n", boundary)?;
        write!(data, "Content-Disposition: form-data;")?;
        write!(data, "name=\"file\";filename=\"{file_name}\";\r\n")?;
        write!(data, "\r\n")?;
        data.write_all(contents)?;
        write!(data, "\r\n")?;
        write!(data, "--{}--\r\n", boundary)?;
        Ok(data)
    }

    fn upload_query(file_name: &str, contents: &[u8]) -> Request<Body> {
        let boundary = "ABCDEFGHIJKLMNOPQRSTUVXYZ";
        let r = Request::post("/file").header(
            CONTENT_TYPE,
            format!("multipart/form-data;boundary=\"{}\"", boundary),
        );
        let body = form_data(file_name, contents, boundary).unwrap();
        r.body(Body::from(body)).unwrap()
    }

    fn delete_query(file_id: i64) -> Request<Body> {
        Request::post(format!("/file/{file_id}/delete"))
            .body(Body::from(()))
            .unwrap()
    }

    fn index_query() -> Request<Body> {
        Request::get("/").body(Body::from(())).unwrap()
    }

    async fn send(app: &mut Router, req: Request<Body>) -> Response<Body> {
        ServiceExt::<Request<Body>>::ready(app)
            .await
            .unwrap()
            .call(req)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn inserting_and_deleting_files_works() {
        let config = Config::test_config(10);
        let pool = config.connection_pool().await.unwrap();
        let mut app = mk_router(pool.clone(), config.max_upload_size()).await;

        let name = "42.tmp";
        let contents: Vec<_> = (0..5_127_123).map(|i| (i % 255) as u8).collect();
        send(&mut app, upload_query(name, &contents)).await;

        let infos = FileInfo::get_all(&pool).await.unwrap();
        assert_eq!(infos.len(), 1);

        let info = infos.first().unwrap();
        assert_eq!(info.name(), name);
        assert_eq!(info.size_formatted(), "5.1 MB");

        let data = FileData::get(&pool, info.id()).await.unwrap().unwrap();
        assert_eq!(data.name(), name);
        assert_eq!(data.data(), contents);

        // adding another one works
        let contents = vec![42u8, 13u8];
        send(&mut app, upload_query(name, &contents)).await;
        let infos = FileInfo::get_all(&pool).await.unwrap();
        assert_eq!(infos.len(), 2);

        // delete works
        send(&mut app, delete_query(info.id())).await;
        // the given id is no longer included
        let infos = FileInfo::get_all(&pool).await.unwrap();
        assert_eq!(infos.len(), 1);
        let remaining_id = infos.first().unwrap().id();
        assert_ne!(remaining_id, info.id());

        // deleting another works
        send(&mut app, delete_query(remaining_id)).await;
        let infos = FileInfo::get_all(&pool).await.unwrap();
        assert_eq!(infos.len(), 0);
    }

    #[tokio::test]
    async fn index_page_works() {
        let config = Config::test_config(10);
        let pool = config.connection_pool().await.unwrap();
        let mut app = mk_router(pool.clone(), config.max_upload_size()).await;

        let index_exp = index(State(pool.clone())).await.0.render().unwrap();

        let contents = send(&mut app, index_query()).await;
        let b = to_bytes(contents.into_body(), usize::MAX).await.unwrap();
        let index_resp = String::from_utf8(b.to_vec()).unwrap();

        assert_eq!(index_exp, index_resp);

        let name = "movie.mp4";
        let contents: Vec<_> = (0..2_700).map(|i| (i % 255) as u8).collect();
        send(&mut app, upload_query(name, &contents)).await;

        let index_exp_new = index(State(pool)).await.0.render().unwrap();
        assert_ne!(index_exp_new, index_exp);

        let contents = send(&mut app, index_query()).await;
        let b = to_bytes(contents.into_body(), usize::MAX).await.unwrap();
        let index_resp = String::from_utf8(b.to_vec()).unwrap();

        assert_eq!(index_exp_new, index_resp);
    }
}
