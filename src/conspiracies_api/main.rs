extern crate conspiracies;
extern crate clap; 
extern crate diesel;
extern crate dotenv;
extern crate env_logger;
extern crate actix;
extern crate actix_web;
extern crate futures;
#[macro_use] 
extern crate serde_derive;

use actix::{Addr,Syn};
use actix::prelude::*;
use conspiracies::actors::{
    tags::{AddTag, Tags}, 
    conspiracies::*, 
    db_executor::*,
};
use actix_web::{http, http::header, App, AsyncResponder, FutureResponse, Path, HttpRequest, HttpResponse};
use actix_web::server::HttpServer;
use futures::Future;
use actix_web::Error;
use actix_web::fs;
use actix_web::Json;
use actix_web::middleware::Logger;
use actix_web::middleware::cors::{Cors};
use diesel::prelude::*;
use conspiracies::models;

/// This is state where we will store *DbExecutor* address.
struct State {
    db: Addr<Syn, DbExecutor>,
}

fn add_tag((req, tag): (HttpRequest<State>, Json<models::NewTag>)) -> Box<Future<Item=HttpResponse, Error=Error>> {
    req.state().db.send(AddTag{tag: tag.into_inner()})
        .from_err()
        .and_then(|res| {
            match res {
                Ok(i) => Ok(HttpResponse::Ok().json(i)),
                Err(e) => {
                    println!("add_tag error: {}", e);
                    Ok(HttpResponse::InternalServerError().into())
                }
            }
        })
        .responder()
}
/// Returns a paginated list of tags that are available
fn get_tags(req: HttpRequest<State>) -> impl Future<Item=HttpResponse, Error=Error> {
    let page_num = req.query().get("page").unwrap_or("0").parse::<i32>().unwrap();

    req.state().db.send(Tags{page_num: page_num})
        .from_err()
        .and_then(|res| {
            match res {
                Ok(tags) => Ok(HttpResponse::Ok().json(tags)),
                Err(_) => Ok(HttpResponse::InternalServerError().into())
            }
        })
        .responder()
}

/// Returns a paginated list of conspriacies. IF no page size is given the default is 25
fn get_conspiracies(req: HttpRequest<State>) -> impl Future<Item=HttpResponse, Error=Error> {
    let page_num = req.query().get("page").unwrap_or("0").parse::<i32>().unwrap();

    req.state().db.send(Conspiracies{page_num: page_num})
        .from_err()
        .and_then(|res| {
            match res {
                Ok(conspiracies) => Ok(HttpResponse::Ok().json(conspiracies)),
                Err(_) => Ok(HttpResponse::InternalServerError().into())
            }
        })
        .responder()
}

/// returns the conspiracy by the given id
fn get_conspiracies_by_id(req: HttpRequest<State>) -> impl Future<Item=HttpResponse, Error=Error> {
    let page_id = &req.match_info()["page_id"];

    // Send message to `DbExecutor` actor
    req.state().db.send(GetConspiracy{page_id: page_id.to_owned()})
        .from_err()
        .and_then(|res| {
            match res {
                Ok(conspiracy) => Ok(HttpResponse::Ok().json(conspiracy)),
                Err(_) => Ok(HttpResponse::NotFound().into())
            }
        })
        .responder()
}

/// returns the conspiracy by the given id
fn get_conspiracies_by_tag((req, params): (HttpRequest<State>, Path<(i32)>)) -> Box<Future<Item=HttpResponse, Error=Error>> {
    let page_num = req.query().get("page").unwrap_or("0").parse::<i32>().unwrap();

    req.state().db.send(GetConspiraciesByTag{page_num: page_num, tag_id: params.into_inner()})
        .from_err()
        .and_then(|res| {
            match res {
                Ok(i) => Ok(HttpResponse::Ok().json(i)),
                Err(e) => {
                    println!("get_conspiracies_by_tag error: {}", e);
                    Ok(HttpResponse::InternalServerError().into())
                }
            }
        })
        .responder()
}


fn tag_conspiracy((req, conspiracy_tag): (HttpRequest<State>, Json<models::ConspiracyTag>)) -> Box<Future<Item=HttpResponse, Error=Error>> {
    
    req.state().db.send(TagConspiracy{tag: conspiracy_tag.into_inner()})
        .from_err()
        .and_then(|res| {
            match res {
                Ok(i) => Ok(HttpResponse::Ok().json(i)),
                Err(e) => {
                    println!("tag_conspiracy error: {}", e);
                    Ok(HttpResponse::InternalServerError().into())
                }
            }
        })
        .responder()
}

fn index(_req: HttpRequest<State>) -> std::result::Result<actix_web::fs::NamedFile, std::io::Error> {
    match fs::NamedFile::open("site/index.html") {
        Ok(file) => Ok(file),
        Err(e) => Err(e)
    }
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let sys = actix::System::new("conspiracies-api");
    // Start 3 parallel db executors
    let addr = SyncArbiter::start(3, || {
        DbExecutor(SqliteConnection::establish("database/conspiracies.sqlite3").unwrap())
    });
 
    // Start http server

    HttpServer::new(move || {
        App::with_state(State{db: addr.clone()})
            .middleware(Logger::default())
            .configure(|app| {
                Cors::for_app(app)
                    .allowed_methods(vec![http::Method::GET, http::Method::POST, http::Method::PUT, http::Method::OPTIONS])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600)
                    .resource("/", |r| r.method(http::Method::GET).f(index))
                    .resource("/conspiracies/{page_id}", |r| r.method(http::Method::GET).a(get_conspiracies_by_id))
                    .resource("/conspiracies/{page_id}/tag", |r| r.method(http::Method::POST).with(tag_conspiracy))
                    .resource("/tags/new", |r| r.method(http::Method::POST).with(add_tag))
                    .resource("/tags", |r| r.method(http::Method::GET).a(get_tags))
                    .resource("/tags/{tag_id}/conspiracies", |r| r.method(http::Method::GET).with(get_conspiracies_by_tag))
                    .resource("/conspiracies", |r| r.method(http::Method::GET).a(get_conspiracies))
                    .register()
            })})
        .bind("127.0.0.1:8088").unwrap()
        .start();

    println!("Started http server: 127.0.0.1:8088");
    let _ = sys.run();
}
