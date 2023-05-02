#![feature(stmt_expr_attributes)]

use std::net::SocketAddr;

use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::cors::cors_layer;
use crate::endpoints::create_routing;

mod cors;
mod endpoints;
mod extractor;

const BASE_URL: &str = "[::]:3000";

#[tokio::main]
async fn main() {
    #[cfg(feature = "test-requester")]
    test_requester::hoge();

    // FIXME: The log is not emitted when the client smashed endpoints.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let make_service = create_routing()
        .layer(cors_layer())
        .layer(TraceLayer::new_for_http())
        .into_make_service();

    axum::Server::bind(&BASE_URL.parse().unwrap())
        .serve(make_service)
        .await
        .unwrap();
}

#[cfg(feature = "test-requester")]
mod test_requester {
    use std::time::Duration;

    use prost::Message;
    use shoutter_api_interface::protobuf::{Greeting, GreetingName};

    pub fn hoge() {
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let bytes = GreetingName {
                name: "Flisan".to_string(),
            };

            let response = reqwest::Client::new()
                .post("http://localhost:3000/greet")
                .body(bytes.encode_to_vec())
                .send()
                .await
                .unwrap();

            let hoge = Greeting::decode(response.bytes().await.unwrap());

            println!("{:#?}", hoge);
        });
    }
}
