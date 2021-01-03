use std::borrow::BorrowMut;

use actix_web::{Error, get, HttpResponse, Result, web};
use actix_web::body::Body;
use json::JsonValue;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::db::accessor::{DbENRecord, PgConnection};

#[derive(Debug, Serialize, Deserialize)]
struct Hello {
    name: String,
}


pub struct Router {}

impl Router {
    fn return_response<T>(db_res: Result<T, tokio_postgres::Error>) -> Result<HttpResponse<Body>, Error>
    where T: serde::Serialize
    {
        let res = db_res.map(|recs| HttpResponse::Ok().json(recs))
            .map_err(|err| {
                eprintln!("{}", err);
                HttpResponse::InternalServerError()
            })?;
        Ok(res)
    }

    pub async fn get_notification(
        path: web::Path<String>,
        db: web::Data<PgConnection>) -> Result<HttpResponse<Body>, Error> {
        let id: i32 = path.into_inner().parse().unwrap();
        let record = Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(db.select_notification(id));
        Router::return_response(record)
    }
    pub async fn get_notifications(db: web::Data<PgConnection>) -> Result<HttpResponse<Body>, Error> {
        let records = Runtime::new()
            .expect("Failed to create Tokio runtime")
            .block_on(db.select_notifications_imut());
        Router::return_response(records)
    }

    pub fn init_routes(config: &mut web::ServiceConfig) {
        let mut pool = PgConnection::new();
        pool.create_connection();

        config
            .data(pool)
            .route("/notifications", web::get().to(Router::get_notifications))
            .route("/notifications/{id}", web::get().to(Router::get_notification));
    }
}
