# conf

`conf` is an alternative derive macro for [`clap`](https://docs.rs/clap/latest/clap/). It supports some powerful features that `clap-derive` does not, which help with the configuration of large projects, such as a web app following the [12-factor style](https://12factor.net/config). In exchange, some features are not implemented that are less useful for such purposes.

In some cases, there are deviations from `clap-derive` to either help avoid mistakes, or to make the defaults closer to a good 12-factor app behavior.

`conf` is heavily influenced by [`clap-derive`](https://docs.rs/clap/latest/clap/) and the earlier `struct-opt` which I used for years. They are both great and became popular for a reason.
However, there are some specific missing features (prefixing on flatten) and related pain points that I ran into over and over again. It seems to be hard to add these features now.
These features are very helpful when you want to be able to configure a large web app with many parts entirely via the environment.
See [motivation](./MOTIVATION.md) for more detail.

## Using `conf` in a cargo project

First add `conf` to the dependencies in your `Cargo.toml` file:

```
[dependencies]
conf = "0.1"
```

NOTE: Not actually published to crates.io yet...

Then, create a `struct` which represents the configuration data your application needs to read on startup.
This struct should derive the `Conf` trait, and the `conf` attributes should be used to describe how each field can be read.

```
#[derive(Conf)]
pub struct Config {
    /// This flag enables something
    #[conf(long)]
    my_flag: bool,

    /// URL to hit
    #[conf(long, env)]
    url: String,
}
```

Finally, you can parse the config:

```
    let config = Config::parse();
```

Usually you would call that somewhere in `fn main()` and then use the `config` to initialize your application.

The `parse()` function will automatically add a `--help` option for users that contains auto-generated documentation, based on your doc strings.

Additionally, if parsing fails for some reason, it will display a helpful error message and exit.

(The `Conf` trait offers a few variants of this function, which you can read about in the docs.)

Generally, the CLI interface and help text that is generated is meant to conform to POSIX and GNU conventions.

## A tour

A field in your struct can be read from a few sources:

* `#[conf(short)]` means that it corresponds to a "short" option, such as `-u`. By default the first letter of your field is used. This can be overridden with `#[conf(short='t')]` for example.
* `#[conf(long)]` means that it corresponds to a "long" option, such as `--url`. By default the kebab-case name of your field is used. This can be overridden with `#[conf(long="target-url")]` for example.
* `#[conf(env)]` means that it corresponds to an environment variable, such as `URL`. By default the upper snake-case name of your field is used. This can be overridden with `#[conf(env="TARGET_URL")]` for example.
* `#[conf(default_value)]` specifies a default value for this field if none of the other three possible sources provides one.

Such attributes can be combined by separating them with commas, for example `#[conf(long, env, default_value="x")]` means the field has an assocated long option, an associated environment variable, and a default value if both of these are omitted.

Your field can have any type as long as it implements `FromStr`, and this will be used to parse it.
The type `bool` is special and results in a "flag" being generated rather than a "parameter", which expects no string parameter to be passed during parsing.
`Option<T>` is also special, and indicates that the value is optional rather than required. You can also specify an alternative parsing function using `value_parser`.

So far this is almost exactly the same `clap-derive`. Where it gets more interesting is the `flatten` option.

You may have one structure that derives `Conf` and declares a bunch of related config values:

```
#[derive(Conf)]
pub struct DbConfig {
   ...
}
```

Then you can "flatten" it into a larger `Conf` structure using the `conf(flatten)` attribute.

```
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

```
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

```
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

```
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

```
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

There are a few more proc-macro attributes besides this, which you can read about in the docs or the [reference](./REFERENCE.md), but hopefully this is enough to get started.

## Who should use this crate?

The best reason to use this is crate is if you have a medium-to-large project, such as a web app consisting of multiple services, which has a lot of configuration needs. You have multiple services that have several common subsystems or components, and these components have subcomponents, some of which are shared, etc., all of which should read config from the environment in accordance with 12-factor style, and may need to read more such config on short notice. The purpose of the crate is to help you arrange all that config in the simplest and most maintainable way possible, while still ensuring that all values that are needed are checked for on program startup (failing fast), and providing automated `--help` documentation of all of this.

If you think this crate is a good fit for you, I believe that the most effective way to use it is:

* Whenever you have a component that you think should use a value that is read on startup, you should create a config struct for that component.
  You should `derive(Conf)` on that struct, and pass that config struct to the component on initialization.
  The config struct should live in the same module as the component that it is configuring.
* If your component is initialized by a larger component, then that component should have its own config struct and you should use `flatten` to assemble it. You should usually use the `prefix` and `help_prefix` options when flattening.
* Each binary target should have a config struct, and should `::parse()` it in `fn main()`.

This way, whenever you discover in the future that you need to add more config values for one of your small components, all you have to do is add it to the associated config struct, and it will automatically appear in every service that needs it, as many times as needed with appropriate prefixing, without you having to plumb it through every step of the way. Additionally, it makes it easier to create correct config for any future services or tools. And it causes all of your services and tools to have a similar, predictable style, and to have all of their config documented in `--help`, even pretty obscure environment variables and such, which usually just don't get documented if you choose to read them directly from `std::env` instead.

The argument parsing functionality of this crate is of secondary importance -- the 12-factor app reads *all* config from the environment when deployed. The main reason that CLI argument parsing functionality is here is:

* It's sometimes very convenient when running services locally, or in CI, to be able to pass config as CLI arguments instead of env, or to pass these arguments knowing that they will shadow whatever values are in `env`.
* I had some existing projects that were using `clap-derive` that I wanted to be able to migrate to this in a non-disruptive way.
* I wanted to ensure that `--help` could be easily auto-generated in a way that includes all the relevant information about `env` arguments.

This is somewhat different from the history of [`clap`](https://docs.rs/clap/latest/clap/), which started life as a CLI argument parser, and added `env` fallback after the library was already pretty mature.

If you have very specific CLI argument parsing needs, you will likely be better off with `clap` than this crate, because `clap` has considerably more features and configurability around that than this crate does.

## License

Code is available under MIT or Apache 2 at your option.
