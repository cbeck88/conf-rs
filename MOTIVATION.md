# Motivation

This document discusses the motivations for `conf` and how they drove design decisions.

Additionally, it has to answer the question "why a new crate?" when there are other mature crates out there in the same genre.

In order to answer this question, this document has to be *opinionated*. It introduces many value judgments and my own opinions,
and *reasonable people may disagree*. This situation is very similar to [`jiff/DESIGN.md`](https://github.com/BurntSushi/jiff/blob/master/DESIGN.md#the-api-design-rationale-for-jiff)
and all the same caveats apply.

Particularly, the value judgments and opinions here ultimately work to justify an alternative, and so they tend to be oriented towards the technical shortcomings
of other crates, as I perceive them.

Similarly to the story in `jiff/DESIGN.md`, ultimately I perceived that the crates in this genre, as a whole, had reached a local maximum and were unlikely to be able to rapidly improve in the ways that were important to me. So it appeared that there was a niche to be filled, and `conf` attempts to fill it.

For an application developer, it may be hard to believe that things like, "add a prefix to a group of strings", "allow using a custom function instead of `std::env::vars_os`", or "report as many errors as possible", can be in this
category of things that cannot be easily improved in a crate, let alone, many crates in the genre. *All I can say is, read on*. As we'll see, it turns out that many crates in this genre made architectural decisions, and decisions about what their public API is, that made one or more of these things impossible without large amounts of rework and breaking changes to their public API, and so they are limited in what they can achieve here.

Above all, please understand that *the purpose of this document is not to criticize other crates*. The discussion is grounded in practical concerns, pros and cons from a technical point of view, and my own
efforts to make engineering decisions as a user of crates in this space. The document can *help potential users* of `conf` *rapidly build a mental model* of how initial design decisions in `conf` were made and how `conf` might evolve in the near future. This can *help users decide* whether or not `conf` is the right tool for the job in their situation, or if another tool is more appropriate.

In fact, I believe that many of these crates are very well-engineered overall, and just not the best choice for the use-cases that I have in mind. Maintainers and developers of these crates that are saying no to features right now that are important to users like me, are also doing the right thing, given where their projects are now, what niche they are aimed at, and how many users they have now who would be impacted by breaking changes. Please understand that I have only the greatest respect for everyone involved with any of the projects mentioned specifically.

* [Motivating use case](#motivating-use-case)
* [Investigations](#investigations)
  * [More alternatives](#more-alternatives)
  * [Testing interlude](#testing-interlude)
* [Solution](#solution)
  * [Testing](#testing)
* [Future directions](#future-directions)

# Motivating use case

Suppose that you have a web app which consists of 8 microservices (and counting).
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

If I have to deploy 10 times to see 10 different problems and fix them, for me that is a non-starter if I'm working on a large project. So this ruled out all the crates that use `serde` as the interface to the config structs.

If I'm working on a smaller project that doesn't actually have that much config, then there are fewer things that can go wrong at once. Or if the config only changes very rarely for some reason, then I'm less likely to have this problem. In those cases this issue is much less of a concern.

A significant part of my thinking in choosing any of these crates was that I have several large web projects that are already using `clap-derive` to manage the config. The reason that I wanted to change was that I am running up against limitations of `clap-derive`, but changing to a completely different library based on `serde` or something would be a very labor-intensive and high-risk migration. I wanted to be sure that if I was going to spend the time to change to something radically different, it is highly likely that I'm going to end up with something that I'm very happy with.

## More alternatives

There were a few more alternative approaches that engineers have come up with once they hit limitations of `clap-derive` in a large project.

This one caught my eye, from [clap issue 3513 discussion](https://github.com/clap-rs/clap/issues/3513#issuecomment-2105359985)

* [clap_wrapper](https://github.com/wfraser/clap_wrapper)

This is a very clever approach -- this is another proc macro which you are supposed to use in concert with `clap-derive`, but before `clap-derive` runs, it intercepts and modifies the `#[clap(...)]` attributes in order to implement prefixing *on the struct at hand*, but not at the site of flattening. So similarly to this crate, it finds a way to make prefixing possible without throwing out all of `clap`, and also changes some defaults while we're at it, while hopefully not being too distruptive. This approach is not something that I had previously considered -- actually I've never used a proc-macro crate like that before, that modifies the arguments to another proc macro. What's nice about this is that you aren't giving up any of the `clap-derive` features to use this. The problem for me is, I'm worried that it will become very tricky to debug, and also, it doesn't actually let me prefix at the site of flattening, which is what I need to resolve the kinds of conflicts that I have encountered often.

In that same thread, another clap user describes how they [use declarative macros to instantiate their clap-derive structs with prefixes](https://github.com/clap-rs/clap/issues/3513#issuecomment-1344372578) as a workaround for the lack of prefixing.
This is also a clever workaround, but my feeling is that this is stuff that a proc macro should be doing for you, and that it will be more maintainable that way.

## Testing interlude

Many readers may be surprised at my conclusions about use of `serde` above, and the limitations that that will create around error reporting. It seems to be widely believed among rust users that this is not a limitation of serde.

See for example [this reddit thread](https://www.reddit.com/r/rust/comments/1bjc7tp/getting_all_serde_errors_at_once/):

question:

>  Getting all serde errors at once
>
> Currently serde bails out on the first error, once I fix that it throws up the next one and so on. How can I get all the errors at once?

answer:

>  The various serde "dialects" (json, yaml, et c) do not support recovery. Particularly when deserializing data, once the deserialization encounters a structural error, e.g. a missing quote, it would have to guess what is missing in order to be able to continue. Modern source code parsers recover in such situations so as to be able to point out multiple syntax errors at once, but serialization libs typically do not.

answer:

>  The first error of what? What are you "fixing"? serde is a framework for (de)serializing various data formats from/to Rust. It by itself doesn't do anything. Every parser is different and may handle errors differently depending on the format being parsed.

I've seen similar remarks in rust forums (but I can't find those links now) -- many users believe that "serde is just a framework" and deserializers are in total control around error reporting.

To make things very concrete, here's a test program based on the example code from `envy 0.4.2` documentation.

```rust
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Config {
    foo: u16,
    bar: Option<bool>,
    baz: u32,
    boom: Option<u64>,
}

fn main () {
    match envy::from_env::<Config>() {
        Ok(config) => println!("{:#?}", config),
        Err(error) => eprintln!("{:#?}", error),
    }
}
```

Here's some example behavior of the test program:

```shell
$ ./target/debug/envy-test
MissingValue(
    "foo",
)
$ FOO=1 ./target/debug/envy-test
MissingValue(
    "baz",
)
$ FOO=1 BAZ=2 ./target/debug/envy-test
Config {
    foo: 1,
    bar: None,
    baz: 2,
    boom: None,
}
$ FOO=1 BAZ=-2 ./target/debug/envy-test
Custom(
    "invalid digit found in string while parsing value '-2' provided by BAZ",
)
$ FOO=-1 BAZ=-2 ./target/debug/envy-test
Custom(
    "invalid digit found in string while parsing value '-2' provided by BAZ",
)
$ FOO=-1 ./target/debug/envy-test
Custom(
    "invalid digit found in string while parsing value '-1' provided by FOO",
)
```

As you can see, it won't report more than one error at a time when multiple environment variables have a problem, even though it could in principle.
You can write similar programs using `config` and `figment` and get similar results at the revisions that I tested.

To investigate what it would take to change `envy` to improve this and collect all the errors, let's use `cargo expand` to look at what code the `serde::Deserialize` derive-macro
is generating for the user-defined structures here. The code is pretty short so the output is only about 200 lines.

```rust
#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use serde::Deserialize;
struct Config {
    foo: u16,
    bar: Option<bool>,
    baz: u32,
    boom: Option<u64>,
}
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for Config {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> _serde::__private::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __field1,
                __field2,
                __field3,
                __ignore,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private::Ok(__Field::__field0),
                        1u64 => _serde::__private::Ok(__Field::__field1),
                        2u64 => _serde::__private::Ok(__Field::__field2),
                        3u64 => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "foo" => _serde::__private::Ok(__Field::__field0),
                        "bar" => _serde::__private::Ok(__Field::__field1),
                        "baz" => _serde::__private::Ok(__Field::__field2),
                        "boom" => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"foo" => _serde::__private::Ok(__Field::__field0),
                        b"bar" => _serde::__private::Ok(__Field::__field1),
                        b"baz" => _serde::__private::Ok(__Field::__field2),
                        b"boom" => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private::PhantomData<Config>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = Config;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "struct Config")
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match _serde::de::SeqAccess::next_element::<
                        u16,
                    >(&mut __seq)? {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct Config with 4 elements",
                                ),
                            );
                        }
                    };
                    let __field1 = match _serde::de::SeqAccess::next_element::<
                        Option<bool>,
                    >(&mut __seq)? {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    1usize,
                                    &"struct Config with 4 elements",
                                ),
                            );
                        }
                    };
                    let __field2 = match _serde::de::SeqAccess::next_element::<
                        u32,
                    >(&mut __seq)? {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    2usize,
                                    &"struct Config with 4 elements",
                                ),
                            );
                        }
                    };
                    let __field3 = match _serde::de::SeqAccess::next_element::<
                        Option<u64>,
                    >(&mut __seq)? {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    3usize,
                                    &"struct Config with 4 elements",
                                ),
                            );
                        }
                    };
                    _serde::__private::Ok(Config {
                        foo: __field0,
                        bar: __field1,
                        baz: __field2,
                        boom: __field3,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private::Option<u16> = _serde::__private::None;
                    let mut __field1: _serde::__private::Option<Option<bool>> = _serde::__private::None;
                    let mut __field2: _serde::__private::Option<u32> = _serde::__private::None;
                    let mut __field3: _serde::__private::Option<Option<u64>> = _serde::__private::None;
                    while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private::Option::is_some(&__field0) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("foo"),
                                    );
                                }
                                __field0 = _serde::__private::Some(
                                    _serde::de::MapAccess::next_value::<u16>(&mut __map)?,
                                );
                            }
                            __Field::__field1 => {
                                if _serde::__private::Option::is_some(&__field1) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("bar"),
                                    );
                                }
                                __field1 = _serde::__private::Some(
                                    _serde::de::MapAccess::next_value::<
                                        Option<bool>,
                                    >(&mut __map)?,
                                );
                            }
                            __Field::__field2 => {
                                if _serde::__private::Option::is_some(&__field2) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("baz"),
                                    );
                                }
                                __field2 = _serde::__private::Some(
                                    _serde::de::MapAccess::next_value::<u32>(&mut __map)?,
                                );
                            }
                            __Field::__field3 => {
                                if _serde::__private::Option::is_some(&__field3) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("boom"),
                                    );
                                }
                                __field3 = _serde::__private::Some(
                                    _serde::de::MapAccess::next_value::<
                                        Option<u64>,
                                    >(&mut __map)?,
                                );
                            }
                            _ => {
                                let _ = _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)?;
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private::Some(__field0) => __field0,
                        _serde::__private::None => {
                            _serde::__private::de::missing_field("foo")?
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::__private::Some(__field1) => __field1,
                        _serde::__private::None => {
                            _serde::__private::de::missing_field("bar")?
                        }
                    };
                    let __field2 = match __field2 {
                        _serde::__private::Some(__field2) => __field2,
                        _serde::__private::None => {
                            _serde::__private::de::missing_field("baz")?
                        }
                    };
                    let __field3 = match __field3 {
                        _serde::__private::Some(__field3) => __field3,
                        _serde::__private::None => {
                            _serde::__private::de::missing_field("boom")?
                        }
                    };
                    _serde::__private::Ok(Config {
                        foo: __field0,
                        bar: __field1,
                        baz: __field2,
                        boom: __field3,
                    })
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &["foo", "bar", "baz", "boom"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "Config",
                FIELDS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<Config>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};
#[automatically_derived]
impl ::core::fmt::Debug for Config {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field4_finish(
            f,
            "Config",
            "foo",
            &self.foo,
            "bar",
            &self.bar,
            "baz",
            &self.baz,
            "boom",
            &&self.boom,
        )
    }
}
fn main() {
    match envy::from_env::<Config>() {
        Ok(config) => {
            ::std::io::_print(format_args!("{0:#?}\n", config));
        }
        Err(error) => {
            ::std::io::_eprint(format_args!("{0:#?}\n", error));
        }
    }
}

```

At a high level, implementing `serde::Deserialize` means

1. Taking a `Deserializer` as an argument, and calling a function on it (in this case, `deserialize_struct`)
2. To do that, one has to construct an appropriate visitor and pass it to the deserializer. That's `__Visitor<'de>` above.
3. There is a helper visitor in the implementation of that visitor, which is called `__FieldVisitor<'de>`, and a helper enum called `__Field`.

One thing you'll notice right away is that `__Visitor` implements only two functions out of trait `serde::de::Visitor`. These are `visit_seq` and `visit_map`.

And, both of these functions use `?` operator to perform early returns, if there are unknown fields or duplicate fields, if getting the type of value expected fails, or if there are missing fields. So, the fail-on-first-error behavior
is happening within the serde-derive proc macro, in the generated code for the user-defined structures, and not in the code for the `envy` crate.

If `envy` wanted to change their library's behavior, such that `./targed/debug/envy-test` will report both `foo` and `baz` missing, what could they do?

* Maybe, they could change their deserializer so that it doesn't call `visit_map` or `visit_seq` on the visitor. However, in the serde framework, this visitor that is passed to their
  deserializer is the only handle they have to the user-defined type at all. They have to call one of these two functions if they want to return the user-defined type.
* If the `envy` deserializer DOES call these functions and they don't want early returns, they will have to work around the fact that the `serde` generated code has early returns. But, they have a lot of control
  around what actually happens, because they pass whatever `impl` of `serde::de::MapAccess` they want. So maybe, when `visit_map` implementation calls `MapAccess::next_value::<T>(...)?` they can pretend there it was successful even if there was an error. For example, it's conceivable that the deserializer has its own internal mutable buffer of errors. Then, in the implementation of `MapAccess`, if an error happens, they push it on this buffer, and try to return a fake success to `serde`, so that this loop within `serde`'s generated impl of `visit_map` will keep running and they will have a chance to collect all the errors. At the very end of `deserialize_struct`, they can check if there are any errors in the buffer and return all of them. If there are none then they know they generated a correct struct without any junk values in it.
* Unfortunately, if you try to work through the details, returning a fake success doesn't really work. The API that they have to satisfy is `Result<T, E>` in a generic context. So you can't return success without actually producing a value of type `T`. In a generic context, that's not going to work, where `T` is a user-defined type, `T` may not implement default or any similar trait. Even if it did implement `Default`, `serde` doesn't give you that as a trait bound on `T`, so you can't use it.
  * Thinking outside the box, maybe you could use unsafe code here, and try to return `std::mem::uninitialized::<T>()`. As long as you put an error in your buffer, even if a temporary struct contains some uninitialized data, you won't ever have to return it to the user, since you'll return errors instead. You'd only be returning the uninitialized data to serde internals, which eventually returns to your own code before anything actually goes back to the user. So maybe there's a way to create a safe and sound deserializer implementation on top of that -- this approach might work out in `C` code for example, depending on specifics. Unfortunately, `std::mem::uninitialized` will definitely create undefined behavior in the generated `serde` code above per rust language rules, and `std::mem::uninitialized` is actually deprecated and slated to be removed. You can't use the replacement `std::mem::MaybeUninit<T>` here, because that's not the type signature that you have to satisfy. There's no sound way for you to generate a `T` given the trait bounds available to you if deserializing a `T` actually failed.
* If we give up on returning a fake success when there's an error, then whenever a field is missing or invalid, `MapAccess` or similar has to return an error immediately. But then you are giving up on returning the errors for the later fields that may have problems. So, calls to `visit_map` and `visit_seq` have to fail fast on the first error.
* Even if `visit_map` and `visit_seq` have to fail fast, it's not clear that `deserialize_struct` does. Maybe it could try to call them more times and collect different errors. But, that's not the way the framework is intended to be used, and it turns out that all the `de::Visitor` functions have a signature that takes `self` and not `&self` or `&mut self`, presumably because it creates better code-gen, or makes it easier to implement visitors if they are statically guaranteed to be one-time-use. Visitors passed to `deserialize_struct` are not `Clone`, so the only thing you can do as a `Deserializer` is try to visit once. After that you've consumed your handle to the user-defined type and there's no way you could get more information or errors.

This analysis shows that, while it's true that `serde` is a framework and doesn't itself deserialize anything, it's still a tool with opinions and limitations, especially where the derive macro is concerned. Because a bunch of the error-handling code for deserializing user-defined structures is defined by the derive macros, and not by the deserializer implementations, the deserializer is not actually in total control of the behavior. To work within the serde framework, when users are using `derive(Deserialize)`, it has to fail-fast on the first error. (If the users don't use `derive(Deserialize)`, then they can implement all this differently, and the error handling could be different in theory. But if all the users have to do that to use your crate effectively, then a lot of value proposition of `serde` here is lost.)

So, it should come as no surprise that `figment` and `config` similarly can't report all the config errors when reading the configuration fails. All of these crates that chose to not offer a proc-macro, and to use `serde::Deserialize` as the trait that users `derive`, will be similarly limited. I don't believe that they can fix this limitation without somehow changing the code that `serde-derive` is generating. But, `serde` is an enormous library of fundamental importance to the ecosystem, and changing it in a fundamental way is not something that I believe can happen easily. Most likely, it's not just a codegen change, most likely to do this properly the [`serde::de::Error` trait](https://rust-lang.github.io/hashbrown/serde/de/trait.Error.html) would need to gain a function that allows "combining" two `serde::de::Error` into one error, or the whole thing would just have to change to return collections of errors. Either way, that would also be disruptive.

---

For good measure we can do a similar small test for `clap`. I tested `clap` 4.5.8 using the following test program.

```rust
use clap::Parser;

#[derive(Debug, Parser)]
struct Config {
    #[arg(long, env)]
    flag: bool,

    #[arg(long, env)]
    my_val: usize,

    #[arg(long, env)]
    my_other_val: usize,
}

fn main() {
    let config = Config::parse();
    println!("{config:#?}");
}
```

`clap` performs better than the others in testing:

```shell
$ ./target/debug/clap-test
error: the following required arguments were not provided:
  --my-val <MY_VAL>
  --my-other-val <MY_OTHER_VAL>

Usage: clap-test --my-val <MY_VAL> --my-other-val <MY_OTHER_VAL>

For more information, try '--help'.

$ MY_VAL=1 ./target/debug/clap-test
error: the following required arguments were not provided:
  --my-other-val <MY_OTHER_VAL>

Usage: clap-test --my-val <MY_VAL> --my-other-val <MY_OTHER_VAL>

For more information, try '--help'.

$ MY_VAL=1 MY_OTHER_VAL=2 ./target/debug/clap-test
Config {
    flag: false,
    my_val: 1,
    my_other_val: 2,
}

$ MY_VAL=1 MY_OTHER_VAL=-2 ./target/debug/clap-test
error: invalid value '-2' for '--my-other-val <MY_OTHER_VAL>': invalid digit found in string

For more information, try '--help'.

$ MY_VAL=-1 MY_OTHER_VAL=-2 ./target/debug/clap-test
error: invalid value '-1' for '--my-val <MY_VAL>': invalid digit found in string

For more information, try '--help'.

$ MY_VAL=-1 ./target/debug/clap-test
error: invalid value '-1' for '--my-val <MY_VAL>': invalid digit found in string

For more information, try '--help'.

$ MY_OTHER_VAL=-2 ./target/debug/clap-test
error: invalid value '-2' for '--my-other-val <MY_OTHER_VAL>': invalid digit found in string

For more information, try '--help'.
```

So, clap can report multiple "missing required arguments" errors, which is better than the crates based on `serde`, but not multiple invalid values, or a mix of missing and invalid values.

It seems likely to me that this could be improved within `clap`, I'm not sure that there's a major barrier. Possibly the API for `clap::Error` would have to be changed
so that it doesn't assume that there is only one underlying `ErrorKind` if multiple kinds of errors occurred. I don't think that would be disruptive for the vast majority of users though,
most users call `Error::exit` one way or another when a `clap::Error` occurs.

# Solution

I decided that my best path forward was to write the library that I was looking for: a new env-and-argument parser library with an interface similar to `clap-derive`, but where the traits and internals are structured such that `flatten-with-prefix` is easily implemented, with stronger support for `env` generally. And, very comprehensive error reporting of config related problems.

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

I ended up adding more features besides this before the first `crates.io` release as I started migrating more of my projects to this, and encountered things that were either harder to migrate, or were just additional features that I realized I wanted and could fit into the framework with relative ease.

In version 0.1.1, we added support for subcommands.

## Testing

If we change the same simple program that we used for testing `clap-derive` to use `conf` instead, we can see that the error handling in these scenarios becomes better.

```rust
use conf::Conf;

#[derive(Debug, Conf)]
struct Config {
    #[arg(long, env)]
    flag: bool,

    #[arg(long, env)]
    my_val: usize,

    #[arg(long, env)]
    my_other_val: usize,
}

fn main() {
    let config = Config::parse();
    println!("{config:#?}");
}
```

Testing the same examples as before, we get these results:

```shell
$ ./target/debug/clap-test
error: A required value was not provided
  env 'MY_OTHER_VAL', or '--my-other-val', must be provided
  env 'MY_VAL', or '--my-val', must be provided

$ MY_VAL=1 ./target/debug/clap-test
error: A required value was not provided
  env 'MY_OTHER_VAL', or '--my-other-val', must be provided

Help:
      --my-other-val <my_other_val>
          [env: MY_OTHER_VAL]

$ MY_VAL=1 MY_OTHER_VAL=2 ./target/debug/clap-test
Config {
    flag: false,
    my_val: 1,
    my_other_val: 2,
}

$ MY_VAL=1 MY_OTHER_VAL=-2 ./target/debug/clap-test
error: Invalid value
  when parsing env 'MY_OTHER_VAL' value '-2': invalid digit found in string

Help:
      --my-other-val <my_other_val>
          [env: MY_OTHER_VAL]

$ MY_VAL=-1 MY_OTHER_VAL=-2 ./target/debug/clap-test
error: Invalid value
  when parsing env 'MY_OTHER_VAL' value '-2': invalid digit found in string
  when parsing env 'MY_VAL' value '-1': invalid digit found in string

$ MY_VAL=-1 ./target/debug/clap-test
error: A required value was not provided
  env 'MY_OTHER_VAL', or '--my-other-val', must be provided
error: Invalid value
  when parsing env 'MY_VAL' value '-1': invalid digit found in string

$ MY_OTHER_VAL=-2 ./target/debug/clap-test
error: A required value was not provided
  env 'MY_VAL', or '--my-val', must be provided
error: Invalid value
  when parsing env 'MY_OTHER_VAL' value '-2': invalid digit found in string
```

We get similar results when we use args for input instead of `env`.

```shell
$ ./target/debug/clap-test --my-val=-1
error: A required value was not provided
  env 'MY_OTHER_VAL', or '--my-other-val', must be provided
error: Invalid value
  when parsing '--my-val' value '-1': invalid digit found in string

$ ./target/debug/clap-test --my-val=-1 --my-other-val=-2
error: Invalid value
  when parsing '--my-val' value '-1': invalid digit found in string
  when parsing '--my-other-val' value '-2': invalid digit found in string

$ ./target/debug/clap-test --my-val=-1 --my-other-val 2
error: Invalid value
  when parsing '--my-val' value '-1': invalid digit found in string

Help:
      --my-val <my_val>
          [env: MY_VAL]

$ ./target/debug/clap-test --my-other-val a
error: A required value was not provided
  env 'MY_VAL', or '--my-val', must be provided
error: Invalid value
  when parsing '--my-other-val' value 'a': invalid digit found in string
```

It always reports two problems if there are problems with two different parameters, even if it is a combination of missing and invalid values, and whether env is involved or args are involved.

That may come as a surprise. `conf` uses `clap` to do all of the argument parsing, so how could it have more complete error reporting than `clap` when only args are involved?

The answer comes in how we use `clap`. Because `conf` does not use `clap` to handle any `env` (as `clap-derive` does), we can't let the clap parser make any determinations about when a required arg was not found,
because it won't know if it's later going to be considered found because of an `env` value. Instead we have to tell it that all arguments are optional even if they are required from the user's point of view.
For the same reason, we can't pass any `value_parser` to `clap`, we can only use it to parse strings. This also prevents it from early-returning if a single `value_parser` fails.
Once clap has parsed all the args as optional strings, then we walk the target structure and try to parse values into it. We encounter missing and invalid value errors at more or less the same time,
so it's easy for us to give a complete error report, even if `clap` would not have. This also all works correctly even if there are many rounds of flattening and such.

# Future directions

My hope is that others will find this project useful or interesting, and contribute any bug reports, reports of behaviors they find confusing, patches, and so on, to help the project reach maturity.

Note that just because this crate doesn't have a feature like subcommands now doesn't mean that I am opposed to that feature -- patches are welcome, and I can certainly see use-cases.
For example, you may have some targets that are web services, and some that are associated command-line tools, and you may want to be able to share a bunch of config structures between them.

But it's important to understand that I created this crate primarily to *serve an underserved niche*, which is large "12-factor app"  web projects, and not to try to achieve feature parity with `clap-derive`.
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

This is good because it's very simple and very configurable -- if you decide that you'd rather use `serde_json_lenient::from_str` instead, it's easy for you to change to that and you aren't tied to my choices of libraries or versions.

I think it might make sense to make an "extras" crate that contains "common" or "popular" value parsers. One nice thing about that is that those value parsers could also be used with `clap`.
Right now, I don't think that *this* crate should contain any code that reads a file. I haven't seen a compelling reason that that's necessary, and I see good reasons to try to separate concerns.

Personally, I do like using a crate like [`dotenvy`](https://crates.io/crates/dotenvy) to load `.env` files before parsing the config, as described in `README.md`.

My belief is that by keeping the API surface area relatively small and staying focused on the target niche, we can make sure that it stays as easy as possible to add useful features, test them appropriately, and drive the project forwards. I do believe that building on `clap` is the best course in terms of conserving developer energy and serving the users the best.
