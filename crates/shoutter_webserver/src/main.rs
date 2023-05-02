#![feature(stmt_expr_attributes)]

use crate::endpoints::create_routing;

mod endpoints;
mod extractor;

#[tokio::main]
async fn main() {
    let route = create_routing();

    #[cfg(feature = "test-requester")]
    test_requestor::hoge();

    axum::Server::bind(&"127.0.0.1:8000".parse().unwrap())
        .serve(route.into_make_service())
        .await
        .unwrap();
}

#[cfg(feature = "test-requester")]
mod test_requestor {
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
                .post("http://localhost:8000/greet")
                .body(bytes.encode_to_vec())
                .send()
                .await
                .unwrap();

            let hoge = Greeting::decode(response.bytes().await.unwrap());

            println!("{:#?}", hoge);
        });
    }
}
