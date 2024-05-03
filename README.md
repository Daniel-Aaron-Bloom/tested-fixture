# tested-fixture
[![Crates.io](https://img.shields.io/crates/v/tested-fixture.svg)](https://crates.io/crates/tested-fixture)
[![Workflow Status](https://github.com/Daniel-Aaron-Bloom/tested-fixture/workflows/Rust/badge.svg)](https://github.com/Daniel-Aaron-Bloom/tested-fixture/actions?query=workflow%3A%22Rust%22)

Attribute macro for creating fixtures from tests

### Description

Sometimes a series of tests are progressive or incremental; that is to say
one test builds on another. A multi-stage test might have complicated
setup and verification processes for each step, but with clear boundaries
between stages (`test_1` verifies stage 1, `test_2` verifies stage 2, etc.
). The problem arises when stages want to share data (i.e. `test_2` wants
to start where `test_1` left off).

Common advice is to duplicate all the setup code across all tests, or
alternatively to combine the tests into one large test. However the former
approach can significantly slow down tests if setup is costly, and also
introduces significant test maintenance costs if setup procedures change.
The latter however can lead to large and unruly testing functions which are
difficult to maintain, and doesn't solve the problem when dependencies
cross multiple files (i.e. unit tests which test the full setup process for a
`Foo` are difficult to combine with unit tests which test the setup process
of a `Bar` which relies on a fully constructed `Foo`; should the "combined"
test live near `Foo` or `Bar`? What if the tests needs to access internals to
verify assertions?).

This crate provides an alternative approach by allowing a test to return a
fixture which can be used in subsequent tests. Tests can opt in to this
functionality by using a single attribute macro [`tested_fixture`].

### Usage

When writing tests for code like:
```rust
struct Foo {
    // ...
}

struct State {
    // ...
}

impl Foo {
    fn step_1() -> Self {
        Foo {
            // Complicated setup...
        }
    }

    fn step_2(&self) -> State {
        State {
            // Complicated execution...
        }
    }

    fn step_3(&self, v: &State) {
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

#[test]
fn step_3() {
    let foo = Foo::step_1();
    // (Some?) Complicated assertions verify step 1...

    let state = foo.step_2();
    // (Some?) Complicated assertions verify step 2...

    foo.step_3(&state);
    // Complicated assertions verify step 3...
}
```

As you can see, with a lot of steps, this can quickly get out of hand. To
clean it up is straightforward by switching to use the
`tested_fixture` attribute instead of the normal `test`.

```rust
// Save the fixture in a static variable called `STEP_1`
#[tested_fixture::tested_fixture(STEP_1)]
fn step_1() -> Foo {
    let foo = Foo::step_1();
    // Complicated assertions verify step 1...
    foo
}

#[tested_fixture::tested_fixture(STEP_2_STATE)]
fn step_2() -> State {
    let state = STEP_1.step_2()
    // Complicated assertions verify step 2...
    state
}

#[test]
fn step_3() {
    STEP_1.step_3(&STEP_2_STATE);
    // Complicated assertions verify step 3...
}
```

Note that when only `step_2` is run, `STEP_1` will be initialized on
first access. Since the order of tests is not guaranteed, this actually can
occur even if both tests are run. But since results are cached, the
`step_1` test should still succeed (or fail) regardless of if it is run
first or not.

### Advanced usage

The [`tested_fixture`] attribute supports attributes and a visibility level
prefixing the identifier, as well as an optional `: type` suffix. This
optional suffix can be used on tests returning a `Result` to specify that
only `Ok` return values should be captured. For example:

```rust
#[tested_fixture::tested_fixture(
    /// Doc comment on the `STEP_1` global variable
    pub(crate) STEP_1: Foo
)]
fn step_1() -> Result<Foo, &'static str> {
    // ...
}
```

### Limitations

Ordinary `#[test]` functions are able to return anything which implements
[`std::process::Termination`], including unlimited nestings of `Result`s.
While this crate does support returning nested `Result` wrappings, it only
does so up to a fixed depth. Additionally it does not support returning any
other `Termination` implementations besides `Result`.

As with all testing-related global state, it is recommended that tests don't
mutate the state, as doing so will increase the risk of flaky tests due to
changes in execution order or timing. Thankfully this is the default
behavior, as all fixtures defined by this crate are only accessible by
non-mutable reference.

Right now this crate does not support async tests.

## License

Licensed under 
* MIT license ([LICENSE](LICENSE) or https://opensource.org/licenses/MIT)


[`tested_fixture`]: https://docs.rs/tested-fixture/latest/tested_fixture/attr.tested_fixture.html "attr tested_fixture::tested_fixture"
[`std::process::Termination`]: https://doc.rust-lang.org/nightly/std/process/trait.Termination.html "trait std::process::Termination"
