use conf::Conf;
use std::{env, ffi::OsString, fs};

#[path = "./model_service.rs"]
mod model_service;
use model_service::ModelServiceConfig;

pub fn main() {
    // In this example, the user may specify `--config PATH` or `--config=PATH` on the CLI args,
    // or set an environment variable `CONFIG`. Then the file is loaded in json format if present,
    // and used as a value-source during config parsing.
    //
    // This has to be done before calling `conf::Parse` because we have to supply the parsed config
    // file to `conf` as one of the value sources if we want to use it.
    //
    // `conf::find_parameter` is a minimal function which uses `clap_lex` to search for just one
    // parameter, without introducing any additional dependencies.
    let config_path: Option<OsString> =
        conf::find_parameter("config", env::args_os()).or_else(|| env::var_os("CONFIG"));

    let config = if let Some(config_path) = config_path {
        // When the config file is specified, its an error if it can't be found or can't be parsed
        // in the expected format.
        let file_contents = fs::read_to_string(&config_path).expect("Could not open config file");
        let doc_content: toml::Value = toml::from_str(&file_contents).expect("Config file format");
        // Now we pass the parsed content to the builder, with the file name so that it can be used
        // in error messages.
        ModelServiceConfig::conf_builder()
            .doc(config_path.to_string_lossy(), doc_content)
            .parse()
    } else {
        // There is no config file, so we can just try to parse from only args and env.
        ModelServiceConfig::parse()
    };

    println!("{config:#?}");
}
