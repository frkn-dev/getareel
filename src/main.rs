use axum::response::Html;
use axum::routing::get;
use axum::{
    body::{Body, Bytes},
    extract::Query as AxQuery,
    response::{IntoResponse, Response},
    Router,
};
use futures_util::StreamExt;
use http::StatusCode;
use regex::Regex;
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio::process::Command;
use tokio_util::io::ReaderStream;

const VERSION: &str = "0.1.3";

// YouTube: прогрессивный mp4 до 720p — без мержа ffmpeg, начинает отдаваться сразу.
const YT_FORMAT: &str = "best[ext=mp4][height<=720]/best[height<=720]/best[ext=mp4]/best";
// Остальные сервисы: готовый mp4 любого качества, тоже стримится сразу.
const OTHER_FORMAT: &str = "best[ext=mp4]/best";

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/download", get(download));

    let bind_addr = std::env::var("GETAREEL_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    println!("\n🚀 FRKN Рилзокачка запущена!");
    println!("Version: {}", VERSION);

    match Command::new("/usr/local/bin/yt-dlp")
        .arg("--version")
        .output()
        .await
    {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("yt-dlp version: {}", ver);
        }
        _ => println!("yt-dlp version: unknown"),
    }

    println!("🌍 Адрес: http://{}\n", bind_addr);

    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(
        r#"
    <html>
        <head>
            <title>Рилзокачка</title>
            <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
            <style>
                @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;700&display=swap');

                body {
                    font-family: 'Inter', sans-serif;
                    text-align: center;
                    background: #fafafa;
                    margin: 0;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    min-height: 100vh; /* Центрируем по вертикали */
                    padding: 20px; /* Чтобы на мобилках не касалось краев */
                    box-sizing: border-box;
                }

                .card {
                    width: 100%;
                    max-width: 450px;
                    background: white;
                    padding: 30px;
                    border-radius: 20px;
                    box-shadow: 0 10px 25px rgba(0,0,0,0.05);
                }

                h1 { color:#333; margin-top: 0; font-size: 24px; }
                p.subtitle { color:#666; margin-bottom: 25px; font-size: 16px; }

                input {
                    width: 100%;
                    padding: 15px;
                    margin-bottom: 20px;
                    border: 1px solid #eee;
                    border-radius: 12px;
                    font-size: 16px; /* Важно: 16px предотвращает зум на iPhone при клике */
                    box-sizing: border-box;
                    background: #fdfdfd;
                    -webkit-appearance: none; /* Убираем стандартные тени iOS */
                }

                button {
                    width: 100%;
                    padding: 15px;
                    background: #007bff;
                    color: white;
                    border: none;
                    border-radius: 12px;
                    cursor: pointer;
                    font-weight: bold;
                    font-size: 16px;
                    transition: all 0.2s;
                    -webkit-tap-highlight-color: transparent;
                }

                button:active { transform: scale(0.98); opacity: 0.9; }

                .progress-container {
                    width: 100%;
                    height: 8px;
                    background: #eee;
                    border-radius: 4px;
                    margin-top: 15px;
                    overflow: hidden;
                    display: none;
                }

                .progress-bar {
                    height: 100%;
                    width: 0%;
                    background: #007bff;
                    transition: width 0.2s;
                }

                .progress-text {
                    margin-top: 8px;
                    font-size: 13px;
                    color: #666;
                    display: none;
                }

                .brand-footer {
                    margin-top: 30px;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    gap: 6px;
                    color: #bbb;
                    font-size: 11px;
                    font-family: 'Monaco', 'Consolas', monospace;
                    letter-spacing: 1px;
                }

                .brand-logo { width: 14px; height: 14px; fill: #007bff; }
                .brand-footer a { color: inherit; text-decoration: none; }
            </style>
        </head>
        <body>
            <div class="card">
                <h1>Рилзокачка</h1>
                <p class="subtitle">TikTok · X · YouTube · VK и ещё куча всего</p>

                <input type="text" id="url" placeholder="https://..." autofocus>

                <button onclick="go()">Скачать Video</button>

                <div class="progress-container" id="progress-container">
                    <div class="progress-bar" id="progress-bar"></div>
                </div>
                <div class="progress-text" id="progress-text"></div>

                <div class="brand-footer">
                    <span>by</span>
                    <svg class="brand-logo" viewBox="0 0 24 24">
                        <path d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    <strong><a href="https://frkn.org">frkn v0.1.3</a></strong>
                </div>
            </div>

            <script>
                function formatBytes(bytes) {
                    if (bytes === 0) return '0 B';
                    const k = 1024;
                    const sizes = ['B', 'KB', 'MB', 'GB'];
                    const i = Math.floor(Math.log(bytes) / Math.log(k));
                    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
                }

                async function go() {
                    const input = document.getElementById('url');
                    const val = input.value.trim();
                    if (!val) return;

                    const btn = document.querySelector('button');
                    const progressContainer = document.getElementById('progress-container');
                    const progressBar = document.getElementById('progress-bar');
                    const progressText = document.getElementById('progress-text');
                    const originalText = btn.innerText;

                    btn.innerText = 'Качаю...';
                    btn.disabled = true;
                    progressContainer.style.display = 'block';
                    progressText.style.display = 'block';
                    progressBar.style.width = '0%';
                    progressText.innerText = 'Подготовка...';

                    try {
                        const resp = await fetch('/download?url=' + encodeURIComponent(val));
                        if (!resp.ok) {
                            const err = await resp.text();
                            throw new Error(err || 'Ошибка ' + resp.status);
                        }

                        const total = resp.headers.get('Content-Length');
                        const reader = resp.body.getReader();
                        const chunks = [];
                        let loaded = 0;

                        while (true) {
                            const { done, value } = await reader.read();
                            if (done) break;
                            chunks.push(value);
                            loaded += value.length;

                            if (total) {
                                const percent = Math.round((loaded / total) * 100);
                                progressBar.style.width = percent + '%';
                                progressText.innerText = percent + '% (' + formatBytes(loaded) + ' / ' + formatBytes(total) + ')';
                            } else {
                                progressBar.style.width = '100%';
                                progressText.innerText = 'Скачано ' + formatBytes(loaded);
                            }
                        }

                        const blob = new Blob(chunks);
                        const url = window.URL.createObjectURL(blob);
                        const a = document.createElement('a');
                        a.href = url;
                        a.download = 'video.mp4';
                        document.body.appendChild(a);
                        a.click();
                        a.remove();
                        window.URL.revokeObjectURL(url);

                        progressText.innerText = 'Готово!';
                    } catch (err) {
                        alert('Не удалось скачать: ' + err.message);
                        progressText.innerText = 'Ошибка';
                    } finally {
                        btn.innerText = originalText;
                        btn.disabled = false;
                        setTimeout(() => {
                            progressContainer.style.display = 'none';
                            progressText.style.display = 'none';
                            progressBar.style.width = '0%';
                        }, 2000);
                    }
                }
            </script>
        </body>
    </html>
    "#,
    )
}

fn normalize_url(url: &str) -> String {
    // X / Twitter: https://x.com/user/status/123/video/1 → https://x.com/user/status/123
    let x_re =
        Regex::new(r"(?i)^(https?://(?:x|twitter)\.com/[^/]+/status/\d+)(?:/video/\d+)?").unwrap();
    if let Some(caps) = x_re.captures(url) {
        return caps.get(1).unwrap().as_str().to_string();
    }
    url.to_string()
}

fn is_youtube_url(url: &str) -> bool {
    url.contains("youtube.com") || url.contains("youtu.be") || url.contains("youtube-nocookie.com")
}

/// Запускает yt-dlp в stdout и ждёт первые байты (preflight).
/// Если видео недоступно — возвращает None, чтобы можно было повторить с куками.
/// Если всё ок — возвращает стримящий ответ, отдача начинается мгновенно.
async fn try_stream(url: &str, format: &str, cookies: Option<&str>) -> Option<Response<Body>> {
    let mut cmd = Command::new("/usr/local/bin/yt-dlp");
    cmd.arg("-f")
        .arg(format)
        .arg("--no-part")
        .arg("--no-playlist")
        .arg("--quiet")
        .arg("--no-warnings")
        .arg("--user-agent")
        .arg(USER_AGENT)
        .arg("-o")
        .arg("-")
        .arg(url);

    if let Some(c) = cookies {
        cmd.arg("--cookies").arg(c);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to start yt-dlp: {}", e);
            return None;
        }
    };

    let mut stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Preflight: ждём первые байты до 20 секунд. Нет данных — считаем попытку неудачной.
    let mut first = vec![0u8; 64 * 1024];
    let n = match tokio::time::timeout(Duration::from_secs(20), stdout.read(&mut first)).await {
        Ok(Ok(n)) if n > 0 => n,
        _ => {
            let _ = child.kill().await;
            let mut stderr_reader = tokio::io::BufReader::new(stderr);
            let mut stderr_buf = String::new();
            let _ = stderr_reader.read_to_string(&mut stderr_buf).await;
            let err = stderr_buf.trim();
            if !err.is_empty() {
                eprintln!("⚠️ yt-dlp preflight failed: {}", err);
            }
            return None;
        }
    };
    first.truncate(n);

    // Дочитываем stderr и ждём завершения процесса в фоне.
    tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            if !line.is_empty() {
                eprintln!("ℹ️ yt-dlp: {}", line);
            }
        }
        match child.wait().await {
            Ok(status) if !status.success() => eprintln!("❌ yt-dlp exited with {}", status),
            Ok(_) => {}
            Err(e) => eprintln!("❌ Failed to wait for yt-dlp: {}", e),
        }
    });

    let first = Bytes::from(first);
    let stream = futures_util::stream::once(async move { Ok::<Bytes, std::io::Error>(first) })
        .chain(ReaderStream::new(stdout).map(|r| r.map_err(|e| e)));
    let body = Body::from_stream(stream);

    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "video/mp4")
            .header("Content-Disposition", "attachment; filename=\"video.mp4\"")
            .body(body)
            .unwrap()
            .into_response(),
    )
}

async fn download(AxQuery(params): AxQuery<HashMap<String, String>>) -> impl IntoResponse {
    let url = match params.get("url") {
        Some(u) if u.starts_with("http://") || u.starts_with("https://") => normalize_url(u),
        _ => return (StatusCode::BAD_REQUEST, "Invalid or missing URL").into_response(),
    };

    let format = if is_youtube_url(&url) {
        YT_FORMAT
    } else {
        OTHER_FORMAT
    };

    // Попытка 1: без куки (публичный контент — большинство случаев).
    println!("📥 Attempt 1 (no cookies): {}", url);
    if let Some(resp) = try_stream(&url, format, None).await {
        return resp;
    }

    // Попытка 2: с куки, если файл есть.
    let cookies_path = "cookies.txt";
    if tokio::fs::metadata(cookies_path).await.is_ok() {
        println!("📥 Attempt 2 (with cookies): {}", url);
        if let Some(resp) = try_stream(&url, format, Some(cookies_path)).await {
            return resp;
        }
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Не удалось скачать. Возможно, видео приватное, удалено или требует авторизации. Попробуйте обновить cookies.txt.",
    )
        .into_response()
}
