use shoutter_api_interface::protobuf::{Greeting, GreetingName};
use shoutter_api_interface::GreetingEndpoint;

use crate::endpoints::EndpointHandler;

#[derive(Clone)]
pub struct GreetingEndpointHandler;
impl EndpointHandler for GreetingEndpointHandler {
    type Endpoint = GreetingEndpoint;

    fn handle(&mut self, _url_param: (), body: GreetingName) -> Result<Greeting, String> {
        println!(">> Received greeting endpoint.");

        Ok(Greeting {
            content: format!("Hello {}, from shoutter_webserver!", body.name),
        })
    }
}
