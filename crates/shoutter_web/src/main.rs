mod hooks;
mod pages;

use once_cell::sync::Lazy;
use reqwest::Url;
use tracing_subscriber::fmt::format::{FmtSpan, Pretty};
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yew::{function_component, html, Html};
use yew_router::{BrowserRouter, Switch};

use crate::hooks::api_client::ProvideApiClient;
use crate::pages::{switch, Route};

static BASE_URL: Lazy<Url> = Lazy::new(|| "http://localhost:3000".parse().unwrap());

#[function_component]
fn App() -> Html {
    html! {
        <ProvideApiClient base_url={BASE_URL.clone()}>
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
        </ProvideApiClient>
    }
}

fn main() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_timer(UtcTime::rfc_3339())
        .with_writer(tracing_web::MakeConsoleWriter)
        .with_span_events(FmtSpan::ACTIVE);
    let perf_layer = tracing_web::performance_layer().with_details_from_fields(Pretty::default());

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();

    tracing::info!("starting up!");

    yew::Renderer::<App>::new().render();
}
