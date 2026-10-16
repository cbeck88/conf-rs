use conf::Conf;

#[derive(Conf)]
struct BadConfig {
    #[conf(long, default_value_help_str = "some help")]
    bad_field: String,
}

fn main() {}
