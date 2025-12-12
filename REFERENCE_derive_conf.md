# `derive Conf` proc-macro reference

The `#[derive(Conf)]` macro can only be placed on a `struct`.

When using `#[derive(Conf)]`, the result is adjusted by various `#[conf(...)]` attributes that can be applied.
These are documented here.

The `#[conf(...)]` attributes conform to [Rust’s structured attribute convention](https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax).

* [Where can conf attributes be used?](#where-can-conf-attributes-be-used)
* [Field-level attributes](#field-level-attributes)
  * [Flag](#flag)
    * [short](#flag-short)
    * [long](#flag-long)
    * [env](#flag-env)
    * [aliases](#flag-aliases)
    * [env_aliases](#flag-env-aliases)
    * [serde](#flag-serde)
      * [rename](#flag-serde-rename)
      * [alias](#flag-serde-alias)
      * [deserialize_with](#flag-serde-deserialize-with)
      * [try_from](#flag-serde-try-from)
      * [skip](#flag-serde-skip)
  * [Parameter](#parameter)
    * [short](#parameter-short)
    * [long](#parameter-long)
    * [pos](#parameter-pos)
    * [env](#parameter-env)
    * [aliases](#parameter-aliases)
    * [env_aliases](#parameter-env-aliases)
    * [default_value](#parameter-default-value)
    * [default_help_str](#parameter-default-help-str)
    * [value_parser](#parameter-value-parser)
    * [value_parser_os](#parameter-value-parser-os)
    * [allow_hyphen_values](#parameter-allow-hyphen-values)
    * [allow_negative_numbers](#parameter-allow-negative-numbers)
    * [default_if_missing](#parameter-default-if-missing)
    * [default](#parameter-default)
    * [secret](#parameter-secret)
    * [serde](#parameter-serde)
      * [rename](#parameter-serde-rename)
      * [alias](#parameter-serde-alias)
      * [deserialize_with](#parameter-serde-deserialize-with)
      * [try_from](#parameter-serde-try-from)
      * [skip](#parameter-serde-skip)
      * [use_value_parser](#parameter-serde-use-value-parser)
    * [test](#parameter-test)
      * [skip_default_value](#parameter-test-skip-default-value)
  * [Repeat](#repeat)
    * [short](#repeat-short)
    * [long](#repeat-long)
    * [pos](#repeat-pos)
    * [env](#repeat-env)
    * [aliases](#repeat-aliases)
    * [env_aliases](#repeat-env-aliases)
    * [value_parser](#repeat-value-parser)
    * [value_parser_os](#repeat-value-parser-os)
    * [env_delimiter](#repeat-env-delimiter)
    * [no_env_delimiter](#repeat-no-env-delimiter)
    * [allow_hyphen_values](#repeat-allow-hyphen-values)
    * [allow_negative_numbers](#repeat-allow-negative-numbers)
    * [secret](#repeat-secret)
    * [serde](#repeat-serde)
      * [rename](#repeat-serde-rename)
      * [alias](#repeat-serde-alias)
      * [deserialize_with](#repeat-serde-deserialize-with)
      * [try_from](#repeat-serde-try-from)
      * [skip](#repeat-serde-skip)
      * [use_value_parser](#repeat-serde-use-value-parser)
  * [Flatten](#flatten)
    * [env_prefix](#flatten-env-prefix)
    * [long_prefix](#flatten-long-prefix)
    * [prefix](#flatten-prefix)
    * [help_prefix](#flatten-help-prefix)
    * [skip_short](#flatten-skip-short)
    * [serde](#flatten-serde)
      * [rename](#flatten-serde-rename)
      * [alias](#flatten-serde-alias)
      * [try_from](#flatten-serde-try-from)
      * [skip](#flatten-serde-skip)
      * [flatten](#flatten-serde-flatten)
        * [prefix](#flatten-serde-flatten-prefix)
  * [Subcommands](#subcommands)
    * [serde](#subcommands-serde)
      * [skip](#subcommands-serde-skip)
* [Struct-level attributes](#struct-level-attributes)
  * [no_help_flag](#struct-no-help-flag)
  * [version](#struct-version)
  * [version_fn](#struct-version-fn)
  * [about](#struct-about)
  * [name](#struct-name)
  * [display_name](#struct-display-name)
  * [env_prefix](#struct-env-prefix)
  * [serde](#struct-serde)
    * [allow_unknown_fields](#struct-serde-allow-unknown-fields)
  * [one_of_fields](#struct-one-of-fields)
  * [at_most_one_of_fields](#struct-at-most-one-of-fields)
  * [at_least_one_of_fields](#struct-at-least-one-of-fields)
  * [validation_predicate](#struct-validation-predicate)
  * [test](#struct-test)

## Where can conf attributes be used?

The `#[conf(...)]` attributes can appear in two places -- on a `struct` and on a `field`.

```rust
use conf::Conf;

#[derive(Conf)]
#[conf(env_prefix="RUST_")] // This is a struct-level attribute
pub struct MyConfig {
    /// This doc string becomes help text for my_field when --help flag is passed
    #[conf(long, env)] // These are field-level attributes
    pub my_field: String,

    /// This doc string becomes help text for my_flag
    #[conf(short, long)] // These are field-level attributes
    pub my_flag: bool,
}
```

Some attributes "take an argument", which means they are used like

`#[conf(attr=value)]`

When they do, this reference will specify what types of arguments are valid. If those arguments are not required, they
are described as "optional", and we will explain what the behavior is if they are omitted. If the arguments are not
marked optional, then they are required.

Attributes which take an argument like this, with the `=` sign, typically can only be set once. It's an error if they occur a second time on the same item. (In some cases they are allowed to repeat.)

Some attributes "take a parenthetical", which means they are used like

`#[conf(attr(...))]`

Such attributes may be allowed to repeat multiple times:

`#[conf(attr(...), attr(...))]`

In each case like this we'll document what syntax is valid in the parentheses.

## Field-level attributes

For compatibility with `clap-derive`, when a `conf` attribute is used on a field, the labels `#[arg(...)]` and `#[conf(...)]` **can be used interchangeably**.

When `derive(Conf)` encounters a field, the first thing it must determine *what kind of field* this is:

* **Flag**: A flag corresponds to a boolean program option. It is either set or it isn't. For example, `./my_prog --flag1 --flag2`.
* **Parameter**: A parameter corresponds to a program option that expects a string value to be found during parsing. For example `./my_prog --param1 value1 --param2 value2`.
* **Repeat**: A repeat field represents a list of values. It has special parsing -- it is allowed to be specified multiple times on the command-line, and the results are parsed separately and aggregated into a `Vec`. This is similar to what `clap` calls a multi-option, and what `clap-derive` does by default if the field type is a `Vec`. For example, `./my_prog --can-repeat value1 --can-repeat value2`.
* **Flatten**: A flatten field doesn't correspond to an option, but to a collection of options that come from another `Conf` structure, and may be adjusted before being merged in.
* **Subcommands**: A subcommands field doesn't correspond to an option, but to a collection of subcommands defined by a `Subcommands` enum. When a subcommand is used, any values parsed by the subcommand parser appear at the associated enum variant.

If the *first attribute* is `flag`, `parameter`, `repeat`, `flatten`, or `subcommands`, then `conf` will handle the field that way.

If none of these is found, then the *type* of the field is used to classify it [^1].

* If the field is `bool`, then it is a flag
* Otherwise it is a parameter.

In `conf` the only way to specify a repeat parameter is to use the `repeat` attribute. There is, intentionally, no type-based inference for that [^compat-note-1].

Each kind of field then supports a different set of attributes.

### Flag

A flag corresponds to a switch that doesn't take any parameters. It's presence on the command line means the value is `true`, otherwise it is `false`.

**Requirements**: A flag field must have type `bool`.

*  <a name="flag-short"></a> `short` (optional char argument)

   Specifies a short (one-dash) switch associated to this flag.
   If argument is omitted, defaults to the first letter of the field name.

   example usage: `#[arg(short)]`, `#[arg(short = 'b')]`

   example behavior: `./my_prog -b` sets the flag to true

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="flag-long"></a> `long` (optional string argument)

   Specifies a long (two-dash) switch associated to this flag.
   If argument is omitted, defaults to the kebab-cased field name.

   example: `#[arg(long)]`, `#[arg(long = "flag")]`

   example command-line: `./my_prog --flag` sets the flag to true

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="flag-env"></a> `env` (optional string argument)

   Specifies an environment variable associated to this flag.
   If argument is omitted, defaults to the upper snake-case field name.

   When the environment variable is set, the flag is considered to be true, unless
   the value is `0`, `false`, `f`, `off`, `o`, or empty string.

   example: `#[arg(env)]`, `#[arg(env = "FLAG")]`

   example command-line: `FLAG=1 ./my_prog` sets the flag to true

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="flag-aliases"></a>  `aliases` (string array argument)

   Specifies alternate long switches that should be an alias for this flag.
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.visible_aliases)

   example: `#[arg(aliases=["old-flag-name", "older-flag-name"])]`

   example command-line: `./my_prog --old-flag-name` sets the flag to true

*  <a name="flag-env-aliases"></a> `env_aliases` (string array argument)

   Specifies alternate (fallback) environment variables which should be associated to this flag. These are checked in the order listed, and if a value is found, the later ones are not checked.

   example: `#[arg(env_aliases=["OLD_FLAG_NAME", "OLDER_FLAG_NAME"])]`

   example command-line: `OLD_FLAG_NAME=1 ./my_prog` sets the flag to true

*  <a name="flag-serde"></a> `serde` (optional additional attributes)

   example: `#[conf(serde(rename = "foo"))]`

   Configuration specific to the serde integration.

   * <a name="flag-serde-rename"></a> `rename` (string argument)

     example: `#[conf(serde(rename = "foo"))]`

     Similar to [`#[serde(rename)]`](https://serde.rs/field-attrs.html#rename), changes the name used in serialization, which by default is the field name.

   * <a name="flag-serde-alias"></a> `alias` (string argument, repeating)

     example: `#[conf(serde(alias = "old_name"))]`, `#[conf(serde(alias = "old_name", alias = "older_name"))]`

     Similar to [`#[serde(alias)]`](https://serde.rs/field-attrs.html#alias), adds alternative names that can be used when deserializing from serde documents. This is useful for maintaining backwards compatibility when renaming fields - the main name (from `rename` or the field name) continues to work, and the aliases provide fallback names.

     The `alias` attribute can be specified multiple times to add multiple alternative names. All aliases are treated equally - using any alias in the serde document will populate the field.

     If multiple names (including the main name and any aliases) appear in the same serde document, a duplicate field error is raised.

     **Example**:
     ```rust
     # use conf::Conf;
     # #[cfg(feature = "serde")]
     # {
     #[derive(Conf)]
     #[conf(serde)]
     pub struct Config {
         #[arg(long, env, serde(rename = "new_name", alias = "old_name", alias = "legacy_name"))]
         pub enabled: bool,
     }
     # }
     ```
     This allows the field to be read from serde documents using any of: `"new_name"`, `"old_name"`, or `"legacy_name"`.

   * <a name="flag-serde-deserialize-with"></a> `deserialize_with` (path argument)

     example: `#[conf(serde(deserialize_with = "path::to::deserialize_fn"))]`

     Similar to `#[serde(deserialize_with)]`, uses a custom deserialization function when reading from serde documents.
     The function must have the signature `fn<'de, D>(D) -> Result<T, D::Error> where D: Deserializer<'de>`.

     This attribute only affects deserialization from serde documents (JSON, TOML, etc.). Values from CLI arguments
     and environment variables are parsed normally and do not use the custom deserializer.

     **Example**:
     ```rust
     # use conf::Conf;
     # #[cfg(feature = "serde")]
     # {
     fn deserialize_bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
     where
         D: serde::Deserializer<'de>,
     {
         use serde::Deserialize;
         let value = i32::deserialize(deserializer)?;
         Ok(value != 0)
     }

     #[derive(Conf)]
     #[conf(serde)]
     pub struct Config {
         #[conf(long, serde(deserialize_with = "deserialize_bool_from_int"))]
         pub enabled: bool,
     }
     # }
     ```

     This attribute is mutually exclusive with `try_from`.

   * <a name="flag-serde-try-from"></a> `try_from` (type string argument)

     example: `#[conf(serde(try_from = "u32"))]`

     Similar to [`#[serde(try_from)]`](https://serde.rs/container-attrs.html#try_from), deserializes an intermediate type and then converts to the target type using `TryFrom::try_from`.

     The type must implement `TryFrom<IntermediateType>` where the error type implements `Display`.

     This attribute only affects deserialization from serde documents. Values from CLI arguments and environment variables are parsed normally.

     This attribute is mutually exclusive with `deserialize_with`.

   * <a name="flag-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to [`#[serde(skip)]`](https://serde.rs/field-attrs.html#skip), this field won't be read from the serde value source.

### Parameter

A parameter represents a single value that can be parsed from a string.

**Requirements**: A parameter field can have any type as long as it implements `FromStr` or `value_parser` is used.

*  <a name="parameter-short"></a> `short` (optional char argument)

   Specifies a short (one-dash) switch associated to this parameter.
   If argument is omitted, defaults to the first letter of the field name.

   example: `#[arg(short)]`, `#[arg(short = 'p')]`

   example command-line: `./my_prog -p foo` or `./my_prog -p=foo` sets the parameter using the string value `foo`

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="parameter-long"></a> `long` (optional string argument)

   Specifies a long (two-dash) switch associated to this parameter.
   If argument is omitted, defaults to the kebab-cased field name.

   example: `#[arg(long)]`, `#[arg(long = "param")]`

   example command-line: `./my_prog --param foo` or `./my_prog --param=foo` sets the parameter using the string value `foo`

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="parameter-pos"></a> `pos` (no arguments)

   Specifies that this parameter is a positional argument.
   Positional arguments are identified by their position in the command line, not by a flag name.
   The order of positional arguments is determined by the order of fields in the struct.

   example: `#[arg(pos)]`

   example command-line: `./my_prog input.txt output.txt` where the first positional is assigned to the first `pos` field, the second to the second `pos` field, etc.

   **Positional argument filling**:
   - Positional arguments are filled left-to-right based on their declaration order in the struct
   - You cannot skip a positional argument to provide a later one

   **Optional positionals**:
   - A positional parameter can be optional by using `Option<T>` as the field type
   - All optional positional arguments must come after all required positional arguments

   **Compatibility**:
   - `pos` is mutually exclusive with `short` and `long`
   - `pos` is not supported in `flatten` with `Option<T>` (flatten optional) - this will produce an error

   **Examples**:

   Valid configuration:
   ```rust,compile
   use conf::Conf;
   #[derive(Conf)]
   struct MyConfig {
       /// Input file (required positional)
       #[conf(pos)]
       input: String,

       /// Output file (required positional)
       #[conf(pos)]
       output: String,

       /// Optional log file (optional positional)
       #[conf(pos)]
       log_file: Option<String>,
   }
   ```

   Command-line usage: `./my_prog input.txt output.txt` or `./my_prog input.txt output.txt debug.log`

   Invalid configuration (will panic at runtime):
   ```rust,should_panic
   use conf::Conf;
   #[derive(Conf)]
   struct BadConfig {
       #[conf(pos)]
       first: String,

       #[conf(pos)]
       second: Option<String>,  // optional

       #[conf(pos)]
       third: String,  // ERROR: required after optional!
   }

   // This will panic when we try to parse
   let _ = BadConfig::try_parse_from::<&str, &str, &str>(
       vec!["prog", "a", "b"],
       vec![],
   );
   ```

*  <a name="parameter-env"></a> `env` (optional string argument)

   Specifies an environment variable associated to this parameter.
   If argument is omitted, defaults to the upper snake-case field name.

   example: `#[arg(env)]`, `#[arg(env = "PARAM")]`

   example command-line: `PARAM=foo ./my_prog` sets the parameter using the string value `foo`

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="parameter-aliases"></a> `aliases` (string array argument)

   Specifies alternate long switches that should be an alias for this parameter.
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.visible_aliases)

   example: `#[arg(aliases=["old-param-name", "older-param-name"])]`

   example command-line: `./my_prog --old-param-name foo` or `./my_prog --old-param-name=foo` sets the parameter using the string value `foo`

*  <a name="parameter-env-aliases"></a> `env_aliases` (string array argument)

   example: `#[arg(env_aliases=["OLD_PARAM_NAME", "OLDER_PARAM_NAME"])]`

   Specifies alternate (fallback) environment variables which should be associated to this parameter. These are checked in the order listed, and if a value is found, the later ones are not checked.

*  <a name="parameter-default-value"></a> `default_value` (string argument)

   example: `#[arg(default_value = "some value")]`

   Specifies the default value assigned to this parameter if none of the switches or env are present.

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="parameter-default-help-str"></a> `default_help_str` (string argument)

   example: `#[arg(default_value = "actual_value", default_help_str = "displayed in help")]`

   Overrides the use of `default_value` in the help text. This allows displaying a different string in `--help` than the actual default value used for initialization.

   **Example**:
   ```rust
   # use conf::Conf;
   #[derive(Conf)]
   struct Config {
       /// API endpoint URL
       #[conf(long, default_value = "https://api.example.com/v2/data", default_help_str = "<default endpoint>")]
       api_url: String,

       /// Connection timeout
       #[conf(long, default_value = "30", default_help_str = "30 seconds")]
       timeout: u32,
   }
   ```

   In the help output, users will see `<default endpoint>` and `30 seconds` as the default values, while the actual initialization will use `"https://api.example.com/v2/data"` and `"30"` respectively.

*  <a name="parameter-value-parser"></a> `value_parser` (expr argument)

   example: `#[arg(value_parser = my_function)]`

   By default, `conf` invokes the trait function `std::str::FromStr::from_str` to convert the parsed string to the type of the field.
   This can be overrided by setting `value_parser`. Any function expression can be used as long as any generic parameters are either specified or inferred.

   *Note*: This is very similar to `clap-derive`, but there are technical differences [^compat-note-2].

*  <a name="parameter-value-parser-os"></a> `value_parser_os` (expr argument)

   example: `#[arg(value_parser_os = my_osstr_function)]`

   Similar to `value_parser`, but the parser function receives `&OsStr` instead of `&str`. This allows parsing values that may contain non-UTF-8 data in args or env, especially paths on Windows.

   The parser function should have signature `fn(&OsStr) -> Result<T, E>` where `E` implements `Display`.

   **Auto-detection**: `conf` defaults to using an appropriate OsStr-based parser for `PathBuf` and `OsString` types. This means you typically don't need to specify `value_parser_os` explicitly for these types in order to accept non-UTF data.

   **Examples**:

   Auto-detected for PathBuf:
   ```rust
   # use conf::Conf;
   use std::path::PathBuf;

   #[derive(Conf)]
   struct Config {
       /// Input file path (automatically uses OsStr parser)
       #[conf(long, env)]
       input: PathBuf,
   }
   ```

   Custom OsStr parser:
   ```rust
   # use conf::Conf;
   # use std::ffi::{OsStr, OsString};
   # use std::fmt::Display;
   # use std::path::{PathBuf, Path};

   fn strip_home_path(s: &OsStr) -> Result<PathBuf, impl Display> {
        Path::new(s).strip_prefix("/home/").map(Path::to_path_buf)
   }

   #[derive(Conf)]
   struct Config {
       #[conf(long, value_parser_os = strip_home_path)]
       text: PathBuf,
   }
   ```

   *Note*: `value_parser_os` and `value_parser` are mutually exclusive.

*  <a name="parameter-allow-hyphen-values"></a> `allow_hyphen_values` (no arguments)

   example: `#[arg(allow_hyphen_values)]`

   By default, clap's parser considers a leading hyphen in a parameter value like `--my-param --my-value` to be an error, and that the user more likely forgot to give a value to `--my-param`
   and tried to specify a switch `--my-value` afterwards, than that they intended to give the value `--my-value` to `--my-param`. So the default behavior is to give an error in that case.

   If you actually intended to set `--my-param` to the value `--my-value`, you can instead write `--my-param=--my-value`, or set it via an environment variable, which doesn't care about this setting.
   If you set `allow_hyphen_values` then this check is not applied, and `--my-param --my-value` gets parsed the same as `--my-param=--my-value`.

   This corresponds to [`clap::Arg::allow_hyphen_values`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.allow_hyphen_values)

*  <a name="parameter-allow-negative-numbers"></a> `allow_negative_numbers` (no arguments)

   example: `#[arg(allow_negative_numbers)]`

   This corresponds to [`clap::Arg::allow_negative_numbers`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.allow_negative_numbers)

   Similar to `allow_hyphen_values`, but specifically allows negative number values like `-42` or `-3.14` while still treating other hyphenated values like `--my-value` as potential errors.

   This is a more conservative option than `allow_hyphen_values` because it only permits values that look like negative numbers (start with `-` or `+` followed by digits), not arbitrary hyphenated strings.

   `allow_negative_numbers` is automatically enabled when the field type is a signed number type (`i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `f32`, `f64`), making it unnecessary to specify this attribute explicitly in most cases.

*  <a name="parameter-default-if-missing"></a> `default_if_missing` (string argument)

   example: `#[arg(default_if_missing = "80")]`

   This corresponds to [`clap::Arg::default_missing_value`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.default_missing_value)

   Specifies the default value used when the switch appears on the command-line without a value. This only applies when the switch appears without a value following it, not when the switch is completely absent.

   **Note**: This can only be used with parameters with short/long forms, not positional arguments.

   **Example**:
   ```rust
   # use conf::Conf;
   #[derive(Conf)]
   struct Config {
       /// Server port
       #[conf(long, default_if_missing = "80")]
       port: u16,
   }
   ```

   With this configuration:
   - `./app` - An error because `port` is required and not specified
   - `./app --port` - `port` will be `80` (from `default_if_missing`)
   - `./app --port 8080` - `port` will be `8080` (from the command line)

*  <a name="parameter-default"></a> `default` (optional expression argument)

   example: `#[arg(default)]`, `#[arg(default(42))]`, `#[arg(default(vec![1, 2, 3]))]`

   Specifies a default value using a Rust expression (rather than a string).

   Comparable to [`default_value_t`](https://docs.rs/clap/4.5.8/clap/_derive/index.html#default-values) in `clap`, and [`default`](https://serde.rs/field-attrs.html#default) in serde.

   **Basic usage**:
   - `#[arg(default)]` - Uses `Default::default()` as the default value
   - `#[arg(default(expr))]` - Uses the provided expression as the default value

   For `Option<T>` fields, the expression must produce type `T` (not `Option<T>`).

   Unlike `default_value`, the `default` expression completely bypasses the `value_parser`.

   **Help text**: When using `default`, the help text that documents the default value is generated
   using the `Display` trait applied to the default value. If the type doesn't implement `Display`, then
   `default_help_str` must be provided instead.

   **Examples**:

   Default with a literal value:
   ```rust
   # use conf::Conf;
   #[derive(Conf)]
   struct Config {
       /// Port number
       #[conf(long, default(8080))]
       port: u16,
   }
   ```

   Default with an expression:
   ```rust
   # use conf::Conf;
   #[derive(Conf)]
   struct Config {
       /// Computed default (help string will be "15")
       #[conf(long, default(10 + 5))]
       threshold: i32,
   }
   ```

   **Note**: This attribute is mutually exclusive with `default_value`.

*  <a name="parameter-secret"></a> `secret` (optional bool argument)

   example: `#[conf(secret)]`, `#[conf(secret = true)]`, `#[conf(secret = false)]`

   Indicates that this value is secret and `conf` should avoid logging the value if there is an error.

   If the `bool` argument is not specified when this attribute appears, it is considered `true`.
   Values not marked secret are considered not to be secrets.

*  <a name="parameter-serde"></a> `serde` (optional additional attributes)

   example: `#[conf(serde(use_value_parser, rename = "foo"))]`

   Configuration specific to the serde integration.

   * <a name="parameter-serde-rename"></a> `rename` (string argument)

     example: `#[conf(serde(rename = "foo"))]`

     Similar to [`#[serde(rename)]`](https://serde.rs/field-attrs.html#rename), changes the name used in serialization, which by default is the field name.

   * <a name="parameter-serde-alias"></a> `alias` (string argument, repeating)

     example: `#[conf(serde(alias = "old_name"))]`, `#[conf(serde(alias = "old_name", alias = "older_name"))]`

     Similar to [`#[serde(alias)]`](https://serde.rs/field-attrs.html#alias), adds alternative names that can be used when deserializing from serde documents. The `alias` attribute can be specified multiple times to add multiple alternative names. See [flag serde alias](#flag-serde-alias) for more details.

   * <a name="parameter-serde-deserialize-with"></a> `deserialize_with` (path argument)

     example: `#[conf(serde(deserialize_with = "path::to::deserialize_fn"))]`

     Similar to [`#[serde(deserialize_with)]`](https://serde.rs/field-attrs.html#deserialize-with), uses a custom deserialization function when reading from serde documents.
     The function must have the signature `fn<'de, D>(D) -> Result<T, D::Error> where D: Deserializer<'de>`.

     This attribute only affects deserialization from serde documents (JSON, TOML, etc.). Values from CLI arguments
     and environment variables are parsed normally and do not use the custom deserializer.

     **Example**:
     ```rust
     # use conf::Conf;
     # #[cfg(feature = "serde")]
     # {
     fn deserialize_doubled<'de, D>(deserializer: D) -> Result<i32, D::Error>
     where
         D: serde::Deserializer<'de>,
     {
         use serde::Deserialize;
         let value = i32::deserialize(deserializer)?;
         Ok(value * 2)
     }

     #[derive(Conf)]
     #[conf(serde)]
     pub struct Config {
         #[arg(long, serde(deserialize_with = "deserialize_doubled"))]
         pub count: i32,
     }
     # }
     ```

     This attribute is mutually exclusive with `use_value_parser` and `try_from`.

   * <a name="parameter-serde-try-from"></a> `try_from` (type string argument)

     example:
     ```rust ignore
     #[conf(serde(try_from = "U"))]
     t: T,
     ```

     Similar to [`#[serde(try_from)]`](https://serde.rs/container-attrs.html#try_from), deserializes an intermediate type (`U`) and then converts to the target type (`T`) using `TryFrom::try_from`.

     The type `T` must implement `TryFrom<U>` where the error type implements `Display`.

     This can be simpler or more convenient than using `deserialize_with`, especially when `U` has a simple representation in serde.

     This attribute only affects deserialization from serde documents. Values from CLI arguments and environment variables are parsed normally using `FromStr` or `value_parser`.

     **Note**: For `Option<T>` fields with `try_from = "U"`: deserializes `Option<U>` and converts via `.map(TryFrom::try_from)`. This allows the field to be optional in JSON (null or missing), and the same `TryFrom<U> for T` impl works for both optional and required fields. If your `TryFrom` implementation doesn't line up with this, you can use `deserialize_with` to make it fit.

     **Example**:
     ```rust
     # use conf::Conf;
     # #[cfg(feature = "serde")]
     # {
     #[derive(Debug, Clone)]
     pub struct PositiveNumber(u32);

     impl TryFrom<u32> for PositiveNumber {
         type Error = &'static str;
         fn try_from(value: u32) -> Result<Self, Self::Error> {
             if value > 0 { Ok(PositiveNumber(value)) }
             else { Err("number must be positive") }
         }
     }

     impl std::str::FromStr for PositiveNumber {
         type Err = String;
         fn from_str(s: &str) -> Result<Self, Self::Err> {
             let value: u32 = s.parse().map_err(|e| format!("{}", e))?;
             PositiveNumber::try_from(value).map_err(|e| e.to_string())
         }
     }

     #[derive(Conf)]
     #[conf(serde)]
     pub struct Config {
         // Required field
         #[conf(long, serde(try_from = "u32"))]
         pub count: PositiveNumber,

         // Optional field - deserializes Option<u32>, converts inner value
         #[conf(long, serde(try_from = "u32"))]
         pub limit: Option<PositiveNumber>,
     }
     # }
     ```

     Mutually exclusive with `use_value_parser` and `deserialize_with`.

   * <a name="parameter-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to [`#[serde(skip)]`](https://serde.rs/field-attrs.html#skip), this field won't be read from the serde value source.
     This can be useful if the type doesn't implement `serde::Deserialize`.

   * <a name="parameter-serde-use-value-parser"></a> `use_value_parser` (no arguments)

     example: `#[conf(serde(use_value_parser))]`

     If used, then instead of asking `serde` to deserialize the field type, `serde` will deserialize a `String`,
     and then the `value_parser` will convert the string to the field type.

     Mutually exclusive with `deserialize_with` and `try_from`.

*  <a name="parameter-test"></a> `test` (supports nested options)

   Configures test-related behavior for this parameter field.

   * <a name="parameter-test-skip-default-value"></a> `skip_default_value` (no arguments)

     example: `#[conf(test(skip_default_value))]`

     Skip testing the `default_value` for this parameter during `#[conf(test)]` validation.
     This might be useful when the `value_parser` has side-effects or reads files from the file-system
     that might not be there during the test.

#### Notes

When a parameter is parsed from CLI arguments, the parser expects one of two syntaxes `--param=value` or `--param value` to be used. If `value` is not found as expected then parsing will fail.
If `--param` appears twice then parsing will fail. Short switches and long switches behave similarly in this regard.

As in `clap`, if a parameter's field type is `Option<T>`, it has special meaning:

* The parameter is not considered required. If it is omitted, parsing will succeed and the value will be `None`.
* If parsing does produce a string, then the value will be `Some`.
* If a `value_parser` is specified, it should produce `T` rather than `Option<T>`.
* The option will not be considered required when rendering the help text.

Currently none of the other special [type-based intent inferences that clap does](https://docs.rs/clap/4.5.8/clap/_derive/index.html#arg-types) are implemented in this crate, but there are alternative, more explicit ways to get the behavior:

* `bool` is by default a flag rather than a parameter
* `Option<T>` is an optional parameter
* `Option<Option<T>>` is not directly supported, instead you should use `Option<T>` and the `default_if_missing` attribute.
* `T` is a required parameter
* `Vec<T>` is not special -- if you want it to be a repeating field, as in clap, you must make it a `repeat` field explicitly. There are legitimate reasons that you might want, e.g. a parameter of type `Vec<T>` whose `value_parser` is `serde_json::from_str`. Making this implicitly a repeat field can be very surprising.
* `Option<Vec<T>>` is not supported at this time for `repeat` fields.

### Repeat

A repeat field is similar to a parameter, except that it may appear multiple times on the command line, and the collection of string arguments are then parsed individually and aggregated.

**Requirements**: A repeat field must have type `Vec<T>`, where `T` implements `FromStr`, or `value_parser` must be supplied that produces a `T`.

*Note*: A repeat option produces one `T` for each time the option appears in the CLI arguments, and unlike a parameter the option can appear multiple times. If it does not appear, and an `env` variable is specified, then that variable
is read and split on a delimiter character which defaults to `','`, to produce a series of `T` values.

*  <a name="repeat-short"></a> `short` (optional char argument)

   Specifies a short (one-dash) switch associated to this repeat option.
   If argument is omitted, defaults to the first letter of the field name.

   example: `#[arg(repeat, short)]`, `#[arg(repeat, short = 'p')]`

   example command-line: `./my_prog -p peer1 -p peer2`

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="repeat-long"></a> `long` (optional string argument)

   Specifies a long (two-dash) switch associated to this option.
   If argument is omitted, defaults to the kebab-cased field name.

   example: `#[arg(repeat, long)]`, `#[arg(repeat, long = "peer")]`

   example command-line: `./my_prog --peer peer1 --peer peer2`

   *Note*: This behavior of this attribute is the same as in `clap-derive`.

*  <a name="repeat-pos"></a> `pos` (no arguments)

   Marks this repeat field as a positional argument that accepts multiple values.

   When combined with `repeat`, this allows you to accept multiple positional arguments.
   The field must have type `Vec<T>` where `T` implements `FromStr`, or `value_parser` must be supplied.

   - Cannot be combined with `long` or `short` (positional arguments have no switches)
   - No other positional arguments can appear after a repeat positional argument

   **Examples**:

   Simple repeat positional - accept zero or more file paths:
   ```rust
   # use conf::Conf;
   #[derive(Conf)]
   struct Config {
       /// Files to process
       #[conf(repeat, pos)]
       files: Vec<String>,
   }

   let result = Config::try_parse_from::<&str, &str, &str>(
       vec!["prog", "file1.txt", "file2.txt", "file3.txt"],
       vec![],
   ).unwrap();
   assert_eq!(result.files, vec!["file1.txt", "file2.txt", "file3.txt"]);
   ```

   Combined with regular positional - first arg is command, rest are arguments:
   ```rust
   # use conf::Conf;
   #[derive(Conf)]
   struct Config {
       /// Command to run
       #[conf(pos)]
       command: String,

       /// Arguments to pass to the command
       #[conf(repeat, pos, allow_hyphen_values)]
       args: Vec<String>,
   }

   let result = Config::try_parse_from::<&str, &str, &str>(
       vec!["prog", "ls", "-l", "-a", "/tmp"],
       vec![],
   ).unwrap();
   assert_eq!(result.command, "ls");
   assert_eq!(result.args, vec!["-l", "-a", "/tmp"]);
   ```

   With environment variable fallback:
   ```rust
   # use conf::Conf;
   #[derive(Conf)]
   struct Config {
       /// Files to process (can be from args or env)
       #[conf(repeat, pos, env)]
       files: Vec<String>,
   }

   // From command line
   let result = Config::try_parse_from::<&str, &str, &str>(
       vec!["prog", "a.txt", "b.txt"],
       vec![],
   ).unwrap();
   assert_eq!(result.files, vec!["a.txt", "b.txt"]);

   // From environment variable (comma-delimited by default)
   let result = Config::try_parse_from::<&str, &str, &str>(
       vec!["prog"],
       vec![("FILES", "x.txt,y.txt")],
   ).unwrap();
   assert_eq!(result.files, vec!["x.txt", "y.txt"]);
   ```

   Command-line usage examples:
   - `./my_prog` - no files
   - `./my_prog file1.txt` - one file
   - `./my_prog file1.txt file2.txt file3.txt` - multiple files
   - `FILES=a.txt,b.txt ./my_prog` - from environment variable

*  <a name="repeat-env"></a> `env` (optional string argument)

   Specifies an environment variable associated to this option.
   If omitted, defaults to the upper snake-cased field name.

   example: `#[arg(repeat, env)]`, `#[arg(repeat, env = "PEERS")]`

   example command-line: `PEERS=peer1,peer2 ./my_prog`

   *Note*: The behavior of this attribute is the same as in `clap-derive`.

*  <a name="repeat-aliases"></a> `aliases` (string array argument)

   Specifies alternate long switches that should be an alias for this option.
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.visible_aliases)

   example: `#[arg(repeat, aliases=["old-param-name", "older-param-name"])]`

*  <a name="repeat-env-aliases"></a> `env_aliases` (string array argument)

   Specifies alternate (fallback) environment variables which should be associated to this option. These are checked in the order listed, and if a value is found, the later ones are not checked.

   example: `#[arg(repeat, env_aliases=["OLD_PARAM_NAME", "OLDER_PARAM_NAME"])]`

*  <a name="repeat-env-delimiter"></a> `env_delimiter` (char argument)

   Controls what character is used as a delimiter when reading the list from an environment variable.

   If omitted, the default is `,`.

   example: `[conf(env_delimiter = '|')]`

   example command-line: `PEERS=peer1|peer2 ./my_prog`

   *Note*: This doesn't have a direct analog in `clap-derive`, but as far as `env` is concerned it's like [`value_delimiter`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.value_delimiter).

*  <a name="repeat-no-env-delimiter"></a> `no_env_delimiter` (no argument)

   If set, then the env is parsed as if it is a single `T` and not a list. This can be used for strict compatibility with common `clap` configurations.

   example: `[conf(no_env_delimiter)]`

*  <a name="repeat-value-parser"></a> `value_parser` (expr argument)

   example: `#[arg(repeat, value_parser = serde_json::from_str)]`

   By default, `conf` invokes the trait function `FromStr::from_str` to convert each string to the type `T`.
   This can be overrided by setting `value_parser`. Any function expression can be used as long as it produces a `T` and any generic parameters are either specified or inferred.

   **Environment variable handling**: When the list is read from env, it is split first on the `env_delimiter`, by default ',',
   unless `no_env_delimiter` is used.

   **Note**: This behavior is the same as in `clap-derive`, and with respect to `env`, when `no_env_delimiter` is used.

*  <a name="repeat-value-parser-os"></a> `value_parser_os` (expr argument)

   example: `#[arg(repeat, value_parser_os = my_osstr_function)]`

   Similar to `value_parser`, but the parser function receives `&OsStr` instead of `&str`.

   The parser function should have signature `fn(&OsStr) -> Result<T, E>` where `E` implements `Display`.

   **Auto-detection**: Like with parameters, `conf` defaults to an appropriate `OsStr`-based parser for `Vec<PathBuf>` and `Vec<OsString>` types, when no `value_parser` or `value_parser_os` is specified.

   **Environment variable handling**: When using `value_parser_os`, the delimiter is restricted to ASCII characters. This is because `OsStr` represents a platform-specific encoding and only splitting by ASCII is supported.

   **Examples**:

   Auto-detected for `Vec<PathBuf>`:
   ```rust
   # use conf::Conf;
   use std::path::PathBuf;

   #[derive(Conf)]
   struct Config {
       /// Input files (supports UTF-16 filenames on windows)
       #[conf(repeat, long, env)]
       inputs: Vec<PathBuf>,
   }
   ```

   With custom delimiter:
   ```rust
   # use conf::Conf;
   use std::path::PathBuf;

   #[derive(Conf)]
   struct Config {
       /// Input files, colon-separated in env (like PATH)
       #[conf(repeat, long, env, env_delimiter = ':')]
       inputs: Vec<PathBuf>,
   }
   ```

   This attribute is mutually exclusive with `value_parser`.

*  <a name="repeat-allow-hyphen-values"></a> `allow_hyphen_values` (no arguments)

   example: `[arg(allow_hyphen_values)]`

   By default, clap's parser considers a leading hyphen in a parameter value like `--my-param --my-value` to be an error, and that the user more likely forgot to give a value to `--my-param`
   and tried to specify a switch `--my-value` afterwards, than that they intended to give the value `--my-value` to `--my-param`. So the default behavior is to give an error in that case.

   If you actually intended to set `--my-param` to the value `--my-value`, you can instead write `--my-param=--my-value`, or set it via an environment variable, which doesn't care about this setting.
   If you set `allow_hyphen_values` then this check is not applied, and `--my-param --my-value` gets parsed the same as `--my-param=--my-value`.

   This corresponds to [`clap::Arg::allow_hyphen_values`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.allow_hyphen_values)

*  <a name="repeat-allow-negative-numbers"></a> `allow_negative_numbers` (no arguments)

   example: `#[arg(repeat, allow_negative_numbers)]`

   This corresponds to [`clap::Arg::allow_negative_numbers`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.allow_negative_numbers)

   Similar to `allow_hyphen_values`, but specifically allows negative number values like `-42` or `-3.14` while still treating other hyphenated values like `--my-value` as potential errors.

   This is a more conservative option than `allow_hyphen_values` because it only permits values that look like negative numbers (start with `-` or `+` followed by digits), not arbitrary hyphenated strings.

   `allow_negative_numbers` is automatically enabled when the inner type `T` of `Vec<T>` is a signed number type (`i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `f32`, `f64`), making it unnecessary to specify this attribute explicitly in most cases.

*  <a name="repeat-secret"></a> `secret` (optional bool argument)

   example: `#[conf(secret)]`, `#[conf(secret = false)]`

   Indicates that this value is secret and `conf` should avoid logging the value if there is an error.

   If the `bool` argument is not specified when this attribute appears, it is considered `true`.
   Values not marked secret are considered not to be secrets.

*  <a name="repeat-serde"></a> `serde` (optional additional attributes)

   example: `#[conf(repeat, serde(...))]`

   Configuration specific to the serde integration.

   * <a name="repeat-serde-rename"></a> `rename` (string argument)

     example: `#[conf(repeat, serde(rename = "foos"))]`

     Similar to [`#[serde(rename)]`](https://serde.rs/field-attrs.html#rename), changes the name used in serialization, which by default is the field name.

   * <a name="repeat-serde-alias"></a> `alias` (string argument, repeating)

     example: `#[conf(serde(alias = "old_name"))]`, `#[conf(serde(alias = "old_name", alias = "older_name"))]`

     Similar to [`#[serde(alias)]`](https://serde.rs/field-attrs.html#alias), adds alternative names that can be used when deserializing from serde documents. The `alias` attribute can be specified multiple times to add multiple alternative names. See [flag serde alias](#flag-serde-alias) for more details.

   * <a name="repeat-serde-deserialize-with"></a> `deserialize_with` (path argument)

     example: `#[conf(serde(deserialize_with = "path::to::deserialize_fn"))]`

     Similar to [`#[serde(deserialize_with)]`](https://serde.rs/field-attrs.html#deserialize-with), uses a custom deserialization function when reading from serde documents.
     The function must have the signature `fn<'de, D>(D) -> Result<Vec<T>, D::Error> where D: Deserializer<'de>`.

     This attribute only affects deserialization from serde documents (JSON, TOML, etc.). Values from CLI arguments
     and environment variables are parsed normally and do not use the custom deserializer.

     **Example**:
     ```rust
     # use conf::Conf;
     # #[cfg(feature = "serde")]
     # {
     fn deserialize_vec_reversed<'de, D>(deserializer: D) -> Result<Vec<i32>, D::Error>
     where
         D: serde::Deserializer<'de>,
     {
         use serde::Deserialize;
         let mut value = Vec::<i32>::deserialize(deserializer)?;
         value.reverse();
         Ok(value)
     }

     #[derive(Conf)]
     #[conf(serde)]
     pub struct Config {
         #[conf(repeat, long, serde(deserialize_with = "deserialize_vec_reversed"))]
         pub items: Vec<i32>,
     }
     # }
     ```

     This attribute is mutually exclusive with `use_value_parser` and `try_from`.

   * <a name="repeat-serde-try-from"></a> `try_from` (type string argument)

     example: `#[conf(repeat, long, serde(try_from = "u32"))]`

     Similar to [`#[serde(try_from)]`](https://serde.rs/container-attrs.html#try_from), deserializes an intermediate type and then converts to the target type using `TryFrom::try_from`.

     For `Vec<T>` fields with `try_from = "U"`: deserializes `Vec<U>` and converts each element via `TryFrom::try_from`.

     This can be simpler or more convenient than using `deserialize_with`, especially when `U` has a simple representation in serde.

     Mutually exclusive with `deserialize_with` and `use_value_parser`.

     **Example**:
     ```rust
     # use conf::Conf;
     # #[cfg(feature = "serde")]
     # {
     # #[derive(Debug, Clone)]
     # pub struct PositiveNumber(u32);
     # impl TryFrom<u32> for PositiveNumber {
     #     type Error = &'static str;
     #     fn try_from(value: u32) -> Result<Self, Self::Error> {
     #         if value > 0 { Ok(PositiveNumber(value)) }
     #         else { Err("number must be positive") }
     #     }
     # }
     # impl std::str::FromStr for PositiveNumber {
     #     type Err = String;
     #     fn from_str(s: &str) -> Result<Self, Self::Err> {
     #         let value: u32 = s.parse().map_err(|e| format!("{}", e))?;
     #         PositiveNumber::try_from(value).map_err(|e| e.to_string())
     #     }
     # }
     #[derive(Conf)]
     #[conf(serde)]
     pub struct Config {
         // Vec field - deserializes Vec<u32>, converts each element
         #[conf(repeat, long, serde(try_from = "u32"))]
         pub counts: Vec<PositiveNumber>,
     }
     # }
     ```

   * <a name="repeat-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to [`#[serde(skip)]`](https://serde.rs/field-attrs.html#skip), this field won't be read from the serde value source.
     This can be useful if the value doesn't implement `serde::Deserialize`.

   * <a name="repeat-serde-use-value-parser"></a> `use_value_parser` (no arguments)

     example: `#[conf(serde(use_value_parser))]`

     If used, then instead of asking `serde` to deserialize `Vec<T>`, `serde` will deserialize a `Vec<String>`,
     and then the `value_parser` will convert each string to `T`. The default `value_parser` is `FromStr`.

     Mutually exclusive with `deserialize_with` and `try_from`.

#### Notes

`clap-derive`'s multi-option's don't work that well in a 12-factor app, because there's a mismatch between, getting multiple strings from the CLI arguments, and getting one string from env.

`clap-derive`'s behavior for a typical case like

```rust ignore
   #[clap(long, env)]
   my_list: Vec<String>,
```

when there is no CLI arg and only env is set, is that the entire env value becomes the one element of `my_list`, and there is no way to configure a list with multiple items by setting only `env`.
So, most likely an app that was using `clap` this way was only using the CLI arguments to configure this value. For `conf`, we consider that this is not a good default behavior.

`clap` does have an additional option for this case called [`value_delimiter`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.value_delimiter), which will cause it to split both CLI arguments and `env` values on a given character.
In `conf` however, at this point the field can just be `parameter` instead of a `repeat`, and a `value_parser` can be used which does the splitting.
So we don't provide the `value_delimiter` feature here.

The main reasons that we provide `repeat` are:

* Ease of migrating an existing `clap-derive` parser that may use the multi-option stuff
* It can be easier to read CLI args, for example in a shell script, when a list is split into many args rather than having one very long list arg.

If your goal is compatiblity with an existing `clap-derive` parser that parses a `Vec` has no [`value_delimiter`](https://docs.rs/clap/4.5.8/clap/struct.Arg.html#method.value_delimiter), you should use `repeat` with `no_env_delimiter`.

If you are making a new option and you want the repeat style of CLI argument parsing, the default for a `repeat` option is `env_delimiter=','`, which preserves your ability to configure via `env`,
and you can customize this if another choice of delimiter is more appropriate.

### Flatten

**Requirements**: A flatten field's type must be `T` or` Option<T>` where `T: Conf`.

*  <a name="flatten-env-prefix"></a> `env_prefix` (optional string argument)

   example: `#[conf(flatten, env_prefix = "AUTH_")]`

   Specifies a string to be prepended to every environment variable of every program option in the target struct.
   If the argument is omitted, it defaults to the upper snake-case of the field name, with an `_` character appended.

*  <a name="flatten-long-prefix"></a> `long_prefix` (optional string argument)

   example: `#[conf(flatten, long_prefix = "auth-")]`

   Specifies a string to be prepended to every long switch of every program option in the target struct.
   If the argument is omitted, it defaults to the kebab-case of the field name, with a `-` character appended.

*  <a name="flatten-prefix"></a> `prefix` (optional string argument)

   example: `#[conf(flatten, prefix = "auth")]`

   Specifies a string to be used in place of the field name in the default constructions of `env_prefix` and `long_prefix`.
   If the argument is omitted, it's the same as specifying `env_prefix` and `long_prefix` both with no argument.

   This option cannot be used if `env_prefix` or `long_prefix` is present.
   If none of these options are used, then no prefixing occurs.

*  <a name="flatten-help-prefix"></a> `help_prefix` (optional string argument)

   example: `#[conf(flatten, help_prefix = "(friend service)")]`

   Specifies that the help strings of every program option of the target struct should be prefixed with a particular string,
   to provide context. If the argument is omitted, it defaults to the doc string on this field.
   If the `help_prefix` attribute is not present then the help strings are unmodified.

   When prefixing is performed, some very simple logic is used to determine how to join the prefix.
   If either the prefix or the help string has multiple lines, then a newline character is used to join them.
   Otherwise a space character is used to join them. (This may change in future revisions.)

*  <a name="flatten-skip-short"></a> `skip_short` (char array argument)

   example: `#[conf(flatten, skip_short = ['a', 'b', 'f'])]`

   A list of short forms of options which should be skipped when options are flattened at this site.

   There is no way to prefix a short form -- it can only be one character. `skip_short` is a method to resolve conflicts when flattening.

   This should only be used as a last resort if you cannot simply remove one of the conflicting short forms at its source, because it would break something else.

   To try to help maintainability in a large project, if `skip_short` would have no effect, it is an error rather than silently
   continuing. That is, it is an error if a `skip_short` attribute is used but the named short flag is not found
   at this flattening site. So, when you see this attribute appearing, you can be sure that all of the named short flags are actually being removed at this location,
   and not by some other `skip_short` attribute appearing in another location.

*  <a name="flatten-serde"></a> `serde` (optional additional attributes)

   example: `#[conf(flatten, serde(...)))]`

   Configuration specific to the serde integration.

   * <a name="flatten-serde-rename"></a> `rename` (string argument)

     example: `#[conf(flatten, serde(rename = "foo"))]`

     Similar to [`#[serde(rename)]`](https://serde.rs/field-attrs.html#rename), changes the name used in serialization, which by default is the field name.

   * <a name="flatten-serde-alias"></a> `alias` (string argument, repeating)

     example: `#[conf(flatten, serde(alias = "old_name"))]`, `#[conf(flatten, serde(alias = "old_name", alias = "older_name"))]`

     Similar to [`#[serde(alias)]`](https://serde.rs/field-attrs.html#alias), adds alternative names that can be used when deserializing from serde documents. The `alias` attribute can be specified multiple times to add multiple alternative names. See [flag serde alias](#flag-serde-alias) for more details.

   * <a name="flatten-serde-try-from"></a> `try_from` (type string argument)

     example:
     ```rust ignore
     #[conf(flatten, serde(try_from = "U"))]
     t: T,
     ```

     Similar to [`#[serde(try_from)]`](https://serde.rs/container-attrs.html#try_from), deserializes an intermediate type (`U`) and then converts to the target type (`T`) using `TryFrom::try_from`.

     **Important**: The intermediate type `U` must implement `ConfSerde` (i.e., it must be a `#[derive(Conf)] #[conf(serde)]` struct). This ensures that CLI arguments and environment variables can still override values from the serde document for fields within the flattened type.

     This can be useful if you want to transform a group of values after hierarchical config resolution.

     This attribute is mutually exclusive with `serde(flatten)`.

   * <a name="flatten-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(flatten, serde(skip))]`

     Similar to [`#[serde(skip)]`](https://serde.rs/field-attrs.html#skip), this substructure won't be read from the serde value source.

   * <a name="flatten-serde-flatten"></a> `flatten` (takes optional parenthetical)

     example: `#[conf(flatten, serde(flatten))]`

     Similar to [`#[serde(flatten)]`](https://serde.rs/attr-flatten.html), the fields of the child struct are inlined into the parent during serde deserialization.
     Without this attribute, the child struct is expected to appear as a nested object in the serde document.
     With this attribute, all fields from the child appear at the same level as the parent's fields.

     **Note**: `conf`'s implementation of `serde(flatten)` doesn't have the same limitations as stock `serde(flatten)` --
     `deny-unknown-fields` works fine (and is on by default), there is no ["internal buffering"](https://github.com/serde-rs/serde/issues/2186#issue-1163413259),
     and it works with any number of flattened or nested flattened fields.

     **Example without `serde(flatten)`**:
     ```rust
     # use conf::Conf;
     # #[cfg(feature = "serde")]
     # {
     #[derive(Conf)]
     #[conf(serde)]
     pub struct DatabaseConfig {
         #[arg(long, env)]
         pub host: String,
         #[arg(long, env)]
         pub port: u16,
     }

     #[derive(Conf)]
     #[conf(serde)]
     pub struct AppConfig {
         #[arg(long, env)]
         pub name: String,
         #[conf(flatten)]
         pub database: DatabaseConfig,
     }
     # }
     ```
     Expected JSON structure (nested):
     ```json
     {
       "name": "myapp",
       "database": {
         "host": "localhost",
         "port": 5432
       }
     }
     ```

     **Example with `serde(flatten)`**:
     ```rust
     # use conf::Conf;
     # #[cfg(feature = "serde")]
     # {
     #[derive(Conf)]
     #[conf(serde)]
     pub struct DatabaseConfig {
         #[arg(long, env)]
         pub host: String,
         #[arg(long, env)]
         pub port: u16,
     }

     #[derive(Conf)]
     #[conf(serde)]
     pub struct AppConfig {
         #[arg(long, env)]
         pub name: String,
         #[conf(flatten, serde(flatten))]
         pub database: DatabaseConfig,
     }
     # }
     ```
     Expected JSON structure (flattened):
     ```json
     {
       "name": "myapp",
       "host": "localhost",
       "port": 5432
     }
     ```

     * <a name="flatten-serde-flatten-prefix"></a> `prefix` (optional string argument)

       example: `#[conf(flatten, serde(flatten(prefix)))]`, `#[conf(flatten, serde(flatten(prefix = "db.")))]`

       Specifies that all flattened fields should have a common prefix in the serde document.

       - `serde(flatten(prefix))` - Uses the field name in snake_case followed by an underscore as the prefix. For example, if the field is named `database`, the prefix would be `database_`.
       - `serde(flatten(prefix = "custom_"))` - Uses a custom prefix string. This allows using any separator (like `.` for hierarchical keys).

       When using `prefix`, all keys from the child struct must include the prefix in the serde document. The prefix is stripped before matching against the child struct's fields.

       **Example with auto-generated prefix**:
       ```rust
       # use conf::Conf;
       # #[cfg(feature = "serde")]
       # {
       #[derive(Conf)]
       #[conf(serde)]
       pub struct DatabaseConfig {
           #[arg(long, env)]
           pub host: String,
           #[arg(long, env)]
           pub port: u16,
       }

       #[derive(Conf)]
       #[conf(serde)]
       pub struct AppConfig {
           #[arg(long, env)]
           pub name: String,
           #[conf(flatten, serde(flatten(prefix)))]
           pub database: DatabaseConfig,
       }
       # }
       ```
       Expected JSON structure (with `database_` prefix):
       ```json
       {
         "name": "myapp",
         "database_host": "localhost",
         "database_port": 5432
       }
       ```

       **Example with custom prefix**:
       ```rust
       # use conf::Conf;
       # #[cfg(feature = "serde")]
       # {
       #[derive(Conf)]
       #[conf(serde)]
       pub struct DatabaseConfig {
           #[arg(long, env)]
           pub host: String,
           #[arg(long, env)]
           pub port: u16,
       }

       #[derive(Conf)]
       #[conf(serde)]
       pub struct AppConfig {
           #[arg(long, env)]
           pub name: String,
           #[conf(flatten, serde(flatten(prefix = "db.")))]
           pub database: DatabaseConfig,
       }
       # }
       ```
       Expected JSON structure (with `db.` prefix):
       ```json
       {
         "name": "myapp",
         "db.host": "localhost",
         "db.port": 5432
       }
       ```

       **Nested prefixes**: When you have multiple levels of flattening with prefixes, the prefixes are concatenated. For example, if `A` flattens `B` with prefix `b_`, and `B` flattens `C` with prefix `c_`, then `C`'s fields appear with prefix `b_c_` in the top-level serde document.

       **Note**: The `prefix` option only affects serde deserialization. CLI arguments and environment variables continue to use their own prefixing scheme controlled by `conf(flatten, prefix)`, `long_prefix`, and `env_prefix`.

#### Notes

Using `flatten` with no additional attributes behaves the same as [`clap(flatten)`](https://docs.rs/clap/4.5.8/clap/_derive/index.html#flattening).

When using `flatten` with `Option<T>`, the parsing behavior is:

* If none of the fields of `T` (after flattening and prefixes) are present among the CLI arguments or env, and the substructure doesn't appear in the `serde` document, then the result is `None`.
* If any of the fields of `T` are present, or if the substructure appears in the serde document, then we must succeed in parsing a `T` as usual, and the result is `Some`.

### Subcommands

A `subcommands` field works similarly to a [`#[clap(subcommand)]`](https://docs.rs/clap/4.5.8/clap/_derive/index.html#subcommands) field, and represents one or more subcommands that can be used with this `Conf`.

**Requirements**: A `subcommands` field type must be `T` or `Option<T>` where `T: Subcommands`.

* `Option<T>` means that use of a subcommand is optional, otherwise, one of the subcommands must appear.
* Each enum variant corresponds to one subcommand. The enum variant determines the name of the subcommand, and the value is a `Conf` structure.
* If the subcommand appears on the command-line, then the subcommand is active, and the remaining arguments are handled by the subcommand parser.
  The result of this parse is stored as the enum value.
* Subcommands at the same level are mutually exclusive, but a subcommand can itself have subcommands.

Each subcommand has its own independent `--help` section, and subcommands are listed in the main help section.

Subcommands are thus well-suited when your application has several "modes" of operation with very different behavior and configuration.
For example, in one mode, you might run a webserver, and in another you might run database migrations, or check the integrity of a data structure.
When you use subcommands, the top-level `--help` won't show any subcommand-specific configuration options, and the help of each subcommand
only shows options relevant to that subcommand. This can help users navigate the help more easily.

See also the [`Subcommands`] trait and proc-macro documentation.

*  <a name="subcommands-serde"></a> `serde` (optional additional attributes)

   Configuration specific to the serde integration.

   * <a name="subcommands-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to [`#[serde(skip)]`](https://serde.rs/field-attrs.html#skip).

     These subcommands won't support reading anything from the serde value source, and not need have `#[serde(conf)]`
     when derived.

**Restrictions**:

*  At most one `subcommands` field can appear in a given struct.
*  `subcommands` fields are only valid at top-level, and cannot be used in a struct that is flattened.

## Struct-level attributes

Some struct attributes are "top-level only". This means they only have an effect when `Conf::parse()` and similar are called
on the struct that they are marking. If the struct that they mark is flattened into another struct, then these attributes have no effect on how `Conf::parse`
works on that struct. Attributes that are not "top-level only" will still have an effect when the struct that they mark is flattened.

*  <a name="struct-no-help-flag"></a> `no_help_flag` (no arguments) (top-level only)

   example: `#[conf(no_help_flag)]`

   Suppresses the automatically generated help option.

   *Note*: Similar to [`disable_help_flag = true`](https://docs.rs/clap/4.5.8/clap/struct.Command.html#method.disable_help_flag) in `clap`, but doesn't propagate to any other structs.

*  <a name="struct-version"></a> `version` (optional string argument) (top-level only)

   example: `#[conf(version)]`, `#[conf(version = "1.2.3")]`

   Enables the `-V` / `--version` flag for the program. When this flag is passed, the version is printed and the program exits immediately (before any argument validation).

   - `#[conf(version)]` - Uses `CARGO_PKG_VERSION` from the crate's `Cargo.toml` at compile time
   - `#[conf(version = "1.2.3")]` - Uses the specified version string

   This attribute is mutually exclusive with `version_fn`.

   **Example**:
   ```rust
   use conf::Conf;

   #[derive(Conf)]
   #[conf(version)]
   pub struct Config {
       #[conf(long, env)]
       pub server_port: u16,
   }
   ```

   With this configuration, `./my_prog --version` will print the version and exit, even if required arguments like `server_port` are missing.

   *Note*: Similar to [`clap::Command::version`](https://docs.rs/clap/4.5.8/clap/struct.Command.html#method.version).

*  <a name="struct-version-fn"></a> `version_fn` (expr argument) (top-level only)

   example: `#[conf(version_fn = my_version_fn)]`

   Enables the `-V` / `--version` flag using a function that returns the version string at runtime. The function must have signature `fn() -> &'static str`.

   This attribute is mutually exclusive with `version`.

   **Example**:
   ```rust
   use conf::Conf;

   fn my_version() -> &'static str {
       "1.0.0-custom"
   }

   #[derive(Conf)]
   #[conf(version_fn = my_version)]
   pub struct Config {
       #[conf(long, env)]
       pub server_port: u16,
   }
   ```

*  <a name="struct-about"></a> `about` (string argument) (top-level only)

   example: `#[conf(about = "Frobnicate as a service")]`

   The about string is displayed as the first line of the automatically-generated help page, before the usage is displayed.

   The about string can be set by passing `#[conf(about="...")]`.
   If it is not set, it defaults to the doc string on the struct.

   *Note*: This matches the behavior of `clap` very closely.

*  <a name="struct-name"></a> `name` (string argument) (top-level only)

   example: `#[conf(name = "frob_server")]`

   The name string is displayed as the name of the binary in the usage string in the help page.

   The name string can be set by passing `#[conf(name="...")]`.
   If it is not set, it defaults to the value of `CARGO_PKG_NAME` when the proc macro is being expanded, which is the same default as `clap-derive`.

   *Note*: This matches the behavior of `clap` very closely.

*  <a name="struct-display-name"></a> `display_name` (string argument)

   example: `#[conf(display_name = "MyConfig")]`

   Sets a custom name for this struct to be used in error messages. If not set, error messages use the struct's type name.

*  <a name="struct-env-prefix"></a> `env_prefix` (string argument)

   example: `#[conf(env_prefix = "FROBCO_")]`

   The given string is concatenated to the beginning of every env form and env alias of every program option associated to this struct.

*  <a name="struct-serde"></a> `serde` (optional additional attributes)

   example: `#[conf(serde)]`, `#[conf(serde(allow_unknown_fields))]`

   Enable serde as a value source for this struct, for additional layered config patterns.

   * <a name="struct-serde-allow-unknown-fields"></a> `allow_unknown_fields` (no arguments)

     Similar to [`#[serde(deny_unknown_fields)]`](https://serde.rs/container-attrs.html#deny_unknown_fields), except that the default is reversed here, to avoid configuration mistakes.

*  <a name="struct-one-of-fields"></a> `one_of_fields` (parenthesized identifier list)

   example: `#[conf(one_of_fields(a, b, c))]`

   Creates a validation constraint that must be satisfied after parsing this struct succeeds.

   Each identifier in the list must correspond to a field in this `struct`.

   Each field must have type `bool` or `Option<T>` or `Vec<T>`.

   The total number of these fields which are "present" (`true` or `Some` or `non-empty`) must be exactly one,
   otherwise an error will be generated describing the offending / missing fields, with context.

   Note that any of the field kinds is potentially supported (`flag`, `parameter`, `repeat`, `flatten`, `subcommands`).

*  <a name="struct-at-most-one-of-fields"></a> `at_most_one_of_fields` (parenthesized identifier list)

   example: `#[conf(at_most_one_of_fields(a, b, c, d)]`

   Each identifier in the list must correspond to a field in this `struct`.

   Same as `one_of_fields` except that it's not an error if zero of the fields are present.

*  <a name="struct-at-least-one-of-fields"></a> `at_least_one_of_fields` (parenthesized identifier list)

   example: `#[conf(at_least_one_of_fields(b, c, d)]`

   Each identifier in the list must correspond to a field in this `struct`.

   Same as `one_of_fields` except that it's not an error if more than one of the fields are present.

*  <a name="struct-validation-predicate"></a> `validation_predicate` (expr argument)

   example: `#[conf(validation_predicate = my_function)]`

   Creates a validation constraint that must be satisfied after parsing this struct succeeds, from a user-defined function.
   The function should have signature `fn(&T) -> Result<(), impl Display>`.

   The `validation_predicate = ...` attribute is allowed to repeat multiple times, to set multiple validation predicates.

*  <a name="struct-test"></a> `test` (no arguments)

   example: `#[conf(test)]`

   Automatically generates a test function that validates the struct's command-line interface configuration.
   This test catches problems that can't be caught at build-time by the proc-macro, which may require global information.

   The generated test function:
   - Is named `conf_debug_assert_{struct_name}`
   - Checks that all `default_value` pass the `value_parser` without error, or that `default` is valid, unless that field
     has the `conf(test(skip))` attribute.
   - Constructs a `Parser` from the struct's program options
   - Runs [`clap::Command::debug_assert()`](https://docs.rs/clap/4.5.8/clap/struct.Command.html#method.debug_assert) to check for configuration errors

   This is useful for catching configuration issues at test time, such as:
   - Conflicting short or long flags
   - Invalid default values
   - Positional argument ordering issues

   **Note**: It is better to put this on the top-level struct that you will actually parse (e.g., the one you call `Conf::parse()` on), rather than on intermediate structs that are flattened into others. This will be more efficient and there shouldn't be any problems that would have been caught
   by testing the intermediate structs.

   **Note**: This attribute cannot be used on structs with generic type parameters. If you need this, please open an issue to discuss.


[^1]: Actually, the *tokens* of the type are used, so e.g. it must be `bool` and not an alias for `bool`.

[^compat-note-1]: In `clap`, `repeat` parameters are inferred by setting the type to `Vec<T>`, and this is the only way to specify a repeat parameter. It also changes the meaning of `value_parser` in a subtle way.
However, this can become confusing and so `conf` deviates from `clap` here. Instead, in `conf` the only way to specify a repeat parameter is to use the `repeat` attribute.

[^compat-note-2]: Our `value_parser` feature is very similar to `clap-derive`, but it's a little easier to use in this crate at time of writing. For instance `value_parser = serde_json::from_str` just works,
while at `clap` version 4.5.8 it fails with type inference errors. The reason seems to be that in clap v4, the [`value_parser!` macro](https://docs.rs/clap/4.5.8/clap/macro.value_parser.html) was introduced, and it uses auto-ref specialization to try to detect
features of the value parser type at build time and handle special cases. However, this adds more layers of complexity and prevents the compiler from inferring things like lifetime parameters, afaict, so it makes the
UX of the `derive` API somewhat worse. Our criteria for the `value_parser` feature are also a bit different because we don't need the solution to work with the `clap_builder` API as well.

