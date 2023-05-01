use std::ops::Deref;
use std::rc::Rc;

use anyhow::Context as _;
use async_hofs::prelude::*;
use prost::Message;
use reqwest::{Client, Url};
use shoutter_api_interface::SafeEndpoint;
use yew::{
    function_component, hook, html, use_context, use_effect_with_deps, use_state, Children,
    ContextProvider, Properties,
};

#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: Url,
    client: Rc<reqwest::Client>,
}

impl Eq for ApiClient {}
impl PartialEq for ApiClient {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.client, &other.client)
    }
}

#[derive(Debug)]
pub enum RequestState<R> {
    Pending,
    Ok(R),
    Err(anyhow::Error), // TODO:
}

#[derive(Properties, PartialEq)]
pub struct ProvideApiClientProps {
    pub base_url: Url,
    pub children: Children,
}

#[function_component]
pub fn ProvideApiClient(props: &ProvideApiClientProps) -> yew::Html {
    let p = ApiClient {
        base_url: props.base_url.clone(),
        client: Rc::new(Client::new()),
    };
    html! {
        <ContextProvider<ApiClient> context={p}>
            {props.children.clone()}
        </ContextProvider<ApiClient>>
    }
}

#[hook]
pub fn use_safe_endpoint<E: SafeEndpoint>(
    endpoint: E,
    url_params: E::UrlParam,
    body: E::RequestBody,
) -> impl Deref<Target = RequestState<E::ResponseBody>> {
    let state = use_state(|| RequestState::Pending);

    let client =
        use_context::<ApiClient>().expect("api client context not found: did you provide it?");

    {
        let state = state.clone();
        use_effect_with_deps(
            move |_| {
                wasm_bindgen_futures::spawn_local(async move {
                    let ApiClient {
                        base_url: mut url,
                        client,
                    } = client;

                    url.set_path(&endpoint.path(url_params));

                    let res = client
                        .request(E::METHOD, url)
                        .body(body.encode_to_vec())
                        .send()
                        .await
                        .context("failed to send request")
                        .and_then(|x| {
                            x.error_for_status()
                                .context("response status code is not ok")
                        })
                        .async_and_then(|x| async move {
                            x.bytes().await.context("failed to fetch response body")
                        })
                        .await
                        .and_then(|x| {
                            E::ResponseBody::decode(x)
                                .context("failed to deserialize response body")
                        });

                    match res {
                        Ok(x) => state.set(RequestState::Ok(x)),
                        Err(e) => state.set(RequestState::Err(e)),
                    }
                })
            },
            (),
        );
    }

    state
}
