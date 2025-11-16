use conf::Conf;
use conf::anstyle::AnsiColor;

/// Example demonstrating colored help text output
///
/// Run with `cargo run --example colored_help -- --help` to see styled help text
/// or `cargo run --example colored_help -- --name Alice --count 42` to run normally

// Define custom color scheme for help text
const HELP_STYLES: conf::Styles = conf::Styles::styled()
    .header(AnsiColor::Blue.on_default().bold())
    .usage(AnsiColor::Blue.on_default().bold())
    .literal(AnsiColor::White.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Conf, Debug)]
#[conf(
    name = "colored_help_example",
    about = "An example program demonstrating colored help output",
    styles = HELP_STYLES
)]
struct Args {
    /// Your name
    #[conf(long)]
    name: String,

    /// How many items to process
    #[conf(long)]
    count: Option<usize>,

    /// Enable verbose output
    #[conf(long)]
    verbose: bool,

    /// Input files to process
    #[conf(repeat, pos)]
    files: Vec<String>,
}

fn main() {
    let args = Args::parse();

    println!("Hello, {}!", args.name);

    if let Some(count) = args.count {
        println!("Processing {} items", count);
    }

    if args.verbose {
        println!("Verbose mode enabled");
    }

    if !args.files.is_empty() {
        println!("Files to process:");
        for file in &args.files {
            println!("  - {}", file);
        }
    }
}
