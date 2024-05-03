[![Crates.io](https://img.shields.io/crates/v/tested-fixture.svg)](https://crates.io/crates/tested-fixture)
[![Workflow Status](https://github.com/Daniel-Aaron-Bloom/tested-fixture/workflows/main/badge.svg)](https://github.com/Daniel-Aaron-Bloom/tested-fixture/actions?query=workflow%3A%22main%22)

# tested-fixture

Attribute macro for creating fixtures from tests

### Description

Sometimes a series of tests are progressive/incremental; One test is
targetted at verifying Step 1 works as expected, while another is focused
on ensuring Step 2 functions correctly. Common advice is to duplicate all
the work of Step 1 into Step 2's test, or to combine the tests into one
large test. However the former approach can significantly slow down tests,
while the latter can lead to large and unruly testing functions which are
difficult to maintain.

This crate takes a different approach by allowing a test to return a
fixture which can be used in subsequent tests, all through the use of a
single attribute macro [`tested_fixture`].

### Usage

When writing tests for code like:
```rust
struct Foo {
    // ...
}

impl Foo {
    fn step_1() -> Self {
        Foo {
            // Complicated setup...
        }
    }

    fn step_2(&self) {
        // Complicated execution...
    }
}
```

An duplicated test setup would look something like
```rust
#[test]
fn step_1() {
    let foo = Foo::step_1();
    // Complicated assertions verify step 1...
}

#[test]
fn step_2() {
    let foo = Foo::step_1();
    // (Some?) Complicated assertions verify step 1...

    foo.step_2();
    // Complicated assertions verify step 2...
}
```

As you can see, with a lot of steps, this can quickly get out of hand. To
clean it up is straightforward by switching `step_1` to use the
`tested_fixture` attribute instead of the normal `test`.

```rust
// Save the fixture in a static variable called `STEP_1`
#[tested_fixture::tested_fixture(STEP_1)]
fn step_1() -> Foo {
    let foo = Foo::step_1();
    // Complicated assertions verify step 1...
    foo
}

#[test]
fn step_2() {
    STEP_1.step_2();
    // Complicated assertions verify step 2...
}
```

In the case where only `step_2` is run, `STEP_1` will be initialized on
first access. Since the order of tests is not guaranteed, this actually can
occur even if both tests are run. But since results are cached, the
`step_1` test should still reproduce the same result regardless of if it is
the first access or not.

### Advanced usage

The [`tested_fixture`] attribute supports attributes and a visibility level
prefixing the identifier, as well as an optional `: type` suffix. This
optional suffix can be used on tests returning a `Result` to specify that
only `Ok` return values should be captured. For example:

```rust
#[tested_fixture::tested_fixture(pub(crate) STEP_1: Foo)]
fn step_1() -> Result<Foo, &'static str> {
    // ...
}
```

### Limitations

Ordinary test are able to return anything which implements
[`std::process::Termination`], which unlimited nestings of `Result`s. While
this crate does support returning nested `Result` wrappings, it only does
so up to a fixed depth. Additionally it does not support returning any other
types of `Termination`

Right now this crate does not support async tests.


## License

Licensed under 
* MIT license ([LICENSE](LICENSE) or https://opensource.org/licenses/MIT)


[`tested_fixture`]: https://docs.rs/tested-fixture/latest/tested_fixture/attr.tested_fixture.html "attr tested_fixture::tested_fixture"
[`std::process::Termination`]: https://doc.rust-lang.org/nightly/std/process/trait.Termination.html "trait std::process::Termination"
