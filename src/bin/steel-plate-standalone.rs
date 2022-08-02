use gtmpl;
use lazy_static;
use minify_html;
use tokio;

use rust_embed::RustEmbed;
use std::sync::atomic::{AtomicU32, Ordering};

use hyper::body::HttpBody;
use hyper::service::{make_service_fn, service_fn};
use hyper::{http, Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;

static TOTAL: AtomicU32 = AtomicU32::new(0);

lazy_static::lazy_static! {
    static ref INDEX: String = get_index();
}

#[derive(RustEmbed)]
#[folder = "src/public/"]
#[exclude = "src/public/src_raw/*"]
struct Asset;

#[tokio::main]
async fn main() {
    let count_base = option_env!("STEEL_PLATE_COUNT_BASE");
    if count_base.is_some() {
        TOTAL.store(
            count_base
                .unwrap()
                .parse::<u32>()
                .expect("Error when parsing count base from env"),
            Ordering::Relaxed,
        );
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 8082));
    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
    let server = Server::bind(&addr).serve(make_service);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

async fn handle(_req: Request<Body>) -> http::Result<Response<Body>> {
    let uri = _req.uri().clone();
    let path = uri.path();
    if path.starts_with("/src/") && _req.method().as_str() == "GET" {
        let ass = Asset::get(path.trim_start_matches("/"));
        if ass.is_none() {
            let resp = Response::builder().status(StatusCode::NOT_FOUND);
            return resp.body(Body::from("404 Not Found"));
        }
        let ass = ass.unwrap();
        return Ok(Response::new(Body::from(ass.data)));
    }

    return match (path, _req.method().as_str()) {
        ("/", "GET") => {
            let index = gtmpl::template(INDEX.as_str(), TOTAL.load(Ordering::Relaxed)).unwrap();
            Ok(Response::new(Body::from(index)))
        }
        ("/submit", "POST") => {
            let body = _req.into_body().data().await;
            if body.is_some() {
                let body = body.unwrap();
                if body.is_ok() {
                    let body = body.unwrap();
                    let body = String::from_utf8(body.to_vec());
                    if body.is_ok() {
                        let d_count = body.unwrap().parse::<u32>();
                        if d_count.is_ok() {
                            TOTAL.fetch_add(d_count.unwrap(), Ordering::Relaxed);
                        }
                    }
                }
            }
            Ok(Response::new(Body::from(format!(
                "{{\"total\": {}}}",
                TOTAL.load(Ordering::Relaxed)
            ))))
        }
        _ => {
            let resp = Response::builder().status(StatusCode::NOT_FOUND);
            resp.body(Body::from("404 Not Found"))
        }
    };
}

fn get_index() -> String {
    let index_html = Asset::get("index.html").expect("Error reading index_html from embed");
    let index_str = std::str::from_utf8(&index_html.data).expect("Error parsing from index_html");

    let mut cfg = minify_html::Cfg::new();
    cfg.minify_css = true;
    cfg.minify_js = true;
    cfg.do_not_minify_doctype = true;
    cfg.keep_comments = true;
    cfg.remove_bangs = true;
    cfg.keep_spaces_between_attributes = false;
    let minified = minify_html::minify(index_str.as_bytes(), &cfg);
    let index = std::str::from_utf8(minified.as_slice()).expect("Error parsing from minified html");
    index.to_string()
}
