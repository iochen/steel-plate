use gtmpl;
use infer;
use lazy_static;
use minify_html;
use tokio;

use async_once::AsyncOnce;
use hyper::body::HttpBody;
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::cmp::min;

// aws SDK
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::model::{AttributeAction, AttributeValue, AttributeValueUpdate, ReturnValue};
use aws_sdk_dynamodb::Client;

// lambda runtime
use lambda_http::{http, lambda_runtime::Error, service_fn, Body, Request, Response};

struct DBClient {
    client: Client,
}

lazy_static::lazy_static! {
    static ref DB_CLIENT:AsyncOnce<DBClient> =   AsyncOnce::new(async {
            let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
            let config = aws_config::from_env().region(region_provider).load().await;
            let client = Client::new(&config);
            DBClient {
                client
            }
        });

    static ref INDEX: String = get_index();
}

#[derive(RustEmbed)]
#[folder = "src/public/"]
#[exclude = "src/public/src_raw/*"]
struct Asset;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // lambda run
    lambda_http::run(service_fn(handle)).await?;
    Ok(())
}

// lambda handler
async fn handle(_req: Request) -> http::Result<Response<Body>> {
    // get uri
    let uri = _req.uri().clone();
    // get && refine path
    let path = uri.path().trim_start_matches("/prod");

    // GET /src/*
    if path.starts_with("/src/") && _req.method().as_str() == "GET" {
        // get relative path
        let ass = Asset::get(path.trim_start_matches("/"));
        if ass.is_none() {
            return not_found_resp(Some(path));
        }
        let ass = ass.unwrap();
        // return file
        return file_resp(ass.data);
    }

    return match (path, _req.method().as_str()) {
        // GET /
        ("/", "GET") => {
            // get current total clicks
            let total = DB_CLIENT.get().await.get_total().await;
            if total.is_err() {
                eprintln!("Error getting total: {}", total.unwrap_err());
                return server_err_resp();
            }
            // render index
            let index = gtmpl::template(INDEX.as_str(), total.unwrap()).unwrap();
            html_resp(index.as_str())
        }
        // POST /submit
        ("/submit", "POST") => {
            // get user body data
            let body = _req.into_body().data().await;
            // has body
            if body.is_some() {
                let body = body.unwrap();
                // body okay
                if body.is_ok() {
                    let body = body.unwrap();
                    let body = String::from_utf8(body.to_vec());
                    // able to convert to utf-8
                    if body.is_ok() {
                        let d_count = body.unwrap().parse::<u32>();
                        // able to parse as u32
                        if d_count.is_ok() {
                            let d_count = d_count.unwrap();
                            // adequate number
                            if d_count > 0 && d_count < 100 {
                                let total = DB_CLIENT.get().await.total_add(d_count).await;
                                if total.is_err() {
                                    eprintln!("Error getting total: {}", total.unwrap_err());
                                    return server_err_resp();
                                }
                                return json_resp(
                                    format!("{{\"total\": {}}}", total.unwrap().to_string())
                                        .as_str(),
                                );
                            }
                        }
                    }
                }
            }
            let total = DB_CLIENT.get().await.get_total().await;
            if total.is_err() {
                eprintln!("Error getting total: {}", total.unwrap_err());
                return server_err_resp();
            }
            json_resp(format!("{{\"total\": {}}}", total.unwrap().to_string()).as_str())
        }
        // *
        _ => not_found_resp(Some(path)),
    };
}

// get minified index file
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

// response binary file
fn file_resp(b: Cow<'static, [u8]>) -> http::Result<Response<Body>> {
    let mut resp = Response::builder()
        .status(200)
        .header("Cache-Control", "public, max-age=6048000, immutable");
    let buf = b.get(0..min(31, b.len()));
    if buf.is_some() {
        let ct = infer::get(buf.unwrap()).map(|t| t.to_string());
        if ct.is_some() {
            resp = resp.header("Content-Type", ct.unwrap());
        }
    }
    resp.body(Body::Binary(b.to_vec()))
}

// response json
fn json_resp(b: &str) -> http::Result<Response<Body>> {
    let resp = Response::builder()
        .status(200)
        .header("Content-Type", "application/json");
    resp.body(Body::from(b))
}

// response html
fn html_resp(b: &str) -> http::Result<Response<Body>> {
    let resp = Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=UTF-8");
    resp.body(Body::from(b))
}

// return 404 Not Found
fn not_found_resp(path: Option<&str>) -> http::Result<Response<Body>> {
    let body: Body = Body::from("404 Not Found");
    if path.is_some() {
        Some(Body::from(format!(
            "404 Not Found (path: \"{}\")",
            path.unwrap()
        )));
    }
    let resp = Response::builder().status(404);
    resp.body(body)
}

// return 500 Internal Server Error
fn server_err_resp() -> http::Result<Response<Body>> {
    let resp = Response::builder().status(500);
    resp.body(Body::from("500 Internal Server Error"))
}

impl DBClient {
    // add clicks count to db
    async fn total_add(&self, d: u32) -> Result<u32, Error> {
        if d < 1 {
            return self.get_total().await;
        }
        let upd = AttributeValueUpdate::builder()
            .action(AttributeAction::Put)
            .value(AttributeValue::N((self.get_total().await? + d).to_string()))
            .build();
        let resp = self
            .client
            .update_item()
            .table_name("steel-plate")
            .key("key", AttributeValue::S("total".to_string()))
            .attribute_updates("value", upd)
            .return_values(ReturnValue::UpdatedNew)
            .send()
            .await;
        if resp.is_err() {
            return Err(Error::from(resp.unwrap_err().to_string()));
        }
        Ok(resp
            .unwrap()
            .attributes
            .unwrap()
            .get("value")
            .unwrap()
            .as_n()
            .unwrap()
            .parse()
            .unwrap())
    }

    // get total clicks from db
    async fn get_total(&self) -> Result<u32, Error> {
        let q = self
            .client
            .get_item()
            .table_name("steel-plate")
            .key("key", AttributeValue::S("total".to_string()))
            .attributes_to_get("value");
        let resp = q.send().await;
        if resp.is_err() {
            return Err(Error::from(resp.unwrap_err().to_string()));
        }

        let result = resp
            .unwrap()
            .item
            .unwrap()
            .get("value")
            .unwrap()
            .as_n()
            .unwrap()
            .parse::<u32>();
        if result.is_err() {
            return Err(Error::from(result.unwrap_err().to_string()));
        }
        Ok(result.unwrap())
    }
}
