use http::Method;

pub mod protobuf;

pub trait Endpoint: Send + Sync + 'static {
    const METHOD: Method;

    type UrlParam;
    type RequestBody: prost::Message + Default + Send + Sync + 'static;
    type ResponseBody: prost::Message + Default + Send + Sync + 'static;

    fn path(&self, params: Self::UrlParam) -> String;
}

/// https://developer.mozilla.org/en-US/docs/Glossary/Safe/HTTP
pub trait SafeEndpoint: Endpoint {}

pub struct GreetingEndpoint;

impl SafeEndpoint for GreetingEndpoint {}

impl Endpoint for GreetingEndpoint {
    // FIXME: POST method is not safe. but we'll use POST method for protobuf demo.
    const METHOD: Method = Method::POST;

    type UrlParam = ();
    type RequestBody = protobuf::GreetingName;
    type ResponseBody = protobuf::Greeting;

    fn path(&self, _: Self::UrlParam) -> String {
        "/greet".to_owned()
    }
}
