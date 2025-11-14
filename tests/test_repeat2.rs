use conf::Conf;

mod common;
use common::vec_str;

#[derive(Conf, Debug)]
struct Config {
    #[conf(repeat, env)]
    pub assets: Vec<String>,
    #[conf(repeat, env)]
    pub asset_pairs: Vec<String>,
    #[conf(long, env, default_value = "10s")]
    pub error_pause: String,
}

#[test]
fn test_repeat2() {
    let result = Config::try_parse_from::<&str, &str, &str>(vec!["."], vec![]).unwrap();
    assert_eq!(result.assets, vec_str([]));
    assert_eq!(result.asset_pairs, vec_str([]));
    assert_eq!(result.error_pause, "10s");
}
