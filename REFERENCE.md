# Proc-macro reference

To use `conf`, use the `#[derive(Conf)]` proc macro on your configuration struct.
Then call a `Conf` trait function to parse your configuration struct.

This section documents the different `#[conf(...)]` attributes that the derive macro reads and what they do.
When the attributes are similar to those in `clap-derive`, we will call out any differences (as they exist in `clap 4.5.8`.)

The `#[conf(...)]` attributes conform to [Rust’s structured attribute convention](https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax).

## Where can conf attributes be used?

These attributes can appear in two places -- on a `struct` and on a `field`.

```
use conf::Conf;

#[derive(Conf)]
#[conf(env_prefix="RUST_")] // This is a struct-level attribute
pub struct MyConfig {
    #[conf(long, env)] // These are field-level attributes
    pub my_field: String,
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

Such attributes may allowed to repeat multiple times:

`#[conf(attr(...), attr(...))]`

In each case like this we'll document what syntax is valid in the parentheses.

## Field-level attributes

For compatibility with `clap-derive`, when a `conf` attribute is used on a field, the labels `#[arg(...)]` and `#[conf(...)]` can be used interchangeably.

When `derive(Conf)` encounters a field, the first thing it must determine what kind of field this is:

* Flag: A flag corresponds to a boolean program option. It is either set or it isn't. For example, `./my_prog --flag1 --flag2`.
* Parameter: A parameter corresponds to a program option that expects a string value to be found during parsing. For example `./my_prog --param1 value1 --param2 value2`.
* Repeat: A repeat option represents a list of values. It has special parsing -- it is allowed to be specified multiple times on the command-line, and the results are parsed separately and aggregated into a `Vec`. This is similar to what `clap` calls a multi-option, and what `clap-derive` does by default if the field type is a `Vec`. For example, `./my_prog --can-repeat value1 --can-repeat value2`.
* Flatten: A flatten field doesn't correspond to an option, but to a collection of options that come from another `Conf` structure, and may be adjusted before being merged in.

If the *first attribute* is `flag`, `parameter`, `repeat` or `flatten`, then `conf` will handle the field that way.

If none of these is found, then the *type* of the field is used to classify it.

* If the field is `bool`, then it is a flag
* Otherwise it is a parameter.

Each kind of field then supports a different set of attributes.

*Note*: In `clap`, `repeat` parameters are inferred by setting the type to `Vec<T>`, and this is the only way to specify a repeat parameter. It also changes the meaning of `value_parser` in a subtle way.
However, this can become confusing and so `conf` deviates from `clap` here. Instead, in `conf` the only way to specify a repeat parameter is to use the `repeat` attribute.

### Flag

A flag corresponds to a switch that doesn't take any parameters. It's presence on the command line means the value is `true`, otherwise it is `false`.

*Requirements*: A flag field must have type `bool`.

* `short` (optional char argument)

   Specifies a short (one-dash) switch associated to this flag.
   If argument is omitted, defaults to the first letter of the field name.

   example usage: `#[arg(short)]`, `#[arg(short = 'b')]`

   example behavior: `./my_prog -b` sets the flag to true

   *Note*: This behavior is the same as in `clap-derive`.

* `long` (optional string argument)

   Specifies a long (two-dash) switch associated to this flag.
   If argument is omitted, defaults to the kebab-cased field name.

   example: `#[arg(long)]`, `#[arg(long = "flag")]`

   example command-line: `./my_prog --flag` sets the flag to true

   *Note*: This behavior is the same as in `clap-derive`.

* `env` (optional string argument)

   Specifies an environment variable associated to this flag.
   If argument is omitted, defaults to the upper snake-case field name.

   When the environment variable is set, the flag is considered to be true, unless
   the value is `0`, `false`, `f`, `off`, `o`, or empty string.

   example: `#[arg(env)]`, `#[arg(env = "FLAG")]`

   example command-line: `FLAG=1 ./my_prog` sets the flag to true

   *Note*: This behavior is the same as in `clap-derive`.

* `aliases` (string array argument)

   Specifies alternate long switches that should be an alias for this flag.
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.visible_aliases)

   example: `#[arg(aliases=["old-flag-name", "older-flag-name"])]`

   example command-line: `./my_prog --old-flag-name` sets the flag to true

* `env_aliases` (string array argument)

   Specifies alternate (fallback) environment variables which should be associated to this flag. These are checked in the order listed, and if a value is found, the later ones are not checked.

   example: `#[arg(env_aliases=["OLD_FLAG_NAME", "OLDER_FLAG_NAME"])]`

   example command-line: `OLD_FLAG_NAME=1 ./my_prog` sets the flag to true

