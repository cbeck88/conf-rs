# `derive Subcommands` proc-macro reference

The `#[derive(Subcommands)]` macro can only be placed on an `enum`.

When using `#[derive(Subcommands)]`, the result is adjusted by various `#[conf(...)]` attributes that can be applied.
These are documented here.

The `#[conf(...)]` attributes conform to [Rust’s structured attribute convention](https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax).

* [Where can conf attributes be used?](#where-can-conf-attributes-be-used)
* [Enum-level attributes](#enum-level-attributes)
    * [serde](#enum-serde)
* [Variant-level attributes](#variant-level-attributes)
    * [name](#variant-name)
    * [serde](#variant-serde)
      * [rename](#variant-serde-rename)
      * [skip](#variant-serde-skip)

## Where can conf attributes be used?

The `#[conf(...)]` attributes can appear on an `enum` or a `variant` of the `enum`.

```rust ignore
use conf::Subcommands;

#[derive(Subcommands)]
#[conf(serde)] // This is an enum-level attribute
pub enum MySubcommands {
    Run(RunConfig),
    // This is a variant-level attribute
    #[conf(name = "migrate")]
    RunMigrations(MigrateConfig),
    // This is also a variant-level attribute
    #[conf(name = "validate")]
    RunValidation(ValidateConfig),
}
```

Each enum variant must have one unnamed field, which is a `struct` type which implements [`Conf`] [^compat-note-1].

## Enum-level attributes

*  <a name="enum-serde"></a> `serde` (no arguments)

   example: `#[conf(serde)]`, `#[subcommands(serde)]`

   Enable the serde integration on this enum.

   The interaction with `serde` is:

   * Each subcommand that is not `#[conf(serde(skip))]` now has a serialization name as well.
   * If that key appears in the serde document, *and* the subcommand appears in the CLI args,
     then the subcommand variant reads from the corresponding corresponding value in the serde document.
   * If the key appears in the serde document, but the subcommand *does not* appear in the CLI args,
     then this serde value is simply ignored, and it is not an error.

   This allows the previous example to work with a TOML config file structured like this:

   ```toml
   [run]
   run_param = "..."

   [run_migrations]
   migrations_param = "..."

   [run_validation]
   validation_param = "..."
   ```

   If you invoke `./my_prog run`, the `run` subcommand will pick up values from the `[run]` block,
   and the other sections won't cause an error even though they are unused.
   Similarly `./my_prog migrate` would pick up values from the `[run_migrations]` block, without errors.

   You can change the serialization name of a subcommand using `#[conf(serde(rename = "..."))]`.

   You may also prefer that two or more subcommands that have a lot of overlap read from the same
   section of the config file. For this, you can just make the serialization names the same [^2].

## Variant-level attributes

*  <a name="variant-name"></a> `name` (string argument)

   example: `#[conf(name = "migrate")]`

   Set the name of this subcommand, which is used to activate the subcommand and is documented in the help.

   If this attribute is not present, the name is the lower snake-case of the variant name.

*  <a name="variant-serde"></a> `serde` (optional additional attributes)

   example: `#[conf(serde(rename = "foo"))]`

   Configuration specific to the serde integration.

   * <a name="variant-serde-rename"></a> `rename` (string argument)

     example: `#[conf(serde(rename = "foo"))]`

     Similar to `#[serde(rename)]`, changes the name used in serialization.

     If this attribute is not present, the serialization name is the lower snake-case of the variant name.

   * <a name="variant-serde-skip"></a> `skip` (no arguments)

     example: `#[conf(serde(skip))]`

     Similar to `#[serde(skip)]`, this subcommand won't read data from the serde value source.

[^compat-note-1]: This is more restrictive than the corresponding `clap` system for subcommands, which allows named fields in the enum variants,
decorated with attributes equivalent to those that appear on struct fields. For now, to do that in `conf`
you have to declare separate structs. This is equally expressive from the user's point of view, and is easier for us to maintain.
[^2]: Normally, making two fields have the same serialization name won't work in `serde`. In `serde` it is only possible to deserialize a value at most once,
      so you can't populate two different fields with the same deserializer content. Also it would likely break `Serialize`. In this case, we aren't serializing
      anything, and the enum semantics ensure that we will only deserialize this value at most once.
