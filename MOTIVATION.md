# Motivating use case

Suppose you have a web app which consists of 8 microservices (and counting).
When using `clap-derive` to configure them, each one is going to have a service-specific config structure, which might look like this

```rust
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Config {
    /// Socket to listen on for http traffic
    #[clap(long, env = "LISTEN_ADDR", default_value = "127.0.0.1:4040")]
    pub listen_addr: SocketAddr,

    /// URL of auth agent
    #[clap(long, env)]
    pub auth_agent_url: String,

    /// URL of database
    #[clap(long, env)]
    pub database_url: String,

    /// How frequently to frobnicate
    #[clap(long, env, value_parser = utils::parse_duration)]
    pub frobnicate_interval: Duration,

    ...
}
```

When you are just starting, this is pretty manageable. You probably have just a few URLs and maybe a few simple parameters.
`clap-derive` gives you a concise and declarative approach, and generates good `--help` text automatically.

As your project becomes more mature, you discover that you need far more configurability.
Every one of your microservices has shared common infrastructure, which has configuration options.
Logging, metrics, thread-pools, database config, telemetry, auth systems, all have several parameters. You want to be able to add new parameters
in any of these subsystems easily and have it just get added to all of your services that use this subsystem with minimal effort.

At this point, the `clap(flatten)` feature comes to your rescue. You decide that each of these subsystems should declare its own config structure,
which derives `clap::Parser`. Each subsystem is going to be initialized by passing it the parsed config. Then you flatten these configs into the config each of service that needs them.

So your service config looks like:

```rust
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Config {
    ...

    #[clap(flatten)]
    pub logging: LoggingConfig,

    #[clap(flatten)]
    pub metrics: MetricsConfig,

    ...
}
```

And your `main` function looks something like

```rust
#[tokio::main]
async fn main() {
    let _ = dotenvy::from_filename("my_app.env");
    let config = Config::parse();
    let _logging_handle = init_logging(&config.logging);
    let metrics_handle = init_metrics(&config.metrics);
    let db = Db::connect(&config.database_url, &config.database_options);
    let app_state = AppState::new(config, db);
    web_framework::serve(make_app_router(metrics_handle), app_state).await;
}
```

This feels pretty good. If you need to add more options for configuring metrics, database, whatever, you can go to the relevant module, add a new item
to the clap config structure it defines, give it a sane default, and now all your services that need this option just have it now, and you don't have to touch any of the code
specific to your 8 microservices.

However, at some point your project gets even more mature, and you start to run into limitations of `clap(flatten)`.
You discover that you get lots of errors in production, and now for each service, each of your outbound connections needs to have retries. Not only that, you need run-time configurable retries so that you can
react to problems quickly without rebuilding all the code and docker containers.

No problem, you know what to do. You make another shared config structure:

```rust
#[derive(Clone, Debug, Parser)]
pub struct HttpClientConfig {
    #[clap(long, env)]
    pub url: String,
    #[clap(long, env)]
    pub max_retries: u32,
    #[clap(long, env)]
    pub min_backoff: Duration,
    ...
}
```

Then where previously you just had simple URLs in your config, you try putting this:

```rust
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Config {
    /// Socket to listen on for http traffic
    #[clap(long, env = "LISTEN_ADDR", default_value = "127.0.0.1:4040")]
    pub listen_addr: SocketAddr,

    /// Auth agent
    #[clap(flatten)]
    pub auth_agent_client_config: HttpClientConfig,

    /// Friend service
    #[clap(flatten)]
    pub friend_service_client_config: HttpClientConfig,

    /// Buddy service
    #[clap(flatten)]
    pub buddy_service_client_config: HttpClientConfig,

    /// Pal service
    #[clap(flatten)]
    pub pal_service_client_config: HttpClientConfig,

    ...
}
```

The problem you run into immediately is, when you flatten `HttpClientConfig` four times,
you end up with four `url` fields, four `max_retries` fields, etc. and `clap` considers this
ambiguous and panics.