### Parameter

A parameter represents a single value that can be parsed from a string.

*Requirements*: A parameter field can have any type as long as it implements `FromStr` or `value_parser` is used.

* `short` (optional char argument)

   Specifies a short (one-dash) switch associated to this parameter.
   If argument is omitted, defaults to the first letter of the field name.

   example: `#[arg(short)]`, `#[arg(short = 'p')]`

   example command-line: `./my_prog -p foo` or `./my_prog -p=foo` sets the parameter using the string value `foo`

   *Note*: This behavior is the same as in `clap-derive`.

* `long` (optional string argument)

   Specifies a long (two-dash) switch associated to this parameter.
   If argument is omitted, defaults to the kebab-cased field name.

   example: `#[arg(long)]`, `#[arg(long = "param")]`

   example command-line: `./my_prog --param foo` or `./my_prog --param=foo` sets the parameter using the string value `foo`

   *Note*: This behavior is the same as in `clap-derive`.

* `env` (optional string argument)

   Specifies an environment variable associated to this parameter.
   If argument is omitted, defaults to the upper snake-case field name.

   example: `#[arg(env)]`, `#[arg(env = "PARAM")]`

   example command-line: `PARAM=foo ./my_prog` sets the parameter using the string value `foo`

   *Note*: This behavior is the same as in `clap-derive`.

* `aliases` (string array argument)

   Specifies alternate long switches that should be an alias for this parmeter.
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.visible_aliases)

   example: `#[arg(aliases=["old-param-name", "older-param-name"])]`

   example command-line: `./my_prog --old-param-name foo` or `./my_prog --old-param-name=foo` sets the parameter using the string value `foo`

* `env_aliases` (string array argument)

   Specifies alternate (fallback) environment variables which should be associated to this parameter. These are checked in the order listed, and if a value is found, the later ones are not checked.

   example: `#[arg(env_aliases=["OLD_PARAM_NAME", "OLDER_PARAM_NAME"])]`

* `default_value` (string argument)

   Specifies the default value assigned to this parameter if none of the switches or env are present.

   example: `#[arg(default_value = "some value")]`

   *Note*: This behavior is the same as in `clap-derive`.

* `value_parser` (expr argument)

   By default, `conf` invokes the trait function `std::str::FromStr::from_str` to convert the parsed string to the type of the field.
   This can be overrided by setting `value_parser`. Any function expression can be used as long as any generic parameters are either specified or inferred.

   example: `#[arg(value_parser = my_function)]`

   *Note*: This is very similar to `clap-derive`, but it seems to work a little better in this crate at time of writing. For instance `value_parser = serde_json::from_str` just works,
   while at `clap` version 4.5.8 it doesn't work. I'm not totally sure why that is, but it seems to be something about lifetime inferences.

* `allow_hyphen_values` (no arguments)

   By default, clap's parser considers a leading hyphen in a parameter value like `--my-param --my-value` to be an error, and that the user more likely forgot to give a value to `--my-param`
   and tried to specify a switch `--my-value` afterwards, than that they intended to give the value `--my-value` to `--my-param`. So the default behavior is to give an error in that case.

   If you actually intended to set `--my-param` to the value `--my-value`, you can instead write `--my-param=--my-value`, or set it via an environment variable, which doesn't care about this setting.
   If you set `allow_hyphen_values` then this check is not applied, and `--my-param --my-value` gets parsed the same as `--my-param=--my-value`.

   `allow_hyphen_values` is automatically set when the field value has a built-in type `i8`, `i16`, `i32`, `i64`, `f32`, `f64`, since it is more likely in these cases that you intend to pass a negative number.

   This corresponds to [`clap::Arg::allow_hypen_values`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.allow_hyphen_values)

   example: `#[conf(allow_hyphen_values)]`

* `secret` (optional bool argument)

   Indicates that this value is secret and `conf` should avoid logging the value if there is an error.
   
   If the `bool` argument is not specified when this attribute appears, it is considered `true`.
   Values not marked secret are considered not to be secrets.

   example: `#[conf(secret)]`, `#[conf(secret = false)]`

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

*Requirements*: A repeat field must have type `Vec<T>`, where `T` implements `FromStr`, or `value_parser` must be supplied that produces a `T`.

*Note*: A repeat option produces one `T` for each time the option appears in the CLI arguments, and unlike a parameter the option can appear multiple times. If it does not appear, and an `env` variable is specified, then that variable
is read and split on a delimiter character which defaults to `','`, to produce a series of `T` values.

