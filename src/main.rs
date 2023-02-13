mod entity;

use std::env::var;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU16, Ordering};
use actix_web::{get, App, HttpResponse, HttpServer, web, post};
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json, scope, ServiceConfig};
use serde_json::json;
use tracing::{info, trace, warn};
use tracing_subscriber::layer::SubscriberExt;
use chrono::Local;
use tracing_subscriber::util::SubscriberInitExt;
use uuid::Uuid;
use crate::entity::book::Book;
use crate::entity::create_book::CreateBook;
use crate::entity::error::{Error};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "actix-memory-repository=debug".into())))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = var("PORT").unwrap_or("8090".to_string());
    let address = format!("127.0.0.1:{}", port);

    info!("Starting server on {}", address);
    let thread_counter = Arc::new(AtomicU16::new(1));
    let data = Data::new(MemoryRepository::init());

    HttpServer::new(move || {
        let thread_index = thread_counter.fetch_add(1, Ordering::SeqCst);
        trace!("Starting thread {}", thread_index);

        App::new()
            .app_data(web::Data::new(thread_index))
            .app_data(data.clone())
            .configure(config)
    })
        .bind(&address)
        .unwrap_or_else(|err| {
            panic!("🔥🔥🔥 Couldn't start the server in port {}: {:?}", port, err)
        })
        .run()
        .await
}

fn config(config: &mut ServiceConfig) {
    let scope = scope("/api")
        .service(hello)
        .service(health)
        .service(create_book);
    config.service(scope);
}

#[get("/")]
async fn hello() -> HttpResponse {
    HttpResponse::Ok().json(Json(json!({ "message": "API Rust with Actix Web is running!!!"})))
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(Json(json!({ "status": "UP"})))
}

#[post("/books")]
async fn create_book(payload: Json<CreateBook>, data: Data<MemoryRepository>) -> HttpResponse {
    if has_invalid_params_on_create(payload.clone()) {
        return HttpResponse::BadRequest()
            .json(Error::new("Invalid params".to_string(), StatusCode::BAD_REQUEST.to_string()));
    }
    let mut books = data.books.lock().unwrap();
    let book_repo = books.iter().find(|book| book.title == payload.title.clone().unwrap());
    if book_repo.is_some() {
        warn!("Book with title {} already exists", book_repo.unwrap().title);
        return HttpResponse::UnprocessableEntity()
            .json(Error::new("Book already exists".to_string(), StatusCode::UNPROCESSABLE_ENTITY.to_string()))
    }

    let book = Book {
        id: Uuid::new_v4(),
        title: payload.title.clone().unwrap(),
        author: payload.author.clone().unwrap(),
        pages: payload.pages.clone().unwrap(),
        created_at: Local::now(),
        updated_at: Local::now()
    };

    books.push(book.clone());
    HttpResponse::Created().json(book)
}

fn has_invalid_params_on_create(payload: CreateBook) -> bool {
    if payload.title.is_none() || payload.author.is_none() || payload.pages.is_none() { return true } return false
}

pub struct MemoryRepository {
    books: Arc<Mutex<Vec<Book>>>
}

impl MemoryRepository {
    fn init() -> Self {
        Self {
            books: Arc::new(Mutex::new(Vec::new()))
        }
    }
}