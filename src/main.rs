use axum::response::Html;
use axum::routing::get;
use axum::{
    Router,
    body::Body,
    extract::Query as AxQuery,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use std::collections::HashMap;
use tokio::{fs::File, process::Command};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

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
            <head>
                <title>Рилзокачалка</title>
                <style>
                    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;700&display=swap');
                    body { font-family: 'Inter', sans-serif; text-align:center; padding:100px; background:#fafafa; margin:0; }
                    .card { max-width:500px; margin:0 auto; background:white; padding:40px; border-radius:15px; box-shadow:0 4px 15px rgba(0,0,0,0.1); }
                    h1 { color:#333; margin-bottom: 10px; }
                    p.subtitle { color:#666; margin-bottom: 25px; }
                    input { width:100%; padding:12px; margin-bottom:20px; border:1px solid #ddd; border-radius:8px; box-sizing: border-box; }
                    button { width:100%; padding:12px; background:#007bff; color:white; border:none; border-radius:8px; cursor:pointer; font-weight:bold; transition: background 0.2s; }
                    button:hover { background: #0056b3; }

                    /* Стили для подписи FRKN */
                    .brand-footer {
                        margin-top: 25px;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        gap: 6px;
                        color: #999;
                        font-size: 12px;
                        font-family: 'Monaco', 'Consolas', monospace;
                        letter-spacing: 1px;
                    }
                    .brand-logo {
                        width: 16px;
                        height: 16px;
                        fill: #007bff;
                    }
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>Рилзокачалка</h1>
                    <p class="subtitle">Вставь ссылку на рилзик</p>

                    <input type="text" id="url" placeholder="https://www.instagram.com/reel/..." autofocus>

                    <button onclick="go()">Скачать Video</button>

                    <div class="brand-footer">
                        <span>by</span>
                        <svg class="brand-logo" viewBox="0 0 24 24">
                            <path d="M13 10V3L4 14h7v7l9-11h-7z"/> </svg>
                        <strong><a href="https://frkn.org">frkn</a></strong>
                    </div>
                </div>

                <script>
                    function go() {
                        const val = document.getElementById('url').value;
                        if(val) {
                            const btn = document.querySelector('button');
                            btn.innerText = 'Качаю...';
                            btn.style.opacity = '0.7';
                            window.location.href='/download?url='+encodeURIComponent(val.trim());
                        }
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

    // 👇 уникальный файл
    let id = Uuid::new_v4();
    let file_path = format!("/tmp/reel-{}.mp4", id);

    // 👇 скачиваем
    let status = match Command::new("yt-dlp")
        .arg("-f")
        .arg("mp4")
        .arg("--no-part")
        .arg("--quiet")
        .arg("--no-warnings")
        .arg("-o")
        .arg(&file_path)
        .arg(url)
        .status()
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to start yt-dlp: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to start downloader",
            )
                .into_response();
        }
    };

    if !status.success() {
        eprintln!("❌ yt-dlp failed");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Download failed").into_response();
    }

    // 👇 открываем файл
    let file = match File::open(&file_path).await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("❌ Failed to open file: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    // 👇 удаляем файл в фоне после отдачи
    let path_clone = file_path.clone();
    tokio::spawn(async move {
        // даём время на отдачу
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let _ = tokio::fs::remove_file(path_clone).await;
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp4")
        .header("Content-Disposition", "attachment; filename=\"reels.mp4\"")
        .body(body)
        .unwrap()
        .into_response()
}
