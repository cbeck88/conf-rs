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
      * [skip](#flag-serde-skip)
  * [Parameter](#parameter)
    * [short](#parameter-short)
    * [long](#parameter-long)
    * [env](#parameter-env)
    * [aliases](#parameter-aliases)
    * [env_aliases](#parameter-env-aliases)
    * [default_value](#parameter-default-value)
    * [value_parser](#parameter-value-parser)
    * [allow_hyphen_values](#parameter-allow-hyphen-values)
    * [secret](#parameter-secret)
    * [serde](#parameter-serde)
      * [rename](#parameter-serde-rename)
      * [skip](#parameter-serde-skip)
      * [use_value_parser](#parameter-serde-use-value-parser)
  * [Repeat](#repeat)
    * [long](#repeat-long)
    * [env](#repeat-env)
    * [aliases](#repeat-aliases)
    * [env_aliases](#repeat-env-aliases)
    * [value_parser](#repeat-value-parser)
    * [env_delimiter](#repeat-env-delimiter)
    * [no_env_delimiter](#repeat-no-env-delimiter)
    * [allow_hyphen_values](#repeat-allow-hyphen-values)
    * [secret](#repeat-secret)
    * [serde](#repeat-serde)
      * [rename](#repeat-serde-rename)
      * [skip](#repeat-serde-skip)
      * [use_value_parser](#repeat-serde-use-value-parser)
  * [Flatten](#flatten)
    * [prefix](#flatten-prefix)
    * [long_prefix](#flatten-long-prefix)
    * [env_prefix](#flatten-env-prefix)
    * [help_prefix](#flatten-help-prefix)
    * [skip_short](#flatten-skip-short)
    * [serde](#flatten-serde)
      * [rename](#flatten-serde-rename)
      * [skip](#flatten-serde-skip)
  * [Subcommands](#subcommands)
    * [serde](#subcommands-serde)
      * [skip](#subcommands-serde-skip)
* [Struct-level attributes](#struct-level-attributes)
  * [no_help_flag](#struct-no-help-flag)
  * [about](#struct-about)
  * [name](#struct-name)
  * [env_prefix](#struct-env-prefix)
  * [serde](#struct-serde)
    * [allow_unknown_fields](#struct-serde-allow-unknown-fields)
  * [one_of_fields](#struct-one-of-fields)
  * [at_most_one_of_fields](#struct-at-most-one-of-fields)
  * [at_least_one_of_fields](#struct-at-least-one-of-fields)
  * [validation_predicate](#struct-validation-predicate)

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

Attributes which take an argument like this, with the `=` sign, can only be set once. It's an error if they occur a second time on the same item.

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
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.visible_aliases)

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

     Similar to `#[serde(rename)]`, changes the name used in serialization, which by default is the field name.

   * <a name="flag-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to `#[serde(skip)]`, this field won't be read from the serde value source.

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

*  <a name="parameter-env"></a> `env` (optional string argument)

   Specifies an environment variable associated to this parameter.
   If argument is omitted, defaults to the upper snake-case field name.

   example: `#[arg(env)]`, `#[arg(env = "PARAM")]`

   example command-line: `PARAM=foo ./my_prog` sets the parameter using the string value `foo`

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="parameter-aliases"></a> `aliases` (string array argument)

   Specifies alternate long switches that should be an alias for this parmeter.
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.visible_aliases)

   example: `#[arg(aliases=["old-param-name", "older-param-name"])]`

   example command-line: `./my_prog --old-param-name foo` or `./my_prog --old-param-name=foo` sets the parameter using the string value `foo`

*  <a name="parameter-env-aliases"></a> `env_aliases` (string array argument)

   example: `#[arg(env_aliases=["OLD_PARAM_NAME", "OLDER_PARAM_NAME"])]`

   Specifies alternate (fallback) environment variables which should be associated to this parameter. These are checked in the order listed, and if a value is found, the later ones are not checked.

*  <a name="parameter-default-value"></a> `default_value` (string argument)

   example: `#[arg(default_value = "some value")]`

   Specifies the default value assigned to this parameter if none of the switches or env are present.

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="parameter-value-parser"></a> `value_parser` (expr argument)

   example: `#[arg(value_parser = my_function)]`

   By default, `conf` invokes the trait function `std::str::FromStr::from_str` to convert the parsed string to the type of the field.
   This can be overrided by setting `value_parser`. Any function expression can be used as long as any generic parameters are either specified or inferred.

   *Note*: This is very similar to `clap-derive`, but there are technical differences [^compat-note-2].

*  <a name="parameter-allow-hyphen-values"></a> `allow_hyphen_values` (no arguments)

   example: `#[arg(allow_hyphen_values)]`

   By default, clap's parser considers a leading hyphen in a parameter value like `--my-param --my-value` to be an error, and that the user more likely forgot to give a value to `--my-param`
   and tried to specify a switch `--my-value` afterwards, than that they intended to give the value `--my-value` to `--my-param`. So the default behavior is to give an error in that case.

   If you actually intended to set `--my-param` to the value `--my-value`, you can instead write `--my-param=--my-value`, or set it via an environment variable, which doesn't care about this setting.
   If you set `allow_hyphen_values` then this check is not applied, and `--my-param --my-value` gets parsed the same as `--my-param=--my-value`.

   `allow_hyphen_values` is automatically set when the field value has a built-in type `i8`, `i16`, `i32`, `i64`, `f32`, `f64`, since it is more likely in these cases that you intend to pass a negative number.

   This corresponds to [`clap::Arg::allow_hypen_values`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.allow_hyphen_values)

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

     Similar to `#[serde(rename)]`, changes the name used in serialization, which by default is the field name.

   * <a name="parameter-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to `#[serde(skip)]`, this field won't be read from the serde value source.
     This can be useful if the type doesn't implement `serde::Deserialize`.

   * <a name="parameter-serde-use-value-parser"></a> `use_value_parser` (no arguments)

     example: `#[conf(serde(use_value_parser))]`

     If used, then instead of asking `serde` to deserialize the field type, `serde` will deserialize a `String`,
     and then the `value_parser` will convert the string to the field type. The default value parser is `FromStr`.

#### Notes

When a parameter is parsed from CLI arguments, the parser expects one of two syntaxes `--param=value` or `--param value` to be used. If `value` is not found as expected then parsing will fail.
If `--param` appears twice then parsing will fail. Short switches and long switches behave similarly in this regard.

As in `clap`, if a parameter's field type is `Option<T>`, it has special meaning.

* The parameter is not considered required. If it is omitted, parsing will succeed and the value will be `None`.
* If parsing does produce a string, then the value will be `Some`.
* If a `value_parser` is specified, it should produce `T` rather than `Option<T>`.
* The option will not be considered required when rendering the help text.

Currently none of the other special [type-based intent inferences that clap does](https://docs.rs/clap/4.5.8/clap/_derive/index.html#arg-types) are implemented in this crate.

### Repeat

A repeat field is similar to a parameter, except that it may appear multiple times on the command line, and the collection of string arguments are then parsed individually and aggregated.

**Requirements**: A repeat field must have type `Vec<T>`, where `T` implements `FromStr`, or `value_parser` must be supplied that produces a `T`.

*Note*: A repeat option produces one `T` for each time the option appears in the CLI arguments, and unlike a parameter the option can appear multiple times. If it does not appear, and an `env` variable is specified, then that variable
is read and split on a delimiter character which defaults to `','`, to produce a series of `T` values.

*  <a name="repeat-long"></a> `long` (optional string argument)

   Specifies a long (two-dash) switch associated to this option.
   If argument is omitted, defaults to the kebab-cased field name.

   example: `#[arg(long)]`, `#[arg(long = "peer")]`

   example command-line: `./my_prog --peer peer1 --peer peer2`

   *Note*: This behavior of this attribute is the same as in `clap-derive`.

*  <a name="repeat-env"></a> `env` (optional string argument)

   Specifies an environment variable associated to this option.
   If omitted, defaults to the upper snake-cased field name.

   example: `#[arg(env)]`, `#[arg(env = "PEERS")]`

   example command-line: `PEERS=peer1,peer2 ./my_prog`

   *Note*: The behavior of this attribute is the same as in `clap-derive`.

*  <a name="repeat-aliases"></a> `aliases` (string array argument)

   Specifies alternate long switches that should be an alias for this option.
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.visible_aliases)

   example: `#[arg(aliases=["OLD_PARAM_NAME", "OLDER_PARAM_NAME"])]`

*  <a name="repeat-env-aliases"></a> `env_aliases` (string array argument)

   Specifies alternate (fallback) environment variables which should be associated to this option. These are checked in the order listed, and if a value is found, the later ones are not checked.

   example: `#[arg(env_aliases=["OLD_PARAM_NAME", "OLDER_PARAM_NAME"])]`

*  <a name="repeat-value-parser"></a> `value_parser` (expr argument)

   By default, `conf` invokes the trait function `std::str::FromStr::from_str` to convert the parsed string to the type `T`.
   This can be overrided by setting `value_parser`. Any function expression can be used as long as it produces a `T` and any generic parameters are either specified or inferred.

   *Note*: This behavior is the same as in `clap-derive`.

*  <a name="repeat-env-delimiter"></a> `env_delimiter` (char argument)

   Controls what character is used as a delimiter when reading the list from an environment variable.

   example: `[conf(env_delimiter = '|')]`

   example command-line: `PEERS=peer1|peer2 ./my_prog`

   *Note*: This doesn't have a direct analog in `clap-derive`, but as far as `env` is concerned it's like `value_delimieter`.

*  <a name="repeat-no-env-delimiter"></a> `no_env_delimiter` (no argument)

   If set, then the env is parsed as if it is a single `T` and not a list. This can be used for strict compatibility with common `clap` configurations.

   example: `[conf(no_env_delimiter)]`

*  <a name="repeat-allow-hyphen-values"></a> `allow_hyphen_values` (no arguments)

   example: `[arg(allow_hyphen_values)]`

   By default, clap's parser considers a leading hyphen in a parameter value like `--my-param --my-value` to be an error, and that the user more likely forgot to give a value to `--my-param`
   and tried to specify a switch `--my-value` afterwards, than that they intended to give the value `--my-value` to `--my-param`. So the default behavior is to give an error in that case.

   If you actually intended to set `--my-param` to the value `--my-value`, you can instead write `--my-param=--my-value`, or set it via an environment variable, which doesn't care about this setting.
   If you set `allow_hyphen_values` then this check is not applied, and `--my-param --my-value` gets parsed the same as `--my-param=--my-value`.

   `allow_hyphen_values` is automatically set when the field value has a built-in type `i8`, `i16`, `i32`, `i64`, `f32`, `f64`, since it is more likely in these cases that you intend to pass a negative number.

   This corresponds to [`clap::Arg::allow_hypen_values`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.allow_hyphen_values)

*  <a name="repeat-secret"></a> `secret` (optional bool argument)

   example: `#[conf(secret)]`, `#[conf(secret = false)]`

   Indicates that this value is secret and `conf` should avoid logging the value if there is an error.

   If the `bool` argument is not specified when this attribute appears, it is considered `true`.
   Values not marked secret are considered not to be secrets.

*  <a name="repeat-serde"></a> `serde` (optional additional attributes)

   example: `#[conf(serde(use_value_parser, rename = "foos"))]`

   Configuration specific to the serde integration.

   * <a name="repeat-serde-rename"></a> `rename` (string argument)

     example: `#[conf(serde(rename = "foos"))]`

     Similar to `#[serde(rename)]`, changes the name used in serialization, which by default is the field name.

   * <a name="repeat-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to `#[serde(skip)]`, this field won't be read from the serde value source.
     This can be useful if the value doesn't implement `serde::Deserialize`.

   * <a name="repeat-serde-use-value-parser"></a> `use_value_parser` (no arguments)

     example: `#[conf(serde(use_value_parser))]`

     If used, then instead of asking `serde` to deserialize `Vec<T>`, `serde` will deserialize a `Vec<String>`,
     and then the `value_parser` will convert each string to `T`. The default value parser is `FromStr`.

#### Notes

`clap-derive`'s multi-option's don't work that well in a 12-factor app, because there's a mismatch between, getting multiple strings from the CLI arguments, and getting one string from env.

`clap-derive`'s behavior for a typical case like

```rust ignore
   #[clap(long, env)]
   my_list: Vec<String>,
```

when there is no CLI arg and only env is set, is that the entire env value becomes the one element of `my_list`, and there is no way to configure a list with multiple items by setting only `env`.
So, most likely an app that was using `clap` this way was only using the CLI arguments to configure this value. For `conf`, we consider that this is not a good default behavior.

`clap` does have an additional option for this case called `value_delimiter`, which will cause it to split both CLI arguments and `env` values on a given character.
In `conf` however, at this point the field can just be `parameter` instead of a `repeat`, and a `value_parser` can be used which does the splitting.
So we don't provide the `value_delimiter` feature here.

The main reasons that we provide `repeat` are:

* Ease of migrating an existing `clap-derive` parser that may use the multi-option stuff
* It can be easier to read CLI args, for example in a shell script, when a list is split into many args rather than having one very long list arg.

If your goal is compatiblity with an existing `clap-derive` parser that parses a `Vec` has no `value_delimiter`, you should use `repeat` with `no_env_delimiter`.

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

   example: `#[conf(serde(rename = "foo"))]`

   Configuration specific to the serde integration.

   * <a name="flatten-serde-rename"></a> `rename` (string argument)

     example: `#[conf(serde(rename = "foo"))]`

     Similar to `#[serde(rename)]`, changes the name used in serialization, which by default is the field name.

   * <a name="flatten-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to `#[serde(skip)]`, this substructure won't be read from the serde value source.

#### Notes

Using `flatten` with no additional attributes behaves the same as `clap(flatten)`.

When using `flatten` with `Option<T>`, the parsing behavior is:

* If none of the fields of `T` (after flattening and prefixes) are present among the CLI arguments or env, and the substructure doesn't appear in the `serde` document, then the result is `None`.
* If any of the fields of `T` are present, or if the substructure appears in the serde document, then we must succeed in parsing a `T` as usual, and the result is `Some`.

### Subcommands

A `subcommands` field works similarly to a `#[clap(subcommand)]` field, and represents one or more subcommands that can be used with this `Conf`.

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

   *Note*: Similar to `disable_help_flag = true` in `clap`, but doesn't propagate to any other structs.

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

*  <a name="struct-env-prefix"></a> `env_prefix` (string argument)

   example: `#[conf(env_prefix = "FROBCO_")]`

   The given string is concatenated to the beginning of every env form and env alias of every program option associated to this struct.

*  <a name="struct-serde"></a> `serde` (optional additional attributes)

   example: `#[conf(serde)]`, `#[conf(serde(allow_unknown_fields))]`

   Enable serde as a value source for this struct, for additional layered config patterns.

   * <a name="struct-serde-allow-unknown-fields"></a> `allow_unknown_fields` (no arguments)

     Similar to `#[serde(deny_unknown_fields)]`, except that the default is reversed here, to avoid configuration mistakes.

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


[^1]: Actually, the *tokens* of the type are used, so e.g. it must be `bool` and not an alias for `bool`.

[^compat-note-1]: In `clap`, `repeat` parameters are inferred by setting the type to `Vec<T>`, and this is the only way to specify a repeat parameter. It also changes the meaning of `value_parser` in a subtle way.
However, this can become confusing and so `conf` deviates from `clap` here. Instead, in `conf` the only way to specify a repeat parameter is to use the `repeat` attribute.

[^compat-note-2]: Our `value_parser` feature is very similar to `clap-derive`, but it seems to work a little better in this crate at time of writing. For instance `value_parser = serde_json::from_str` just works,
while at `clap` version 4.5.8 it doesn't work. The reason seems to be that in clap v4, the `value_parser!` macro was introduced, and it uses auto-ref specialization to try to detect
features of the value parser type at build time and handle special cases. However, this adds more layers of complexity and prevents the compiler from inferring things like lifetime parameters, afaict, so it makes the
UX of the `derive` API somewhat worse. Our criteria for how to implement `value_parser` are also a bit different because we don't have a need for the solution to work with the `clap_builder` API as well.

