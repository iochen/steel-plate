use gtmpl;
use lazy_static;
use minify_html;
use tokio;

use async_once::AsyncOnce;
use hegel::http;
use lambda_runtime::{service_fn, Error};
use rust_embed::RustEmbed;

// aws SDK
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::model::{AttributeAction, AttributeValue, AttributeValueUpdate, ReturnValue};
use aws_sdk_dynamodb::Client;

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
    lambda_runtime::run(service_fn(handle)).await?;
    Ok(())
}

// lambda handler
async fn handle(evt: http::Event) -> Result<http::Response, Error> {
    // get path
    let path = evt.payload.path();

    // GET /src/*
    if path.starts_with("/src/") && evt.payload.method().as_str() == "GET" {
        // get relative path
        let ass = Asset::get(path.trim_start_matches("/"));
        if ass.is_none() {
            return Ok(http::Response::new_status(404));
        }
        let ass = ass.unwrap();
        // return file
        return Ok(http::Response::new_file(ass.data.into()).header(
            "Cache-Control".to_string(),
            "public, max-age=6048000, immutable".to_string(),
        ));
    }

    return match (path.as_str(), evt.payload.method().as_str()) {
        // GET /
        ("/", "GET") => {
            // get current total clicks
            let total = DB_CLIENT.get().await.get_total().await;
            if total.is_err() {
                eprintln!("Error getting total: {}", total.unwrap_err());
                return Ok(http::Response::new_status(500));
            }
            // render index
            let index = gtmpl::template(INDEX.as_str(), total.unwrap()).unwrap();
            Ok(http::Response::new_html(index))
        }
        // POST /submit
        ("/submit", "POST") => {
            let body = evt.payload.body();
            // body not error
            if body.is_ok() {
                let body = body.unwrap_or(None);
                // body not none
                if body.is_none() {
                    let body = body.unwrap();

                    let d_count = body.parse::<u32>();
                    // able to parse as u32
                    if d_count.is_ok() {
                        let d_count = d_count.unwrap();
                        // adequate number
                        if d_count > 0 && d_count < 100 {
                            let total = DB_CLIENT.get().await.total_add(d_count).await;
                            if total.is_err() {
                                eprintln!("Error getting total: {}", total.unwrap_err());
                                return Ok(http::Response::new_status(500));
                            }
                            return Ok(http::Response::new_json(format!(
                                "{{\"total\": {}}}",
                                total.unwrap().to_string()
                            )));
                        }
                    }
                }
            }
            let total = DB_CLIENT.get().await.get_total().await;
            if total.is_err() {
                eprintln!("Error getting total: {}", total.unwrap_err());
                return Ok(http::Response::new_status(500));
            }
            Ok(http::Response::new_json(format!(
                "{{\"total\": {}}}",
                total.unwrap().to_string()
            )))
        }
        // *
        _ => Ok(http::Response::new_status(404)),
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