* `long` (optional string argument)

   Specifies a long (two-dash) switch associated to this option.
   If argument is omitted, defaults to the kebab-cased field name.

   example: `#[arg(long)]`, `#[arg(long = "peer")]`

   example command-line: `./my_prog --peer peer1 --peer peer2`

   *Note*: This behavior of this attribute is the same as in `clap-derive`.

* `env` (optional string argument)

   Specifies an environment variable associated to this option.
   If omitted, defaults to the upper snake-cased field name.

   example: `#[arg(env)]`, `#[arg(env = "PEERS")]`

   example command-line: `PEERS=peer1,peer2 ./my_prog`

   *Note*: The behavior of this attribute is the same as in `clap-derive`.

* `aliases` (string array argument)

   Specifies alternate long switches that should be an alias for this option.
   This corresponds to [`clap::Arg::visible_aliases`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.visible_aliases)

   example: `#[arg(aliases=["OLD_PARAM_NAME", "OLDER_PARAM_NAME"])]`

* `env_alias` (string array argument)

   Specifies alternate (fallback) environment variables which should be associated to this option. These are checked in the order listed, and if a value is found, the later ones are not checked.

   example: `#[arg(env_aliases=["OLD_PARAM_NAME", "OLDER_PARAM_NAME"])]`

* `value_parser` (expr argument)

   By default, `conf` invokes the trait function `std::str::FromStr::from_str` to convert the parsed string to the type `T`.
   This can be overrided by setting `value_parser`. Any function expression can be used as long as it produces a `T` and any generic parameters are either specified or inferred.

   *Note*: This behavior is the same as in `clap-derive`.

* `env_delimiter` (char argument)

   Controls what character is used as a delimiter when reading the list from an environment variable.

   example: `[arg(env_delimiter = '|')]`

   example command-line: `PEERS=peer1|peer2 ./my_prog`

   *Note*: This doesn't have a direct analog in `clap-derive`, but as far as `env` is concerned it's like `value_delimieter`.

* `no_env_delimiter` (no argument)

   If set, then the env is parsed as if it is a single `T` and not a list. This can be used for strict compatibility with common `clap` configurations.

   example: `[arg(no_env_delimiter)]`

* `allow_hyphen_values` (no arguments)

   By default, clap's parser considers a leading hyphen in a parameter value like `--my-param --my-value` to be an error, and that the user more likely forgot to give a value to `--my-param`
   and tried to specify a switch `--my-value` afterwards, than that they intended to give the value `--my-value` to `--my-param`. So the default behavior is to give an error in that case.

   If you actually intended to set `--my-param` to the value `--my-value`, you can instead write `--my-param=--my-value`, or set it via an environment variable, which doesn't care about this setting.
   If you set `allow_hyphen_values` then this check is not applied, and `--my-param --my-value` gets parsed the same as `--my-param=--my-value`.

   `allow_hyphen_values` is automatically set when the field value has a built-in type `i8`, `i16`, `i32`, `i64`, `f32`, `f64`, since it is more likely in these cases that you intend to pass a negative number.

   This corresponds to [`clap::Arg::allow_hypen_values`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.allow_hyphen_values)

   example: `[arg(allow_hyphen_values)]`

* `secret` (optional bool argument)

   Indicates that this value is secret and `conf` should avoid logging the value if there is an error.
   
   If the `bool` argument is not specified when this attribute appears, it is considered `true`.
   Values not marked secret are considered not to be secrets.

   example: `#[conf(secret)]`, `#[conf(secret = false)]`

#### Notes

`clap-derive`'s multi-option's don't work that well in a 12-factor app, because there's a mismatch between, getting multiple strings from the CLI arguments, and getting one string from env.

`clap-derive`'s behavior for a typical case like

```ignore
   #[clap(long, env)]
   my_list: Vec<String>,
```

when there is no CLI arg and only env is set, is that the entire env value becomes the one element of `my_list`, and there is no way to configure a list with multiple items by setting only `env`.
So, most likely an app that was using `clap` this way was only using the CLI arguments to configure this value.

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

*Note*: A flatten field's type `T` must implement `Conf`, or the field type must be `Option<T>` where `T` implements `Conf`.

* `env_prefix` (optional string argument)

   Specifies a string to be prepended to every environment variable of every program option in the target struct.
   If the argument is omitted, it defaults to the upper snake-case of the field name, with an `_` character appended.

   example: `#[conf(flatten, env_prefix = "AUTH_")]`

* `long_prefix` (optional string argument)

   Specifies a string to be prepended to every long switch of every program option in the target struct.
   If the argument is omitted, it defaults to the kebab-case of the field name, with a `-` character appended.

   example: `#[conf(flatten, long_prefix = "auth-")]`