No problem, you think, I'll fix that name collision by prefixing. There's surely a way that I can compose all this
config that I need in a way that will work.

Unfortunately, prefixing before flattening is a feature of `clap-derive` that has been requested for years and appears
to be impossible without significant changes.

[clap-3117](https://github.com/clap-rs/clap/issues/3117)
[clap-3513](https://github.com/clap-rs/clap/issues/3513)
[clap-5050](https://github.com/clap-rs/clap/issues/5050#issuecomment-1659413560)

So unfortunately, flattening is just not going to work out for you here.

Scrappy engineer that you are, you come up with another solution:

"Instead of using a structure for `HttpClientConfig`, I'll stuff it all into one string, the URL, and any additional config will become query parameters, so my CLI parameter might look like
 `--friend-service-client-config=http://foo.service?max_retries=5&min_backoff=20`"

This has some merit as a quick fix in this particular case, and will probably work well until you get to the point where this config gets large / complicated. For instance, you need to associate an RSA key
and do mTLS. Even if you feel you can tolerate these URLs becoming very long, you may have further deployment constraints. Suppose you are deploying in kubernetes. This RSA key may be a secret, and the way
kubernetes manages secrets is exclusively by setting environment variables. If your idea is to stuff the RSA key into the URL, and the RSA key is secret, then the whole URL is going to become a secret.
But that may be very inconvenient. The `max_retries` and `min_backoff` are things you'd like to be able to review and change easily, and it may become a lot harder if they are a secret, and there's no reason that they should be a secret.

The other common workaround I've seen is, when you get to the point of needing multiple copies of `X` in your config structure, but `clap(flatten)` isn't going to let you do that,
you represent it all as JSON instead. You collapse all the `X` parameters into one parameter, and set a `clap(value_parser)` that uses `serde_json` to parse it.

This can work okay but it can become annoying if the JSON object gets very big. Relatively few env-file parsers actually support multi-line values. Rust `dotenvy` crate [doesn't support it](https://github.com/allan2/dotenvy/issues/104) for instance.
The `docker run --env-file` parser [doesn't support it either](https://github.com/moby/moby/issues/12997). If you just accept having very long lines in your env file, it becomes harder to review diffs in git and github.
And again, it becomes annoying when parts of the JSON object need to become secret, but not all of it should become secret.

I think these techniques have their place, but your go-to approach should probably not be, creative ways to stuff multiple config values into one env parameter, just because it's hard to make your parsing library
parse the pieces as separate env values.

The larger point for me is, it happens very often that your configuration needs grow significantly as your project proceeds. It's better if this can be always as straightforward as replacing what was previously
a single field with a struct, and you aren't forced to react to this by using a more complicated serialization format to put a lot of configuration into one field or env-value, unless you're sure you want to do that.
And you also shouldn't have to change the code of every service to resolve problems like this.

# Investigations

Over time, what I've realized is that in the context of large web services, having a flatten-with-prefix feature that lets me compose config structures again and again with prefixing as needed, is probably more valuable to me than many of the other `clap-derive` features.

In a complex rust program, where you have a stack of systems and subsystems that may each require configuration, and there is not generally "life before main", you usually need to find a way to plumb all the config from main to all these various systems. (Or, if you give up on that, then you are giving up on having complete `--help` documentation for your program, and possibly on failing fast when there is a configuration problem.)

Nested flatten-with-prefix is basically the perfect tool to manage that while preserving separation of concerns across your project. It's all the conveience of [`gflags`](https://github.com/gflags/gflags), where you just declare a flag at the site where it is needed and it magically appears in all binaries that link to that code, but with none of the life-before-main nonsense.

I looked at many of the existing alternatives to `clap`. In particular I looked at all the libraries listed in [argparse-roseetta-rs](https://github.com/rosetta-rs/argparse-rosetta-rs)

(You can read more about one person's view of the [pros and cons of different arg parser crates](https://rust-cli-recommendations.sunshowers.io/cli-parser.html))

Of these, only `clap` has a `flatten` feature at all, let alone the `flatten-with-prefix` feature that I want. Many of them are motivated by simplicity, build times, or compiled code size compared to clap.

There are a number of other "config" libraries out there that I considered as alternatives:

* [envy](https://crates.io/crates/envy)
* [envconfig](https://crates.io/crates/envconfig)
* [env-config](https://crates.io/crates/env-config)
* [conf_from_env](https://crates.io/crates/conf_from_env)
* [structconf](https://crates.io/crates/structconf)
* [figment](https://crates.io/crates/figment)
* [config](https://crates.io/crates/config)
* [lazy_conf](https://docs.rs/lazy_conf/0.1.1/lazy_conf/)

None of these seemed like they were going to meet my needs.

* `envy` tries to use `serde::Deserialize` rather than create it's own proc macro, which is very KISS. But I think in practice it's not going to work that well if you try to flatten a lot of structures together.
  `#[serde(flatten)]` has several known bugs which you can read about in the serde issue tracker which have been open for years. It works fine for small `serde_json` examples but in more complicated examples it has confusing / broken behavior.
  That makes me nervous. `figment` is similarly based on `serde::Deserialize`. (Sure enough, you can find issues on their tracker [like this one](https://github.com/SergioBenitez/Figment/issues/80)). `config-rs` also relies on `serde` in this way.
* I also think that in an env / config crate based on `serde::Deserialize`, you aren't going to be able to do error handling in the most helpful way, where you report all the configuration problems and not just the first one you encounter.
  That's very important when your deployment cycles take a long time, and unfortunately that's just not how the `serde::Deserialize` derive macro works.
* My testing of `envy 0.4.2`, `config 0.14.0`, and `figment 0.10.19` showed that indeed, as suspected, they can only report one missing or invalid value at a time. There doesn't appear to be any way that I as a user could improve this, or any way that they, as custom deserializer implementers, could change the behavior and report all the missing or invalid values at once. This is just a limitation of using `serde` for this purpose.
* `envconfig`, `env-config`, `conf_from_env` all provide proc macros of their own, but of these only `envconfig` has a flatten feature (at time of writing). It doesn't have a flatten-with-prefix feature though, and it doesn't do any kind of auto-generated help / discoverability for the `env` read by the final program.
* The other crates that I looked at don't seem to address my problem.

Speaking from my own experience, when you have a *large* web project with *a lot* of config, you can easily get into a situation where there are more than 10 different problems with the config (missing env values, misspelled or wrong env names, invalid json blobs, etc. etc.). It could be caused by simple mistakes, or by adding new features that add a lot of config, or refactoring helm templates, or changes to the underyling infrastructure that have unexpected consequences.

If I have to deploy 10 times to see 10 different problems and fix them, for me that is a non-starter if I'm working on a large project. So this ruled out all the crates that use `serde` as the interface to use config structs.

If I'm working on a smaller project that doesn't actually have that much config, then there are fewer things that can go wrong at once. Or if the config only changes very rarely for some reason, then I'm less likely to have this problem. In those cases this issue is much less of a concern.

A significant part of my thinking in choosing any of these crates was that I have several large web projects that are already using `clap-derive` to manage the config. The reason that I wanted to change was that I am running up against limitations of `clap-derive`, but changing to a completely different library based on `serde` or something would be a very labor-intensive and high-risk migration. I wanted to be sure that if I was going to spend the time to change to something radically different, it is highly likely that I'm going to end up with something that I'm very happy with.

## More alternatives

There were a few more alternative approaches that engineers have come up with once they hit limitations of `clap-derive` in a large project.

This one caught my eye, from [clap issue 3513 discussion](https://github.com/clap-rs/clap/issues/3513#issuecomment-2105359985)

* [clap_wrapper](https://github.com/wfraser/clap_wrapper)

This is a very clever approach -- this is another proc macro which you are supposed to use in concert with `clap-derive`, but before `clap-derive` runs, it intercepts and modifies the `#[clap(...)]` attributes in order to implement prefixing *on the struct at hand*, but not at the site of flattening. So similarly to this crate, it finds a way to make prefixing possible without throwing out all of `clap`, and also changes some defaults while we're at it, while hopefully not being too distruptive. This approach is not something that I had previously considered -- actually I've never used a proc-macro crate like that before, that modifies the arguments to another proc macro. What's nice about this is that you aren't giving up any of the `clap-derive` features to use this. The problem for me is, I'm worried that it will become very tricky to debug, and also, it doesn't actually let me prefix at the site of flattening, which is what I need to resolve the kinds of conflicts that I have encountered often.

In that same thread, another clap user describes how they [use declarative macros to instantiate their clap-derive structs with prefixes](https://github.com/clap-rs/clap/issues/3513#issuecomment-1344372578) as a workaround for the lack of prefixing.
This is also a clever workaround, but my feeling is that this is stuff that a proc macro should be doing for you, and that it will be more maintainable that way.

# Solution

I decided that my best path forward was to write the library that I was looking for: a new env-and-argument parser library with an interface similar to `clap-derive`, but where the traits and internals are structured such that `flatten-with-prefix` is easily implemented, and with stronger support for `env` generally.

I cut scope drastically in order to make the goal achieveable. I decided to only offer a `derive` macro to minimize API surface area. I chose to cut many features that I have never used in a web service such as subcommands and positional arguments. These things only really make sense when at least some of the config is happening via CLI args, since environment variables are not ordered.

At some point I had built a first draft, working on and off in spare time. Eventually it got to the point where I could play with large examples and see how it felt, and particularly, see if I would actually feel good about migrating a large project
that was using `clap-derive` to use this library instead.

At some point I had a realization: I would be much better off using `clap::Builder` under the hood rather than building my own parser and error rendering, and I would not have to give up much of anything. The way I had already structured things, this was a relatively easy change. The goal of the project became about building an alternative derive macro that used `clap` under the hood for CLI args, but supported flatten-with-prefix, and other improvements around `env`, rather than building a completely new argument parser (and help rendering, which clap is very good at). This was different from what I expected, because I had thought that there would be changes needed in the `builder` side and not only the `derive` side of `clap` to implement flatten-with-prefix, but it turned out not to really be the case. This ultimately saved a lot of work to get to a minimum viable state.

The initial feature set was the features of `clap-derive` I had used most heavily in the past, and I tried to keep very similar syntax and behavior for these features, plus flatten with prefix.

* Support for `flags` (on or off), `parameters` (take a value, either next arg or using `--flag=value` syntax), and `repeat` options (what `clap` calls "multi" options, which can appear several times and the results get aggregated).
* Support for long form (`--switch`) and short form (`-s`) switches, and env variable association
* Parse into user-defined types using `FromStr`, but allow overriding this by specifying `value_parser`.
* Doc strings become help strings
* [Infer intent from value type](https://docs.rs/clap/latest/clap/_derive/index.html#arg-types). `bool` indicates a flag, non-bool indicates a parameter. `Option<T>` indicates that it's not required to appear.
  * In this crate though, the assumptions based on type can always be overidden easily, by saying what option kind it is in the proc-macro attribute. In `clap`, I've seen developers get stuck when they have a `Vec<T>` argument and they are
    trying to get clap to parse it from a JSON string by setting `value_parser`, but they don't realize that clap has inferred that it's a multi-option because `Vec` is used, and the semantic of `value_parser` is different now too, which
    throws them off, and the error messages they get about JSON failing to parse are hard for them to make sense of.
  * In this crate, I decided that you should have to write `repeat` explicitly to get a multi-option.
  * There are some other features here like `Option<Option<T>>` that I like in principle, but didn't make the cut for MVP.
* Default flag names and env names based on the field name.
* The syntax for `flatten` is very similar, but it now supports additional options like `prefix`, `env_prefix`, `long_prefix`, `help_prefix`, which can be defaulted or explicitly set.

I ended up adding more features besides this before the first `crates.io` release as I started migrating more of my projects to this, and encountered things that were either harder to migrate, or were just additional features that I realized I wanted and could fit into the framework cleanly with relative ease.

## Testing

If we change the same simple program that we used for testing `clap-derive` to use `conf` instead, we can see that the error handling in these scenarios becomes better.

```

```

It always reports two problems if there are problems with two different parameters, even if it is a combination of missing and invalid values, and whether env is involved or args are involved.

That may come as a surprise. `conf` uses `clap` to do all of the argument parsing, so how could it have more complete error reporting than `clap` when only args are involved?

The answer comes in how we use `clap`. Because `conf` does not use `clap` to handle any `env` (as `clap-derive` does), we can't let the clap parser make any determinations about when a required arg was not found,
because it won't know if it's later going to be considered found because of an `env` value. Instead we have to tell it that all arguments are optional even if they are required from the user's point of view.
For the same reason, we can't pass any `value_parser` to `clap`, we can only use it to parse strings. This also prevents it from early-returning if a single `value_parser` fails.
Once clap has parsed the args as optional strings, then we walk the target structure and try to parse values into it. We encounter missing and invalid value errors at more or less the same time,
so it's easy for us to give a complete error report, even if `clap` would not have.

# Future directions

My hope is that others will find this project useful or interesting, and contribute any bug reports, reports of behaviors they find confusing, patches, and so on, to help the project reach maturity.

Note that just because this crate doesn't have a feature like subcommands now doesn't mean that I am opposed to that feature -- patches are welcome, and I can certainly see use-cases.
For example, you may have some targets that are web services, and some that are associated command-line tools, and you may want to be able to share a bunch of config structures between them.

But it's important to understand that I created this crate primarily to *serve an underserved niche*, which is large 12-factor app web projects, and not to try to achieve feature parity with `clap-derive`.
`clap` is at this point an enormous library and I'm sure that it has many very useful features that I'm still not aware of after years of use, and many if not all are exposed through `clap-derive` somehow it seems.
If you have rather complex or specific requirements around CLI argument parsing, then you should probably be using `clap` directly and not this crate, because this crate is more oriented towards `env` anyways.
We would need considerably more developer / maintainer energy than I am willing to commit to in order to realize a more ambitious vision.

Additionally, in my view, optimizing parsing time or code size should not be a major development goal of this crate, because it's very unlikely to have a noticeable benefit for a web service.
There are [half a dozen libraries](https://github.com/rosetta-rs/argparse-rosetta-rs) that have parsing time and code size as goals, which you can use in situations where that's more important.
That's not to say that I won't take patches that refactor to avoid copies and allocations during parsing and such, but if a patch like that has negative impact on readability of the code, or ease of developing interesting features in the future,
then it requires more justification. I have made some minimal efforts to avoid needless copies, but as long as performance is comparable to `clap-derive`, then I think users in the targetted niche will be happy.

Another feature that I noticed in many `config` libraries is special support for reading config from files in various formats. From my experience using `clap-derive`, the simplest way to handle this is to write a `value_parser` that opens a file and parses it. This can usually be a one liner if you want it to be.

```rust
fn read_json_file<T>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(std::fs::read_to_string(path)?)?)
}
```

This is good because it's very simple and very configurable -- if you decide that you'd rather use `serde_json_lenient::from_str` instead of `serde_json::from_str`, it's easy for you to change to that and you aren't tied to my choices of libraries or versions.

I think it might make sense to make an "extras" crate that contains "common" or "popular" value parsers. One nice thing about that is that those value parsers could also be used with `clap`.
Right now, I don't think that *this* crate should contain any code that reads a file. I haven't seen a compelling reason that that's necessary, and I see good reasons to try to separate concerns.

Personally, I do like using a crate like [`dotenvy`](https://crates.io/crates/dotenvy) to load `.env` files before parsing the config, as described in `README.md`.

My belief is that by keeping the API surface area relatively small and staying focused on the target niche, we can make sure that it stays as easy as possible to add useful features, test them appropriately, and drive the project forwards. I do believe that building on `clap` is the best course in terms of conserving developer energy and serving the users the best.
