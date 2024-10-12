# conf

`conf` is a `derive`-based env-and-argument parser aimed at the practically-minded web developer building large web projects.

[![Crates.io](https://img.shields.io/crates/v/conf?style=flat-square)](https://crates.io/crates/conf)
[![Crates.io](https://img.shields.io/crates/d/conf?style=flat-square)](https://crates.io/crates/conf)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square)](LICENSE-APACHE)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE-MIT)
[![Build Status](https://img.shields.io/github/actions/workflow/status/cbeck88/conf-rs/ci-rust.yml?branch=develop&style=flat-square)](https://github.com/cbeck88/conf-rs/actions/workflows/ci-rust.yml?query=branch%3Adevelop)

[API Docs](https://docs.rs/conf/latest/conf/) | [Proc-macro Reference](./REFERENCE.md)

## Overview

[`conf`](https://docs.rs/conf/latest/conf/) uses [`clap`](https://docs.rs/clap/latest/clap/) under the hood to parse CLI arguments and generate help text.

`conf` has an intentionally similar proc-macro API to `clap-derive`, but it is not a fork. It is a new library with different goals. It offers some powerful features and support that `clap-derive` does not, which help with the configuration of large projects. But it also doesn't offer some features of `clap`, which I have found to be less useful in a typical web project.

The features that you get for this bargain are:

* You can **assign a prefix to a structure's fields when flattening** it into another structure, and you can similarly do `env` prefixing in a controlled way.
* **You get ALL the errors and not just one of them** if some required env is missing and/or several of the values are invalid. In my searching I found that surprisingly few config crates out there actually do this. Very helpful if your deployments take a while.
* **Isolation & testability around `env`**. `clap` only supports reading env values from `std::env::var_os`.
  * If you want to test what happens when different variables are set, your tests can become racy.
  * If you want to test a component that takes config as an argument, and use `::parse_from` to initialize the config, then your tests will pass or fail depending on your local env.
  * If you want to implement `Default` based on the default values your declared on your structure, you can't really because you can't isolate it from `env`.
  * `conf` lets you pass an iterator to represent a snapshot of the environment.
* **Support for `env` aliases**. `clap` supports aliases for command-line arguments but not for `env`. Make changes without breaking compatibility.
* **You can declare fields which are only read from `env`** and cannot be read from args at all.
* **You can declare fields which represent secrets.** This controls whether or not the entire value should be printed in error messages if it fails to parse.
* **Support for an optional-flatten syntax**. This can be simpler and more idiomatic than using argument groups and such in `clap-derive`.
* **Support for user-defined validation predicates**. This allows you to express constraints that can't be expressed in `clap`.

`conf` is heavily influenced by [`clap-derive`](https://docs.rs/clap/latest/clap/) and the earlier [`struct-opt`](https://docs.rs/structopt/latest/structopt/) which I used for years. They are both great and became popular for a reason.

In most cases, `conf` tries to stay extremely close to `clap-derive` syntax and behavior, for familiarity and ease of migrating a large project.
In some cases, there are small deviations from the behavior of `clap-derive` to either help avoid mistakes, or to make the defaults closer to a good [12-factor app](https://12factor.net/config) behavior.
For some advanced features of `clap`, `conf` has a way to achieve the same thing, but we took a different approach. This is typically in an attempt to simplify how it works for the user of the `derive` macro, to have fewer named concepts, or to ease maintenance going forward.

The public API here is restricted to the `Conf` and `Subcommands` traits, proc-macros to derive them, and one error type. It is hoped that this will both reduce the learning curve and ease future development and maintenance.

See [MOTIVATION.md](./MOTIVATION.md) for more discussion about this project and the other various alternatives out there.

* [Using conf in a cargo project](#using-conf-in-a-cargo-project)
* [A tour](#a-tour)
* [Topics](#topics)
  * [Reading files](#reading-files)
  * [Hierarchical config](#hierarchical-config)
  * [Secrets](#secrets)
  * [Argument groups and constraints](#argument-groups-and-constraints)
* [Who should use this crate?](#who-should-use-this-crate)
  * [When should clap-derive be preferred to this crate?](#when-should-clap-derive-be-preferred-to-this-crate)
* [License](#license)

## Using conf in a cargo project

First add `conf` to the dependencies in your `Cargo.toml` file:

```
[dependencies]
conf = "0.1"
```

Then, create a `struct` which represents the configuration data your application needs to read on startup.
This struct should derive the `Conf` trait, and the `conf` attributes should be used to describe how each field can be read.

```rust
#[derive(Conf)]
pub struct Config {
    /// This is a string parameter, which can be read from args as `--my-param` or from env as `MY_PARAM`.
    #[arg(long, env)]
    my_param: String,

    /// This flag corresponds to `-f` or `--force` in args
    #[arg(short, long)]
    force: bool,

    /// URL to hit, which can be read from args as `--url` or from env as `URL`.
    #[arg(long, env)]
    url: Url, // This works because Url implements `FromStr`.
}
```

Finally, you can parse the config:

```rust
    let config = Config::parse();
```

Usually you would call that somewhere in `fn main()` and then use the `config` to initialize your application.

The `parse()` function will automatically add a `--help` option for users that contains auto-generated documentation, based on your doc strings.

Additionally, if parsing fails for some reason, it will display a helpful error message and exit.

(The `Conf` trait offers a few variants of this function, which you can read about in the docs.)

Generally, the CLI interface and help text that is generated is meant to conform to POSIX and GNU conventions. Read more in [`clap` docu](https://docs.rs/clap/latest/clap/) about this.

## A tour

A field in your struct can be read from a few sources:

* `#[arg(short)]` means that it has an associated "short" command-line option, such as `-u`. By default the first letter of your field is used. This can be overridden with `#[arg(short='t')]` for example.
* `#[arg(long)]` means that it has an associated "long" command-line option, such as `--url`. By default the kebab-case name of your field is used. This can be overridden with `#[arg(long="target-url")]` for example.
* `#[arg(env)]` means that it has an associated environment variable, such as `URL`. By default the upper snake-case name of your field is used. This can be overridden with `#[arg(env="TARGET_URL")]` for example.
* `#[arg(default_value)]` specifies a default value for this field if none of the other three possible sources provides one.

Such attributes can be combined by separating them with commas, for example `#[arg(long, env, default_value="x")]` means the field has an assocated long option, an associated environment variable, and a default value if both of these are omitted.

Your field can have any type as long as it implements `FromStr`, and this will be used to parse it.
The type `bool` is special and results in a "flag" being generated rather than a "parameter", which expects no string parameter to be passed during parsing.
`Option<T>` is also special, and indicates that the value is optional rather than required. You can also specify an alternative parsing function using `value_parser`.

So far this is almost exactly the same `clap-derive`. Where it gets more interesting is the `flatten` option.

You may have one structure that derives `Conf` and declares a bunch of related config values:

```rust
#[derive(Conf)]
pub struct DbConfig {
    /// Database connection URL.
    #[arg(long)]
    pub db_url: String,

    /// Set the maximum number of connections of the pool.
    #[arg(long)]
    pub db_max_connections: Option<u32>,

    /// Set the minimum number of connections of the pool.
    #[arg(long)]
    pub db_min_connections: Option<u32>,

    /// Set the timeout duration when acquiring a connection.
    #[arg(long)]
    pub db_connect_timeout: Option<u64>,

    /// Set the maximum amount of time to spend waiting for acquiring a connection.
    #[arg(long)]
    pub db_acquire_timeout: Option<u64>,

    /// Set the idle duration before closing a connection.
    #[arg(long)]
    pub db_idle_timeout: Option<u64>,

    /// Set the maximum lifetime of individual connections.
    #[arg(long)]
    pub db_max_lifetime: Option<u64>
}
```

Then you can "flatten" it into a larger `Conf` structure using the `conf(flatten)` attribute.

```rust
#[derive(Conf)]
pub struct Config {
    /// Database
    #[conf(flatten)]
    db: DbConfig,
}
```

Intuitively, this is meant to read a lot like the [`serde(flatten)`](https://serde.rs/attr-flatten.html) attribute, and has a similar behavior.
During parsing, the parser behaves as if every field of `DbConfig` were declared within `Config`, and generates matching options, env, and help, but then the parsed values actually
get stored in subfields of the `.db` field.

Using `flatten` can save a lot of labor. For example, suppose your web application consists of ten different web services, and they all need a `DbConfig`. Instead of duplicating all the values,
any env, any defaults, any help text, in each `Config` that you have, you can write that once and then `flatten` it ten times. Then, later when you discover that `DbConfig` should contain another value,
you only have to add it to `DbConfig` once, and every service that uses `DbConfig` will get the new config parameter. Also, when you need to initialize your db connection, you can just pass it the entire `.db` field rather
than pick out needed config arguments one-by-one.

Where `conf` differs from `clap-derive` is that we expect that you will use `flatten` in your project quite a lot.

For example, you might need to do this:

```rust
#[derive(Conf)]
pub struct Config {
    #[conf(flatten)]
    pub auth_service: HttpClientConfig,

    #[conf(flatten)]
    pub friend_service: HttpClientConfig,

    #[conf(flatten)]
    pub snaps_service: HttpClientConfig,
}
```

because logically, you have three different http clients that you need to configure.

However with `clap-derive`, this is going to cause a problem, because when the fields from `HttpClientConfig` get flattened, their names will collide, and the parser will reject it as ambiguous.

When using `conf`, you can resolve it by declaring a prefix.

```rust
#[derive(Conf)]
pub struct Config {
    #[conf(flatten, prefix)]
    pub auth_service: HttpClientConfig,

    #[conf(flatten, prefix)]
    pub friend_service: HttpClientConfig,

    #[conf(flatten, prefix)]
    pub snaps_service: HttpClientConfig,
}
```

This will cause every option associated to the `auth_service` structure to get a prefix, derived from the field name, `auth_service`, on any long-form options and on any env variables. The prefix will be kebab-case for long-form options and upper snake-case for env variables. And similarly for `friend_service` and `snaps_service`.

You can also override this prefix:

```rust
#[derive(Conf)]
pub struct Config {
    #[conf(flatten, prefix="auth")]
    pub auth_service: HttpClientConfig,

    #[conf(flatten, prefix="friend")]
    pub friend_service: HttpClientConfig,

    #[conf(flatten, prefix="snaps")]
    pub snaps_service: HttpClientConfig,
}
```

You can also configure env prefixes and option prefixes separately if you want that. Setting `env_prefix` will cause env vars to be prefixed, but not options. `long_prefix` will cause long-form options to be prefixed, but not env vars. (Short options are never prefixed, so there is not usually a good way to resolve a conflict among them. Short options should be used with caution in a large project.)

Finally, you can also declare prefixes at the level of a struct rather than a field. So for example, if you need every environment variable your program reads to be prefixed with `ACME_`, you can achieve that very easily.

```rust
#[derive(Conf)]
#[conf(env_prefix="ACME_")]
pub struct Config {
    #[conf(flatten, prefix="auth")]
    pub auth_service: HttpClientConfig,

    #[conf(flatten, prefix="friend")]
    pub friend_service: HttpClientConfig,

    #[conf(flatten, prefix="snaps")]
    pub snaps_service: HttpClientConfig,
}
```

`Option<T>` can also be used with a flattened structure, so if one of these services is optional, you can simply write:

```rust
#[derive(Conf)]
#[conf(env_prefix="ACME_")]
pub struct Config {
    #[conf(flatten, prefix="auth")]
    pub auth_service: HttpClientConfig,

    #[conf(flatten, prefix="friend")]
    pub friend_service: HttpClientConfig,

    #[conf(flatten, prefix="snaps")]
    pub snaps_service: Option<HttpClientConfig>,
}
```

You can read about all the attributes and usage in the docs or the [REFERENCE.md](./REFERENCE.md), but hopefully this is enough to get started.

See also the [examples](./examples).

## Topics

This section discusses more advanced features and usage patterns, as well as alternatives.

### Reading files

Sometimes, a web service needs to read a file on startup. `conf` supports this by using the `value_parser` feature, which works very similarly as in `clap`.

A `value_parser` is a function that takes a `&str` and returns either a value or an error.

For example, if you need to read a `yaml` file on startup according to a schema, one way you could do that is

```rust
use conf::Conf;
use serde::Deserialize;
use std::{error::Error, fs};

#[derive(Deserialize)]
pub struct MyYamlSchema {
    pub example: String,
}

#[derive(Conf)]
pub struct Config {
    #[conf(long, env, value_parser = |file: &str| -> Result<_, Error> { Ok(serde_yaml::from_str(fs::read_to_string(&file)?)?) }]
    pub yaml_file: MyYamlSchema,
}
```

This will read a file path either from CLI args or from env, then attempt to open the file and parse it according to the yaml schema.

If your `value_parser` is complex or needs to be reused, the best practice is to put it in a named function.

```rust
#[derive(Conf)]
pub struct Config {
    #[conf(long, env, value_parser = utils::read_yaml_file)]
    pub yaml_file: MyYamlSchema,
}
```

This can also be a good pattern for things like reading a certificate or a cryptographic key from a file, which you want to check on startup, failing fast if the file is not found or is invalid.

### Hierarchical config

[Hierarchical config](https://rust-cli-recommendations.sunshowers.io/hierarchical-config.html) is the idea that config values should be merged in from files as well as from args and env.

> Applications *should* follow a hierarchical configuration structure. Use the following order, from highest priority to lowest.
>
>    1. Command-line arguments
>    2. Environment variables
>    3. Directory or repository-scoped configuration
>    4. User-scoped configuration
>    5. System-wide configuration
>    6. Default configuration shipped with the program.

`conf` has built-in support for (1), (2), and (6) here.

To get the others when using something like `conf`, a common practice is to use a crate like [`dotenvy`](https://crates.io/crates/dotenvy). This crate can search for an `.env` file, and then set `env` values if they are not already set in your program.
You can do this right before calling `Config::parse()`, and in this manner achieve hierarchical config. You can load multiple `.env` files this way if you need to.

In web applications, I often use this approach for *development* rather than production.

If your application has a lot of required values, it may take an engineer a while to figure out how to just run it locally. But you may not want to provide default values in the program that would not be appropriate in production, for safety. Instead, you can provide a `.env` file which is checked in to the repo, with values which are appropriate for local testing / CI. Then an engineer can use `cargo run` and it will just work. When you go to build docker containers, you can leave out these `.env` files, and then be sure that in the deployed environment, kubernetes or similar is in total control, and any missing or misspelled values in the helm charts and whatnot will be loud and fail fast.

These `.env` files work well if you are using [`diesel`](https://crates.io/crates/diesel), because the `diesel` cli tool also [uses `dotenvy` to search for a `.env` file](https://diesel.rs/guides/getting-started) and find the `DATABASE_URL` when manging database migrations locally.

This approach to hierarchical config is much less general than what crates like [`config`](https://crates.io/crates/config) and [`figment`](https://crates.io/crates/figment) offer, but it's also simpler, and it's easy to change and debug. There are other reasons discussed in [MOTIVATION.md](./MOTIVATION.md) that I personally favor this approach. This of course is highly opinionated -- over time `conf` may add more features that support other ways of using it. To start, I only built what I felt I needed. Your mileage may vary.

### Secrets

`conf` tries to provide the most helpful and detailed errors that it can, and also to report as many problems as it can when parsing fails.

Usually, if a user-provided value cannot be parsed, we want to provide the value and the error in the error message to help debugging. But if the value represents a *secret*, then logging its value is bad.

To prevent `conf` from logging the value, you can mark the field as `secret`.

```rust
    #[arg(env, secret)]
    pub api_key: ApiKey
```

When `conf` knows that something is a secret, it will avoid revealing the value when generating any kind of error message or help text.
`conf` will also describe it with the `[secret]` tag in the help text.

Handling secrets is a complex topic and much of the discussion is out of scope here.
We'll offer just three points of guidance around this tool.

1. The more valuable the secrets are, and the more challenging the threat model is, the more time it makes sense to spend working on defensive measures. The converse is also true.
   No one really has context to judge this except you, so instead of offering one-size-fits-all guidance, I prefer to think in terms of a sliding scale.
2. If you're at a point where systematically marking things `secret` seems like a good idea, then you should also be using special types to manage the secrets.
   For example, using [`SecretString` from the `secrecy` crate](https://docs.rs/secrecy/0.8.0/secrecy/type.SecretString.html) instead of `String` will prevent your password from appearing in debug logs *after* it has been loaded.
   There are alternatives out there if `secrecy` crate doesn't work for your use-case. This is usually a pretty low-effort improvement, and it goes hand-in-hand with what the `secret` marking does.
   * It's very easy to expose your secret by accident if you don't do something like this. For example, just by putting a `#[tracing::instrument]` annotation on a function that some day takes a `config` struct, you could accidentally log your password.
3. If you're at a point where you think you need to *systematically [zeroize](https://docs.rs/zeroize/latest/zeroize/) all copies* of your secret that reside in process memory when they are no longer needed, then you are past the point
   where you can use an environment variable to pass the secret value to the application. Your application most likely needs to *read the secret value from a file instead*.
   * The rust standard library handles environment values as `std::ffi::OsString` internally and in its API, but this type cannot be securely zeroized. There are no public APIs to mutably access the underlying bytes, and no public APIs that would otherwise do this for you.
   * At a lower level, `glibc` [exposes the environment as `char **environ`](https://www.gnu.org/software/libc/manual/html_node/Environment-Access.html), makes copies of the entire environment whenever it is changed using `set_var` or similar, and [leaks the old values](https://inbox.sourceware.org/libc-alpha/87le2od4xh.fsf@oldenburg.str.redhat.com/).
     It is difficult to systematically ensure that all of these copies are cleaned up if they contain sensitive data. `environ` often gets copied by other things very early in the process.
     The rust standard library also interacts with the environment via these `glibc` APIs, which means that typical rust libraries like `dotenvy` do as well.

### Argument groups and constraints

`clap` has support for the concept of "argument groups" (`ArgGroup`) and also "dependencies" among `Arg`'s. This is used to create additional conditions that must be satisfied for the config to be valid, and error messages if it is invalid.
`clap` provides many functions on `Arg` and on `ArgGroup` which can be used to define various kinds of constraints, such as conditional dependency or mutual exclusion, between `Arg`'s or `ArgGroup`'s.

The main reason to use these features in `clap` is that it will generate nicely formatted errors if these constraints are violated, and then you don't have to worry about handling the situation in your application code.

`conf` similarly wants to support adding constraints in this manner that are checked during parsing, but the design goal is that all of these errors should reportable alongside all the other types of errors.

For several reasons, `conf` chose to offer a different API than the `clap` for these purposes.

* In `clap`, this API was designed first for the clap builder API, and then exposed via the `clap-derive` API.
* There are about a dozen functions exposed in total, and multiple named concepts (`Arg` is now joined by `ArgGroup` which is different from `Args`)
* The API relies on explicit `id` values for `Arg`'s and `ArgGroup`s, but this is less idiomatic in the derive API. The derive API is simpler from the user's point of view if these `id`'s are not really exposed and are more like implementation details.
* The API often provides multiple ways to do the same thing, which makes code that uses it less predictable.
* The API has many defaults that I find hard to remember. For example, in an `ArgGroup`, does `required` default to `true` or `false`? Does `multiple` default to `true` or `false`? These defaults are different for an `Args`.
* Sometimes the API doesn't feel idiomatic. For example if I have a group of options where if one of them appears, all of them must appear, the most idiomatic thing is if the API can give me a single `Option` that includes all of them.
  Otherwise I have to unwrap a bunch of options in application code, on the assumption that my constraint works as expected.

`conf` provides one mechanism for idiomatically representing when some collection of arguments are optional-but-mutually-required. Then it provides a few one-offs to express exclusivity between arguments. Finally, it provides a very general mechanism that can express arbitrary constraints.

#### flatten-optional

`conf` supports the following syntax:

```rust
#[derive(Conf)]
pub struct Config {
    #[conf(flatten, prefix="auth")]
    pub auth_service: HttpClientConfig,

    #[conf(flatten, prefix="friend")]
    pub friend_service: HttpClientConfig,

    #[conf(flatten, prefix="snaps")]
    pub snaps_service: Option<HttpClientConfig>,
}
```

Intuitively, this means that the `snaps_service` config is optional, and if none of those fields appear, that's not an error, and `snaps_service` will be `None` in the parsed config object.
However, if any of the fields of `snaps_service` appear, then all of its required fields must appear, and parsing the entire flattened object must succeed.

This allows the code that consumes the conditional config to be simpler -- you can just match on whether `snaps_service` is present or absent, and the type system encodes that when any of those fields are present, all are present.
And you can express which arguments in the group are required to be present or not by marking them optional or not (or giving them a default value), within `HttpClientConfig`.

This feature actually covers every use-case I've had in real-life for argument groups and constraints in `clap` across all my web projects, and I like it because I feel that it introduces fewer named concepts
and promotes code reuse. The same struct can be flattened in a required way in one setting and in an optional way in another setting.

Hopefully it's easy to remember what it means, just by looking at the type of the data, and thinking about what would have to happen for it to succeed.
If we can't see any of the (prefixed) substructure's fields appearing, then we return `None`. If we see some of them appearing, it indicates that we're supposed to be producing a `Some`. Once we decide that we're supposed to produce `Some`, it's an error if we can't do so in the normal (non-optional) manner for `flatten`'ed structures.

#### one_of_fields

`conf` provides a simple way to specify that some fields in a struct are mutually exclusive.

```rust
#[derive(Conf)]
#[conf(at_most_one_of_fields(a, b, c))]
pub struct FooConfig {
    #[conf(short, long)]
    pub a: bool,
    #[conf(short, long)]
    pub b: Option<String>,
    #[conf(long, env)]
    pub c: Vec<String>,
}
```

When used with two fields, it provides a way to translate many usages of `conflicts_with` in the `clap-derive` API.

When used with all fields in a struct, it is similar to an `ArgGroup` with `multiple=false` and `required=false` in the `clap-derive` API.

This also works with the *flatten-optional* feature, so one or more optional flattened groups can be made exclusive with eachother or with simple arguments in this structure.

However, it can only be used with fields on the struct that is marked with this attribute, and cannot be used with fields inside of flattened structs, or elsewhere in the structure.

`conf` provides a variation which requires *exactly* one of the fields to appear.

```rust
#[derive(Conf)]
#[conf(one_of_fields(a, b, c))]
pub struct FooConfig {
    #[conf(short, long)]
    pub a: bool,
    #[conf(short, long)]
    pub b: Option<String>,
    #[conf(long, env)]
    pub c: Vec<String>,
}
```

When used with all fields in a struct, this is similar to an `ArgGroup` with `multiple=false` and `required=true` in the `clap-derive` API.

Finally `conf` provides one more variation

```rust
#[derive(Conf)]
#[conf(at_least_one_of_fields(a, b, c))]
pub struct FooConfig {
    #[conf(short, long)]
    pub a: bool,
    #[conf(short, long)]
    pub b: Option<String>,
    #[conf(long, env)]
    pub c: Vec<String>,
}
```

When used with all fields in a struct, this is similar to an `ArgGroup` with `multiple=true` and `required=true` in the `clap-derive` API.

Any of these attributes can be used multiple times on the same struct to create multiple constraints that apply to that struct.

#### validation predicate

`flatten-optional` and `one_of_fields` provide some easy-to-understand ways to create dependencies and exclusion constraints between different optional fields in a `conf` structure.
They can directly translate many simple uses of `ArgGroup` and some of the constraints in the `clap-derive` API. But, there are many other constraint types supported by `clap`
that don't translate directly into this, and we don't support declaring arg group membership directly on a field, which is something that clap does support.

At the same time, there are other kinds of constraints you might have a legitimate use for that you can't express in `clap`'s API.
For example, one of your arguments might be a `Url` object, and you might want to require that if the `Url` starts with `https` then some other options are required. As far as I know, there's no way to do this in `clap`.

Instead of providing direct analogues for every function in `clap`'s constraint API,
`conf` supports user-defined validation predicates on a per-struct basis.

A validation predicate is a function that takes `&T` where `T` is the struct at hand, and returns `Result<(), impl Display>`.

It behaves similarly to `value_parser`, in that any function expression can be accepted.

The idea here is, rather than adding increasing numbers of one-off constraint types to `conf`, or enabling you to write non-local constraints using proc-macro attributes, it
will be more maintainable for you and for `conf` if you just express what you want in rust code, once your constraints get sophisticated enough.
There's both less API for you to learn and remember, and less API surface area for `conf` to test and maintain. You will also be able to generate very precise error messages when complex constraints fail.

Using these features together, you can express any kind of constraint you want to impose on your config structure, and hopefully make it feel idiomatic and natural.

-----

Given that the `validation_predicate` for a `T` runs after we have actually parsed a `T`, why have this feature at all? The users could just run such functions on their own after `Config::parse` succeeds.

The benefit of using the `validation_predicate` is that if a predicate fails, `conf` is still able to report those errors and any other errors that occurred elsewhere in the tree.

For example, in this config struct:

```rust
#[derive(Conf)]
pub struct Config {
    #[conf(flatten, prefix="auth")]
    pub auth_service: HttpClientConfig,

    #[conf(flatten, prefix="friend")]
    pub friend_service: HttpClientConfig,

    #[conf(flatten, prefix="snaps")]
    pub snaps_service: Option<HttpClientConfig>,
}
```

It's possible that when parsing a `Config`, the `auth_service` fails to parse because of a missing required argument, `friend_service` fails to parse because of a missing argument and an invalid value, and `snaps_service` parses but fails its validation predicate. In this scenario `conf` will report all of these errors, which distinguishes it from other crates in this genre.

## Who should use this crate?

The best reason to use this is crate is if you have a medium-to-large project, such as a web app consisting of multiple services, which has a lot of configuration needs. You have multiple services that have several common subsystems or components, and these components have subcomponents, some of which are shared, etc., all of which should read config from the environment in accordance with 12-factor style, and may need to read more such config on short notice. You may already be using `clap-derive` but have run into limitations as your project has grown.

The purpose of the crate is to help you arrange all of that config in the simplest and most maintainable way possible, while still ensuring that all values that are needed are checked for on program startup (failing fast), reporting as many configuration errors as possible in the most helpful way possible when your deployment goes bad, and providing automated `--help` documentation of all of the config that is being read.

If you think that this crate is a good fit for you, the suggested way to use it is:

* Whenever you have a component that you think should use a value that is read on startup, you should create a config struct for that component.
  You should `derive(Conf)` on that struct, and pass that config struct to the component on initialization.
  The config struct should live in the same module as the component that it is configuring.
* If your component is initialized by a larger component, then that component should have its own config struct and you should use `flatten` to assemble it. You should usually use the `prefix` and `help_prefix` options when flattening.
* Each binary target should have a config struct, and should `::parse()` it in `fn main()`.

This way, whenever you discover in the future that you need to add more config values for one of your small components, all you have to do is add it to the associated config struct, and it will automatically appear in every service that needs it, as many times as needed with appropriate prefixing, without you having to plumb it through every step of the way. Additionally, it makes it easier to create correct config for any future services or tools. And it causes all of your services and tools to have a similar, predictable style, and to have all of their config documented in `--help`, even pretty obscure environment variables and such, which usually just don't get documented if you choose to read them directly from `std::env` instead.

### When should clap-derive be preferred to this crate?

This crate defines itself somewhat differently from [`clap-derive`](https://docs.rs/clap/latest/clap/) and has different features and goals.

* `clap-derive` is meant to be an alternative to the clap builder API, and exposes essentially all of the features of the builder.
* `clap` itself is primarily a CLI argument parser [per maintainers](https://github.com/clap-rs/clap/discussions/5432), and many simple features around `env` support, like, arguments that can only be read from `env`, are considered out of scope.

`conf` places emphasis on features differently.

* `env` is actually the most important thing for a 12-factor web app.
* `conf` has a different architecture, such that it's easier to pass information at runtime between a `struct` and the `struct` that it is flattened into, in both directions. This enables many of the new features that it brings to the table. The details are not part of the public API, so that they can be extended to support new features without a breaking change.
* `conf` has very specific goals around error reporting. We want to return as many config errors as possible at once, because deployment might take a relatively long time.

In order to meet its goals, `conf` does not use `clap` to handle `env` at all. `clap` is only used to parse CLI arguments as strings, and to render help text, which are the two things that it is best at.

This crate can expose more features of the underlying `clap` builder and get closer towards the feature set offered by `clap-derive`, but will probably never expose all of them -- we can only expose features that we are sure will work well with the additional features that we have created, like flatten-with-prefix, and will work well with the manner in which we are using the underlying clap builder. The most interesting features are those that can be motivated by common web development needs.

If you have very specific CLI argument parsing needs, or if you need pixel-perfect help text, you will be better off using `clap` directly instead of this crate, because you will have more control that way. `clap` is the most mature and feature-complete CLI argument parser out there, by a wide margin.

In many web projects, you don't really have such needs. You aren't making very sophisticated use of `clap`, your project is small, and you don't particularly need any features of `conf` either, so you will be able to use `clap-derive` or `conf` equally well and not notice very much difference.

If you prefer, you can stick with `clap-derive`, and then only if you find that you need flatten-with-prefix or another feature, try to switch to `conf` at that point.

`conf` is designed to make this migration relatively easy for such projects. (Indeed, I started working on `conf` because I had several large projects on `clap-derive` and I was hitting limitations and being forced info workarounds that I wasn't happy with, and I couldn't find a wholly satsifactory alternative.) If you find that you get stuck when trying to migrate, you can open a discussion and we can try to help.

## License

Code is available under MIT or Apache 2 at your option.