* `prefix` (optional string argument)

   Specifies a string to be used in place of the field name in the default constructions of `env_prefix` and `long_prefix`.
   If the argument is omitted, it's the same as specifying `env_prefix` and `long_prefix` both with no argument.

   This option cannot be used if `env_prefix` or `long_prefix` is present.
   If none of these options are used, then no prefixing occurs.

   example: `#[conf(flatten, prefix = "auth")]`

* `help_prefix` (optional string argument)

   Specifies that the help strings of every program option of the target struct should be prefixed with a particular string,
   to provide context. If the argument is omitted, it defaults to the doc string on this field.
   If the `help_prefix` attribute is not present then the help strings are unmodified.

   When prefixing is performed, some very simple logic is used to determine how to join the prefix.
   If either the prefix or the help string has multiple lines, then a newline character is used to join them.
   Otherwise a space character is used to join them. (This may change in future revisions.)

   example: `#[conf(flatten, help_prefix = "(friend service)")]`

* `skip_short` (char array argument)

   A list of short forms of options which should be skipped when options are flattened at this site.

   There is no way to prefix a short form -- it can only be one character. `skip_short` is a method to resolve conflicts when flattening.

   This should only be used as a last resort if you cannot simply remove one of the conflicting short forms at its source, because it would break something else.

   To try to help maintainability in a large project, it is an error if `skip_short` is used but no short flag matching this is actually found
   at this flattening site. In other words, when this attribute appears, you can be sure that a short flag is actually being removed, if no
   runtime errors are being reported.

   example: `#[conf(flatten, skip_short = ['a', 'b', 'f'])]`

#### Notes

Using `flatten` with no additional attributes behaves the same as `clap(flatten)`.

When using `flatten` with `Option<T>`, the parsing behavior is:

* If none of the fields of `T` (after flattening and prefixes) are present among the arguments and env, then the result is `None`.
* If any of the fields of `T` are present, then we must succeed in parsing a `T` as usual, and the result is `Some`.

## Struct-level attributes

Some struct attributes are "top-level only". This means they only have an effect when `Conf::parse()` and similar are called
on the struct that they are marking. If the struct that they mark is flattened into another struct, then these attributes have no effect on how `Conf::parse`
works on that struct. Attributes that are not "top-level only" will still have an effect when the struct that they mark is flattened.

* `no_help_flag` (no arguments) (top-level only)

   The `no_help_flag` attribute suppresses the automatically generated help option.

   example: `#[conf(no_help_flag)]`

   *Note*: Similar to `disable_help_flag = true` in `clap`, but doesn't propagate to any other structs.

* `about` (string argument) (top-level only)

   The about string is displayed as the first line of the automatically-generated help page, before the usage is displayed.

   The about string can be set by passing `#[conf(about="...")]`.
   If it is not set, it defaults to the doc string on the struct.

   example: `#[conf(about = "Frobnicate as a service")]`

   *Note*: This matches the behavior of `clap` very closely.

* `name` (string argument) (top-level only)

   The name string is displayed as the name of the binary in the usage string in the help page.

   The name string can be set by passing `#[conf(name="...")]`.
   If it is not set, it defaults to the value of `CARGO_PKG_NAME` when the proc macro is being expanded, which is the same default as `clap-derive`.

   example: `#[conf(name = "frob_server")]`

   *Note*: This matches the behavior of `clap` very closely.

* `env_prefix` (string argument)

   The given string is concatenated to the beginning of every env form and env alias of every program option associated to this struct.

   example: `#[conf(env_prefix = "FROBCO_")]`

* `one_of_fields` (parenthesized identifier list)

   example: `#[conf(one_of_fields(a, b, c))]`

   Creates a validation constraint that must be satisfied after parsing this struct succeeds.

   Each identifier in the list must correspond to a field in this `struct`.

   Each field must have type `bool` or `Option<T>` or `Vec<T>`.

   The total number of these fields which are "present" (`true` or `Some` or `non-empty`) must be exactly one,
   otherwise an error will be generated describing the offending / missing fields, with context.

   Note that any of the field kinds is potentially supported (`flag`, `parameter`, `repeat`, `flatten`).

* `at_most_one_of_fields` (parenthesized identifier list)

   example: `#[conf(at_most_one_of_fields(a, b, c, d)]`

   Each identifier in the list must correspond to a field in this `struct`.

   Same as `one_of_fields` except that it's not an error if zero of the fields are present.

* `at_least_one_of_fields` (parenthesized identifier list)

   example: `#[conf(at_least_one_of_fields(b, c, d)]`

   Each identifier in the list must correspond to a field in this `struct`.

   Same as `one_of_fields` except that it's not an error if more than one of the fields are present.
