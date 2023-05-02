mod greeting;

use axum::Router;
use prost::Message;
use shoutter_api_interface::{Endpoint, GreetingEndpoint};
use tracing::info;

use crate::endpoints::greeting::GreetingEndpointHandler;
use crate::extractor::Proto;

pub trait EndpointHandler {
    type Endpoint: Endpoint;

    // TODO: How can we deal with the error?
    fn handle(
        &mut self,
        url_param: <Self::Endpoint as Endpoint>::UrlParam,
        body: <Self::Endpoint as Endpoint>::RequestBody,
    ) -> Result<<Self::Endpoint as Endpoint>::ResponseBody, String>;
}

pub fn create_routing() -> Router {
    let router = Router::new();

    route(router, GreetingEndpoint, GreetingEndpointHandler)
}

fn route<E, H>(router: Router, endpoint: E, handler: H) -> Router
where
    E: Endpoint<UrlParam = ()>,
    H: EndpointHandler<Endpoint = E> + Clone + Send + Sync + 'static,
{
    use axum::routing::{delete, get, head, options, patch, post, put, trace};

    let path = endpoint.path(());
    info!("🗺️ '{path}'");

    let method_router = match E::METHOD.as_str() {
        "DELETE" => delete,
        "GET" => get,
        "HEAD" => head,
        "OPTIONS" => options,
        "PATCH" => patch,
        "POST" => post,
        "PUT" => put,
        "TRACE" => trace,
        _ => unimplemented!(
            "During registering '{}': Unexpected HTTP method '{}' ({:#?})",
            path,
            E::METHOD.as_str(),
            E::METHOD
        ),
    };

    router.route(
        &path,
        method_router(|body| handler_fn::<E, H>(handler, body)),
    )
}

async fn handler_fn<E, H>(
    mut handler: H,
    Proto(proto): Proto<<E as Endpoint>::RequestBody>,
) -> Vec<u8>
where
    E: Endpoint<UrlParam = ()>,
    H: EndpointHandler<Endpoint = E>,
{
    handler.handle((), proto).unwrap().encode_to_vec()
}
