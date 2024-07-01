Motivating use case
-------------------

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
to be impossible without radical changes to `clap`.

[clap-3117](https://github.com/clap-rs/clap/issues/3117)
[clap-3513](https://github.com/clap-rs/clap/issues/3513)
[clap-5050](https://github.com/clap-rs/clap/issues/5050#issuecomment-1659413560)

Most likely there would have to be large changes to the APIs of the builder part of `clap` so that it can keep track of
prefixing and such. It's not something that can be done only in the derive side of things.
So unfortunately, flattening is just not going to work out for you here.

Scrappy engineer that you are, you come up with another solution:

"Instead of using a structure for HttpClientConfig, I'll stuff it all into one string, the URL, and any additional config will become query parameters, so my CLI parameter might look like
 `--friend-service-client-config=http://foo.service?max_retries=5&min_backoff=20`"

This has some merit as a quick fix in this particular case, and will probably work well until you get to the point where this config gets large / complicated. For instance, you need to associate an RSA key
and do mTLS. Even if you feel you can tolerate these URLs becoming very long, you may have further deployment constraints. Suppose you are deploying in kubernetes. This RSA key may be a secret, and the way
kubernetes manages secrets is exclusively by setting environment variables. If your idea is to stuff the RSA key into the URL, and the RSA key is secret, then the whole URL is going to become a secret.
But that may be very inconvenient. The `max_retries` and `min_backoff` are things you'd like to be able to change easily, and it may become a lot harder if they are a secret, and there's no reason they should be a secret.

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

Solution
--------

Over time, what I've realized is that in the context of web services, having a flatten-with-prefix feature that is flexible, lets me compose config structures again and again with prefixing as needed, and works as one intuitively expects, is probably more valuable to me than any of the other `clap` features.

In a complex rust program, where you have a stack of systems and subsystems that may each require configuration, and there is not generally "life before main", you usually need to find a way to plumb all the config from main to all these various systems. (Or, if you give up on that, then you are giving up on having complete `--help` documentation for your program, and possibly on failing fast when there is a configuration problem.)

Nested flatten-with-prefix is basically the perfect tool to manage that while preserving separation of concerns across your project. It's all the conveience of [`gflags`](https://github.com/gflags/gflags), where you just declare a flag at the site where it is needed and it magically appears in all binaries that link to that code, but with none of the life-before-main nonsense.

I looked at many of the existing alternatives to `clap`. In particular I looked at all the libraries listed here: https://github.com/rosetta-rs/argparse-rosetta-rs

(You can read more about one person's view of the pros and cons of different arg parser crates: https://rust-cli-recommendations.sunshowers.io/cli-parser.html)

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
  That makes me nervous. `figment` is similarly based on `serde::Deserialize`. I also think that in an env crate based on `serde::Deserialize`, you aren't going to be able to do error handling in the most helpful way,
  where you report all the configuration problems and not just the first one you encounter.
  That's important for productivity when your deployment cycles take a long time, and unfortunately that's just not how the `serde::Deserialize` trait works.
* `envconfig`, `env-config`, `conf_from_env` all provide proc macros of their own, but of these only `envconfig` has a flatten feature (at time of writing). It doesn't have a flatten-with-prefix feature though, and it doesn't do any kind of auto-generated help / discoverability for the `env` read by the final program.
* The other crates that I looked at don't seem to address my problem.

Also, I was uncomfortable with setting arg parsing entirely out of scope, because it seems to me that you really want your service to have a `--help` option of some kind that documents all its config, including the environment variables, the way that `clap` does. But usually the arg parsers become responsible for generating that help, so if your `env` handling doesn't hook into that tightly somehow, then I don't see how it's going to work. Also, it can be legitimately very useful to be able to pass config as CLI arguments as an alternative to env, when developing locally or running in CI. It's also useful to be able to do this knowing that CLI parameters will shadow values in `env`.

I decided that my best path forward was to write the library that I was looking for: a new env-and-argument parser library similar to `clap`, but where the traits and internals are structured such that `flatten-with-prefix` is easily implemented.

I decided not to support the "builder" interface at all, and only offer a `derive` macro. My experience is that the derive macros cut out a lot of boilerplate and are easily understandable,
and whatever control I've given up by using them has not turned out to be important later.

I chose to cut many features that I have never used in a web service such as subcommands and positional arguments.
These things only really make sense when at least some of the config is happening via CLI args, since environment variables are not ordered.

I chose to keep many features of `clap` that I used heavily in the past, and I tried to keep very similar syntax and behavior for these features.

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

I built a first draft in spare time. Eventually it got to the point where I could play with large examples and see how it felt, and particularly, see if I would actually feel good about migrating a project
that was using `clap` to use this library instead.

At some point I considered it minimum-viable. Then I added regression tests, iterated a bit on improving error messages, and started using it in a medium-size project that had been using `clap`.

Future directions
-----------------

My hope is that others will find this project interesting, and contribute any bug reports, reports of behaviors they find confusing, patches, and so on, to help the project reach maturity.

Note that just because this crate doesn't have a feature like subcommands now doesn't mean that I am opposed to that feature -- patches are welcome, and I can certainly see use-cases.
For example, you may have some targets that are web services, and some that are associated command-line tools, and you may want to be able to share a bunch of config structures between them.

But it's important to understand that I created this crate primarily to *serve an underserved niche*, which is large 12-factor app web services, and not to try to achieve feature parity with `clap`.
`clap` is at this point an enormous library and I'm sure that it has many very useful features that I'm still not aware of after years of use.
If you have rather complex or specific requirements around CLI argument parsing, then you should probably be using `clap` and not this crate, because this crate is more oriented towards `env` anyways.
We would need considerably more developer / maintainer energy than I am willing to commit to in order to realize a more ambitious vision.

Additionally, in my view, optimizing parsing time or code size should not be a major development goal of this crate, because it's very unlikely to have a noticeable benefit for a web service.
There are half a dozen libraries linked to above that have parsing time and code size as goals, which you can use in situations where that's more important.
That's not to say that I won't take patches that refactor to avoid copies and allocations during parsing and such, but if a patch like that has negative impact on readability of the code, or ease of developing interesting features in the future,
then it requires more justification. I have made some minimal efforts to avoid needless copies, but as long as performance is comparable to `clap`, then I think users in the targetted niche will be happy.

Another feature that I noticed in many `conf` libraries is special support for reading config from files in various formats. From my experience using `clap`, the simplest way to handle this is to write a `value_parser` that opens a file and parses it. This can usually be one line if you want it to be, for example:

 `value_parser = |path: &str| { serde_json::from_str(&std::fs::read_to_string(path).unwrap()) }`

I think it might make sense to make an "extras" crate that contains "common" or "popular" value parsers. One nice thing about that is that those value parsers could also be used with `clap`.
I don't think that *this* crate should contain any code that reads a file. For me, that is a separation of concerns thing.

Personally, I do like using a crate like [`dotenvy`](https://crates.io/crates/dotenvy), which can load a `.env` file and set values to `env` right before you do `Config::parse()`. That `.env` file can be checked into git which helps local development, and you can leave it out when you go to build a docker container. I suppose that some of the value that some developers seem to be getting from reading config from an `.ini` file or such, I am getting that way instead.

My belief is that by keeping the API surface area relatively small and staying focused on the target niche, we can make sure that it stays as easy as possible to add useful features, test them appropriately, and drive the project forwards in a way that serves the users best.
