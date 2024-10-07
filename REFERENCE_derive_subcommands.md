# `derive Subcommands` proc-macro reference

The `#[derive(Subcommands)]` macro can only be placed on an `enum`.

When using `#[derive(Subcommands)]`, the result is adjusted by various `#[conf(...)]` attributes that can be applied.
These are documented here.

The `#[conf(...)]` attributes conform to [Rust’s structured attribute convention](https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax).

## Where can conf attributes be used?

The `#[conf(...)]` attributes can appear on an `enum` or a `variant` of the `enum`.

```rust ignore
use conf::Subcommands;

#[derive(Subcommands)]
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

Each enum variant must have one unnamed field, which is a `struct` type which implements [`Conf`].

This is more restrictive than the corresponding `clap` system, which allows named fields in the enum variants,
decorated with attributes equivalent to those that appear on struct fields. For now, to do that in `conf`
you have actually make separate structs. This is equally expressive from the user's point of view, and is easier for us to maintain.

## Enum-level attributes

There are no enum-level attributes at this time.

## Variant-level attributes

*  <a name="variant-name"></a> `name` (string argument)

   example: `#[conf(name = "migrate")]`

   Set the name of this subcommand, which is used to activate the subcommand and is documented in the help.

   If this attribute is not present, the name is the lower snake-case of the variant name.
