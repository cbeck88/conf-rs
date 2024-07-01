# Proc-macro reference

The `#[derive(Conf)]` proc macro is main the user-facing interface to this crate's functionality.
To use the crate, you derive `Conf` on things, and then call a `Conf` trait function.

This section documents the different `#[conf(...)]` attributes that the derive macro reads and what they do.
When the attributes are similar to those in `clap-derive`, we will call out any differences (as they exist in `clap 4.5.8`.)

## Where can conf attributes be used?

The `#[conf(...)]` attributes can appear in two places -- on a `struct` and on a `field`.

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
are described as "optional", and we will explain what the behavior is if they are omitted. If they are not
marked optional, then they are required.

## Struct-level attributes

Some attributes are "top-level only". This means they only have an effect when `Conf::parse()` and similar are called
on the struct that they are marking. If the struct that they mark is flattened into another struct, then these attributes have no effect on how `Conf::parse`
works on that struct. Attributes that are not "top-level only" will still have an effect when the struct that they mark is flattened.

* `no_help_flag` (no arguments) (top-level only)

   The `no_help_flag` attribute suppresses the automatically generated help option.

   *Note*: Similar to `disable_help_flag = true` in `clap`, but doesn't propagate to any other structs.

* `about` (string argument) (top-level only)

   The about string is displayed as the first line of the automatically-generated help page, before the usage is displayed.

   The about string can be set by passing `#[conf(about="...")]`.
   If it is not set, it defaults to the doc string on the struct.

   *Note*: This matches the behavior of `clap` very closely, but we don't distinguish between `about` and `long_about`. The flags `-h` and `--help` are treated the same.

* `env_prefix` (string argument)

   The given string is concatenated to the beginning of every env form of every program option associated to this struct.

## Field-level attributes

When `derive(Conf)` encounters a field, the first thing it must determine what kind of field this is:

* Flag: A flag corresponds to a boolean program option. It is either set or it isn't. 
* Parameter: A parameter corresponds to a program option that expects a string value to be found during parsing.
* Repeat: A repeat option represents a list of values. It has special parsing -- it is allowed to be specified multiple times on the command-line, and the results are parsed separately and aggregated into a `Vec`. This is similar to what `clap` calls a multi-option, and what `clap-derive` does by default if the field type is a `Vec`.
* Flatten: A flatten field doesn't correspond to an option, but to a collection of options that come from another `Conf` structure.

To classify the field, first it looks at the attribute list on the field. If the *first attribute* is `flag`, `parameter`, `repeat` or `flatten`, then it will handle the field that way.

If none of these is found, then the type of the field is used to classify it.

* If the field is `bool`, then it is a flag
* Otherwise it is a parameter.

Each kind of field then supports a different set of attributes.

*Note*: In `clap`, `repeat` parameters are inferred by setting the type to `Vec<T>`, and this is the only way to specify a repeat parameter. It also changes the meaning of `value_parser` in a subtle way.
However, this can become confusing and so `conf` deviates from `clap` here. Instead, in `conf` the only way to specify a repeat parameter is to use the `repeat` attribute.

### Flag

*Requirements*: A flag field must have type `bool`.

* `short` (optional char argument)

   Specifies the short (one-dash) switch associated to this flag.
   If omitted, defaults to the first letter of the field name.

   *Note*: This behavior is the same as in `clap-derive`.

* `long` (optional string argument)

   Specifies the long (two-dash) switch associated to this flag.
   If omitted, defaults to the kebab-cased field name.

   *Note*: This behavior is the same as in `clap-derive`.

* `env` (optional string argument)

   Specifies the environment variable associated to this flag.
   If omitted, defaults to the upper snake-case field name.

   When the environment variable is set, the flag is considered to be true, unless
   the value is `0`, `false`, `f`, `off`, or `o`.

   *Note*: This behavior is the same as in `clap-derive`.

### Parameter

*Requirements*: A parameter field can have any type as long as it implements `FromStr` or `value_parser` is used.

* `short` (optional char argument)

   Specifies the short (one-dash) switch associated to this parameter.
   If omitted, defaults to the first letter of the field name.

   *Note*: This behavior is the same as in `clap-derive`.

* `long` (optional string argument)

   Specifies the long (two-dash) switch associated to this parameter.
   If omitted, defaults to the kebab-cased field name.

   *Note*: This behavior is the same as in `clap-derive`.

* `env` (optional string argument)

   Specifies the environment variable associated to this parameter.
   If omitted, defaults to the upper snake-case field name.

   *Note*: This behavior is the same as in `clap-derive`.

* `default_value` (string argument)

   Specifies the default value assigned to this parameter if none of the switches or env are present.

   *Note*: This behavior is the same as in `clap-derive`.

