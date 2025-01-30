use mjai_reviewer_service::delete_oldest_files;

use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{
    get, post, web, App, HttpRequest, HttpServer, error::ErrorInternalServerError,
};
use futures_util::stream::StreamExt as _;

use sanitize_filename::sanitize;
use std::{
    fs, io::Write, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}
};

static UPLOAD_FOLDER: &str = "uploads";
static MAX_FOLDER_SIZE_MB: u64 = 100;
static MAX_CONTENT_LENGTH: u64 = 100 * 1024; // 100KB

#[get("/")]
async fn index () -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open("static/index.html")?)
}

#[post("/upload")]
async fn upload_file(req: HttpRequest, payload: Multipart) -> actix_web::Result<NamedFile> {
    // 1. 检查请求大小
    if let Err(resp) = check_request_size(&req) {
        return Err(ErrorInternalServerError(resp));
    }

    // 2. 确保 uploads/ 文件夹存在
    if let Err(resp) = ensure_upload_folder() {
        return Err(ErrorInternalServerError(resp));
    }

    // 3. 解析 multipart 表单, 得到 (player_id, file_path)
    let (player_id, saved_filepath) = match parse_multipart_fields(payload).await {
        Ok(t) => t,
        Err(resp) => return Err(ErrorInternalServerError(resp)),
    };

    // 4. 清理旧文件
    delete_oldest_files(Path::new(UPLOAD_FOLDER), MAX_FOLDER_SIZE_MB);

    // 5. 运行 mjai-reviewer 命令并返回 HTML 结果
    let html_output = match run_mjai_reviewer(&saved_filepath, &player_id) {
        Ok(path) => path,
        Err(e) => {
            return Err(ErrorInternalServerError(e));
        }
    };

    // 6. 返回 HTML 文件
    NamedFile::open(html_output).map_err(|e| {
        ErrorInternalServerError(e.to_string())
    })
}


fn check_request_size(req: &HttpRequest) -> Result<(), String> {
    if req.headers()
        .get("Content-Length")
        .and_then(|h| h.to_str().ok())
        .and_then(|len_str| len_str.parse::<u64>().ok())
        .filter(|&len| len > MAX_CONTENT_LENGTH)
        .is_some()
    {
        return Err(format!("Request size exceeds limit of {} bytes", MAX_CONTENT_LENGTH));
    }
    Ok(())
}

fn ensure_upload_folder() -> Result<(), String> {
    fs::create_dir_all(UPLOAD_FOLDER).map_err(|e| {
        format!("Error creating upload folder: {}", e)
    })?;
    Ok(())
}


async fn parse_multipart_fields(mut payload: Multipart)
    -> Result<(String, PathBuf), String>
{
    // 默认 player_id="0"
    let mut player_id = "0".to_string();
    let mut saved_filepath: Option<PathBuf> = None;

    while let Some(item) = payload.next().await {
        let field = match item {
            Ok(f) => f,
            Err(e) => {
                return Err(format!("Error reading multipart field: {}", e));
            }
        };

        let cd = field.content_disposition();
        if let Some(name) = cd.get_name() {
            if name == "player_id" {
                player_id = match read_player_id(field).await {
                    Ok(id) => id,
                    Err(resp) => return Err(resp),
                };
            } else if name == "file" {
                saved_filepath = match save_uploaded_file(field).await {
                    Ok(path) => Some(path),
                    Err(resp) => return Err(resp),
                };
            }
        }
    }

    // 如果没有 "file" 字段就报错
    let saved_filepath = match saved_filepath {
        Some(p) => p,
        None => {
            return Err("No file provided".to_string());
        }
    };

    Ok((player_id, saved_filepath))
}


async fn read_player_id(mut field: actix_multipart::Field) -> Result<String, String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.next().await {
        let data = match chunk {
            Ok(d) => d,
            Err(e) => {
                return Err(format!("Error reading player_id: {}", e));
            }
        };
        bytes.extend_from_slice(&data);
    }
    let id = String::from_utf8_lossy(&bytes).trim().to_string();
    Ok(id)
}


async fn save_uploaded_file(mut field: actix_multipart::Field) -> Result<PathBuf, String> {
    // 取出原文件名(若无则返回错误)
    let cd = field.content_disposition();
    let filename = match cd.get_filename() {
        Some(f) => sanitize(f),
        None => {
            return Err("No filename provided".to_string());
        }
    };

    // 构造唯一文件名
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let stem = Path::new(&filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = Path::new(&filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let unique_name = format!("{}_{}.{}", stem, now, ext);
    let final_path = Path::new(UPLOAD_FOLDER).join(&unique_name);

    // 创建文件
    let mut file_handle = match fs::File::create(&final_path) {
        Ok(f) => f,
        Err(e) => {
            return Err(format!("Error creating file: {}", e));
        }
    };

    // 逐块写入
    while let Some(chunk) = field.next().await {
        let data = match chunk {
            Ok(d) => d,
            Err(e) => {
                return Err(format!("Error reading file: {}", e));
            }
        };
        if let Err(e) = file_handle.write_all(&data) {
            return Err(format!("Error writing file: {}", e));
        }
    }

    Ok(final_path)
}


fn run_mjai_reviewer(file_path: &Path, player_id: &str) -> Result<String, String> {
    // 执行命令
    let status = Command::new("./mjai-reviewer")
        .args(&[
            "-e",
            "mortal",
            "-i",
            &file_path.to_string_lossy(),
            "-a",
            player_id,
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            let html_output = format!("{}.html", file_path.to_string_lossy());
            Ok(html_output)
        }
        Ok(s) => Err(format!("mjai-reviewer failed with status: {}", s)),
        Err(e) => Err(format!("Error running mjai-reviewer: {}", e)),
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    fs::create_dir_all(UPLOAD_FOLDER).ok();

    HttpServer::new(|| {
        App::new()
            .app_data(web::PayloadConfig::new(MAX_CONTENT_LENGTH as usize))
            .service(index)
            .service(upload_file)
    })
    .bind(("0.0.0.0", 5000))?
    .run()
    .await
}


#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use std::fs;

    // 测试主页
    #[actix_web::test]
    async fn test_index() {
        let app = test::init_service(App::new().service(index)).await;
        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let static_file_path = PathBuf::from("static/index.html");

        // 确保文件存在
        assert!(static_file_path.exists(), "static/index.html does not exist");

        // 读取预期的文件内容
        let expected_content = fs::read_to_string(&static_file_path)
            .expect("Failed to read static/index.html");

        let body = test::read_body(resp).await;
        assert_eq!(body, expected_content);
    }

}
