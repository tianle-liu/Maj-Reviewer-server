mod lib; // 引入同一crate下的 lib.rs
use lib::{get_folder_size, delete_oldest_files};

use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{
    dev::Payload, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder
};
use futures_util::stream::StreamExt as _;
use serde::Serialize;
use sanitize_filename::sanitize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

static UPLOAD_FOLDER: &str = "uploads";
static MAX_FOLDER_SIZE_MB: u64 = 100;
static MAX_CONTENT_LENGTH: u64 = 100 * 1024; // 100KB

#[derive(Serialize)]
struct UploadResponse {
    filepath: Option<String>,
    error: Option<String>,
}

#[get("/")]
async fn index () -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open("static/index.html")?)
}

#[post("/upload")]
async fn upload_files(req: HttpRequest, mut payload: Multipart) -> impl Responder {
    HttpResponse::NotImplemented().body()
}

#[get("/result/{filename:.*}")]
async fn get_result_file(path: web::Path<String>) -> impl Responder {
    HttpResponse::NotImplemented().body()
}

fn check_request_size(req: &HttpRequest) -> Result<(), HttpResponse> {
    if req.headers()
        .get("Content-Length")
        .and_then(|h| h.to_str().ok())
        .and_then(|len_str| len_str.parse::<u64>().ok())
        .filter(|&len| len > MAX_CONTENT_LENGTH)
        .is_some()
    {
        return Err(HttpResponse::BadRequest().json(UploadResponse {
            filepath: None,
            error: Some("Request size too large".to_string()),
        }));
    }
    Ok(())
}

fn ensure_upload_folder() -> Result<(), HttpResponse> {
    fs::create_dir_all(UPLOAD_FOLDER).map_err(|e| {
        HttpResponse::InternalServerError().json(UploadResponse {
            filepath: None,
            error: Some(format!("Could not create uploads folder: {}", e)),
        })
    })?;
    Ok(())
}


async fn parse_multipart_fields(mut payload: Multipart) -> Result<(String, PathBuf), HttpResponse> {
    todo!()
}

async fn read_player_id(mut field: actix_multipart::Field) -> Result<String, HttpResponse> {
    todo!()
}

async fn save_uploaded_file(mut field: actix_multipart::Field) -> Result<PathBuf, HttpResponse> {
    todo!()
}

fn run_mjai_reviewer(file_path: &Path, player_id: &str) -> Result<String, String> {
    todo!()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    todo!()
}