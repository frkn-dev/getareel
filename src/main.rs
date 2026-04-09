use axum::{
    Router,
    body::Body,
    extract::Query as AxQuery,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use http::StatusCode;
use std::collections::HashMap;
use tokio_util::io::ReaderStream;

use tokio::process::Command;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/download", get(download));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("\n🚀 FRKN Rust Downloader запущен!");
    println!("🌍 Адрес: http://127.0.0.1:3000\n");

    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
        <html>
            <head><title>Рилзокачалка</title></head>
            <body style="font-family:sans-serif; text-align:center; padding:100px; background:#fafafa;">
                <div style="max-width:500px; margin:0 auto; background:white; padding:40px; border-radius:15px; box-shadow:0 4px 15px rgba(0,0,0,0.1);">
                    <h1 style="color:#333;">Рилзокачалка</h1>
                    <p style="color:#666;">Вставь ссылку на рилзик</p>
                    <input type="text" id="url" placeholder="https://www.instagram.com/reel/..."
                        style="width:100%; padding:12px; margin-bottom:20px; border:1px solid #ddd; border-radius:8px;">
                    <button onclick="go()"
                        style="width:100%; padding:12px; background:#007bff; color:white; border:none; border-radius:8px; cursor:pointer; font-weight:bold;">
                        Скачать Video
                    </button>
                    <p> by FRKN </p>
                </div>
                <script>
                    function go() {
                        const val = document.getElementById('url').value;
                        if(val) window.location.href='/download?url='+encodeURIComponent(val.trim());
                    }
                </script>
            </body>
        </html>
    "#,
    )
}

async fn download(AxQuery(params): AxQuery<HashMap<String, String>>) -> impl IntoResponse {
    let url = match params.get("url") {
        Some(u) if u.contains("instagram.com") => u,
        _ => return (StatusCode::BAD_REQUEST, "Invalid or missing URL").into_response(),
    };

    println!("📥 Downloading: {}", url);

    let mut child = match Command::new("yt-dlp")
        .arg("-f")
        .arg("mp4")
        .arg("-o")
        .arg("-")
        .arg(url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to start yt-dlp: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to start downloader",
            )
                .into_response();
        }
    };

    let stdout = child.stdout.take().unwrap();
    let stream = ReaderStream::new(stdout);

    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp4")
        .header("Content-Disposition", "attachment; filename=\"reels.mp4\"")
        .body(body)
        .unwrap()
        .into_response()
}
