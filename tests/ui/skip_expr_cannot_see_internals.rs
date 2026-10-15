use conf::Conf;

// A skip expression runs in its own fn, so it can't see the derive's local variables
#[derive(Conf)]
pub struct SkipSeesInternals {
    #[conf(skip = { let _ = __conf_context__; 1 })]
    pub value: u32,
}

fn main() {}