* `value_parser` (expr argument)

   By default, `conf` invokes the trait function `std::str::FromStr::from_str` to convert the parsed string to the type of the field.
   This can be overrided by setting `value_parser`. Any function expression can be used as long as any generic parameters are either specified or inferred.
   Even using a lambda function here will work.

   *Note*: This is very similar to `clap-derive`, but it works a little better in this crate at time of writing. For instance `value_parser = serde_json::from_str` just works,
   while at `clap` version 4.5.8 it doesn't work. It seems that this is because in `clap`, the value parser has to be converted to a `clap::ValueParser` wrapper object, but that
   cannot accommodate the generic lifetime parameters in `serde_json::from_str`. In the `Conf` derive macro, there is as little code as possible between
   the `value_parser` you specify and the actual initialization of the field, so type and lifetime inference are more likely to just work.

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

*Requirements*: A repeat field must have type `Vec<T>`, where `T` implements `FromStr`, or `value_parser` must be supplied that produces a `T`.

*Note*: A repeat option produces one `T` for each time the option appears in the CLI arguments, and unlike a parameter the option can appear multiple times. If it does not appear, and an `env` variable is specified, then that variable
is read and split on a delimiter character which defaults to `','`, to produce a series of `T` values.

* `long` (optional string argument)

   Specifies the long (two-dash) switch associated to this option.
   If omitted, defaults to the kebab-cased field name.

   *Note*: This behavior is the same as in `clap-derive`.

* `env` (optional string argument)

   Specifies the environment variable associated to this option.
   If omitted, defaults to the upper snake-cased field name.

   *Note*: This behavior is the same as in `clap-derive`.

* `value_parser` (expr argument)

   By default, `conf` invokes the trait function `std::str::FromStr::from_str` to convert the parsed string to the type `T`.
   This can be overrided by setting `value_parser`. Any function expression can be used as long as it produces a `T` and any generic parameters are either specified or inferred.

   *Note*: This behavior is the same as in `clap-derive`.

* `env_delimiter` (char argument)

   Controls what character is used as a delimiter when reading the list from an environment variable.

   *Note*: This doesn't have a direct analog in `clap-derive`.

* `no_env_delimiter` (no argument)

   If set, then the env is parsed as if it is a single `T` and not a list. This can be used for strict compatibility with common `clap` configurations.

#### Notes

`clap` multi-option's don't work that well in a 12-factor app, because there's a mismatch between, getting multiple strings from the CLI arguments, and getting one string from env.

`clap-derive`'s behavior for a typical case like

```ignore
   #[clap(long, env)]
   my_list: Vec<String>,
```

when there is no CLI arg and only env is set, is that the entire env value becomes the one element of `my_list`, and there is no way to configure a list with multiple items by setting only `env`.
So, most likely an app that was using `clap` this way was only using the CLI arguments to configure this value.

`clap` does have an additional option for this case called `value_delimiter`, which will cause it to split both CLI arguments and `env` values on a given character.
In `conf` however, at this point the field can just be `parameter` instead of a `repeat`, and a `value_parser` can be used which does the splitting.
So we don't provide the `value_delimiter` attribute here.

The main reasons that we provide `repeat` are:

* Ease of migrating an existing `clap` parser
* It can be easier to read CLI args where a list is split into many args rather than having one very long arg.

If your goal is compatiblity with an existing `clap` parser that parses a `Vec` has no `value_delimiter`, you should use `repeat` with `no_env_delimiter`.

If you are making a new option and you want the repeat style of CLI argument parsing, the default for a `repeat` option is `env_delimiter=','`, which preserves your ability to configure via `env`,
and you can customize this if another choice of delimiter is more appropriate.

### Flatten

*Note*: A flatten field's type must be a `struct` that derives `Conf`. 

* `env_prefix` (optional string argument)

   Specifies a string to be prepended to every environment variable of every program option in the target struct.
   If the argument is omitted, it defaults to the upper snake-case of the field name, with an `_` character appended.

* `long_prefix` (optional string argument)

   Specifies a string to be prepended to every long switch of every program option in the target struct.
   If the argument is omitted, it defaults to the kebab-case of the field name, with a `-` character appended.

* `prefix` (optional string argument)

   Specifies a string to be used in place of the field name in the default constructions of `env_prefix` and `long_prefix`.
   If the argument is omitted, it's the same as specifying `env_prefix` and `long_prefix` both with no argument.

   This option cannot be used if `env_prefix` or `long_prefix` is present.
   If none of these options are used, then no prefixing occurs.

* `help_prefix` (optional string argument)

   Specifies that the help strings of every program option of the target struct should be prefixed with a particular string,
   to provide context. If the argument is omitted, it defaults to the doc string on this field.
   If the `help_prefix` attribute is not present then the help strings are unmodified.

   When prefixing is performed, some very simple logic is used to determine how to join the prefix.
   If either the prefix or the help string has multiple lines, then a newline character is used to join them.
   Otherwise a space character is used to join them. (This may change in future revisions.)

#### Notes

Using `flatten` with no additional attributes behaves the same as `clap(flatten)`.
