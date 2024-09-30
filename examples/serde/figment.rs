use conf::Conf;
use figment::{
    providers::{Format, Json, Toml},
    value::Value,
    Figment,
};
use std::env;

#[path = "./model_service.rs"]
mod model_service;
use model_service::ModelServiceConfig;

pub fn main() {
    // In this example, we show how figment can be used together with conf
    // in order to load content from multiple files, merge them according to
    // some hierarchical order, and then supply the result to conf.
    //
    // This takes advantage of conf's comprehensive error reporting, and gives
    // better results than if you just extract() directly into your final
    // config structure.
    let mut fig = Figment::new();

    if let Some(path) = env::var_os("TOML") {
        fig = fig.merge(Toml::file(path));
    }
    if let Some(path) = env::var_os("TOML2") {
        fig = fig.merge(Toml::file(path));
    }
    if let Some(path) = env::var_os("JSON") {
        fig = fig.merge(Json::file(path));
    }

    let doc_content: Value = fig.extract().unwrap();

    let config = ModelServiceConfig::conf_builder()
        .doc("files", &doc_content)
        .parse();

    println!("{config:#?}");
}
