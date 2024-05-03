//! Attribute macro for creating fixtures from tests
//!
//! ## Description
//!
//! Sometimes a series of tests are progressive or incremental; that is to say
//! one test builds on another. A multi-stage test might have complicated
//! setup and verification processes for each step, but with clear boundaries
//! between stages (`test_1` verifies stage 1, `test_2` verifies stage 2, etc.
//! ). The problem arises when stages want to share data (i.e. `test_2` wants
//! to start where `test_1` left off).
//!
//! Common advice is to duplicate all the setup code across all tests, or
//! alternatively to combine the tests into one large test. However the former
//! approach can significantly slow down tests if setup is costly, and also
//! introduces significant test maintenance costs if setup procedures change.
//! The latter however can lead to large and unruly testing functions which are
//! difficult to maintain, and doesn't solve the problem when dependencies
//! cross multiple files (i.e. unit tests which test the full setup process for a
//! `Foo` are difficult to combine with unit tests which test the setup process
//! of a `Bar` which relies on a fully constructed `Foo`; should the "combined"
//! test live near `Foo` or `Bar`? What if the tests needs to access internals to
//! verify assertions?).
//!
//! This crate provides an alternative approach by allowing a test to return a
//! fixture which can be used in subsequent tests. Tests can opt in to this
//! functionality by using a single attribute macro [`tested_fixture`].
//!
//! ## Usage
//!
//! When writing tests for code like:
//! ```
//! struct Foo {
//!     // ...
//! }
//!
//! struct State {
//!     // ...
//! }
//!
//! impl Foo {
//!     fn step_1() -> Self {
//!         Foo {
//!             // Complicated setup...
//!         }
//!     }
//!
//!     fn step_2(&self) -> State {
//!         State {
//!             // Complicated execution...
//!         }
//!     }
//!
//!     fn step_3(&self, v: &State) {
//!         // Complicated execution...
//!     }
//! }
//! ```
//!
//! An duplicated test setup would look something like
//! ```
//! #[test]
//! fn step_1() {
//!     let foo = Foo::step_1();
//!     // Complicated assertions verify step 1...
//! }
//!
//! #[test]
//! fn step_2() {
//!     let foo = Foo::step_1();
//!     // (Some?) Complicated assertions verify step 1...
//!
//!     foo.step_2();
//!     // Complicated assertions verify step 2...
//! }
//!
//! #[test]
//! fn step_3() {
//!     let foo = Foo::step_1();
//!     // (Some?) Complicated assertions verify step 1...
//!
//!     let state = foo.step_2();
//!     // (Some?) Complicated assertions verify step 2...
//!
//!     foo.step_3(&state);
//!     // Complicated assertions verify step 3...
//! }
//! ```
//!
//! As you can see, with a lot of steps, this can quickly get out of hand. To
//! clean it up is straightforward by switching to use the
//! `tested_fixture` attribute instead of the normal `test`.
//!
//! ```
//! // Save the fixture in a static variable called `STEP_1`
//! #[tested_fixture::tested_fixture(STEP_1)]
//! fn step_1() -> Foo {
//!     let foo = Foo::step_1();
//!     // Complicated assertions verify step 1...
//!     foo
//! }
//!
//! #[tested_fixture::tested_fixture(STEP_2_STATE)]
//! fn step_2() -> State {
//!     let state = STEP_1.step_2();
//!     // Complicated assertions verify step 2...
//!     state
//! }
//!
//! #[test]
//! fn step_3() {
//!     STEP_1.step_3(&STEP_2_STATE);
//!     // Complicated assertions verify step 3...
//! }
//! ```
//!
//! Note that when only `step_2` is run, `STEP_1` will be initialized on
//! first access. Since the order of tests is not guaranteed, this actually can
//! occur even if both tests are run. But since results are cached, the
//! `step_1` test should still succeed (or fail) regardless of if it is run
//! first or not.
//!
//! ## Advanced usage
//!
//! The [`tested_fixture`] attribute supports attributes and a visibility level
//! prefixing the identifier, as well as an optional `: type` suffix. This
//! optional suffix can be used on tests returning a `Result` to specify that
//! only `Ok` return values should be captured. For example:
//!
//! ```
//! #[tested_fixture::tested_fixture(
//!     /// Doc comment on the `STEP_1` global variable
//!     pub(crate) STEP_1: Foo
//! )]
//! fn step_1() -> Result<Foo, &'static str> {
//!     // ...
//! }
//! ```
//!
//! ## Limitations
//!
//! Ordinary `#[test]` functions are able to return anything which implements
//! [`std::process::Termination`], including unlimited nestings of `Result`s.
//! While this crate does support returning nested `Result` wrappings, it only
//! does so up to a fixed depth. Additionally it does not support returning any
//! other `Termination` implementations besides `Result`.
//!
//! As with all testing-related global state, it is recommended that tests don't
//! mutate the state, as doing so will increase the risk of flaky tests due to
//! changes in execution order or timing. Thankfully this is the default
//! behavior, as all fixtures defined by this crate are only accessible by
//! non-mutable reference.
//!
//! Right now this crate does not support async tests.

#![warn(missing_docs)]
#![allow(clippy::test_attr_in_doctest)]

pub use tested_fixture_macros::tested_fixture;

#[doc(hidden)]
pub use tested_fixture_macros::tested_fixture_doctest;

#[doc(hidden)]
pub mod helpers {
    use std::{
        convert::Infallible,
        fmt::Debug,
        process::{ExitCode, Termination},
    };

    // Re-exports
    pub use once_cell::sync::{Lazy, OnceCell};

    /// A helper trait to unify `Result` fixtures types
    pub trait MakeResultRef {
        type Output;
        fn make(self) -> Self::Output;
    }

    impl<T, E: Debug> MakeResultRef for &'static Result<T, E> {
        type Output = Result<&'static T, &'static E>;
        fn make(self) -> Self::Output {
            self.as_ref()
        }
    }

    /// A helper struct for wrapping fixtures
    pub struct ReportSuccess<T>(pub T);

    impl<T> Termination for ReportSuccess<T> {
        fn report(self) -> ExitCode {
            ExitCode::SUCCESS
        }
    }

    /// Helper trait for unwrapping fixtures
    pub trait StaticallyBorrow {
        type T;
        fn static_borrow(&self) -> Self::T;
    }

    impl<T> StaticallyBorrow for &'static T {
        type T = &'static T;
        fn static_borrow(&self) -> Self::T {
            self
        }
    }

    impl<T: StaticallyBorrow> StaticallyBorrow for Result<T, Infallible> {
        type T = T::T;
        fn static_borrow(&self) -> Self::T {
            match self.as_ref() {
                Ok(v) => v.static_borrow(),
                Err(_) => unreachable!(),
            }
        }
    }

    impl<T: StaticallyBorrow> StaticallyBorrow for ReportSuccess<T> {
        type T = T::T;
        fn static_borrow(&self) -> Self::T {
            self.0.static_borrow()
        }
    }

    /// Helper trait for unwrapping fixtures
    pub trait Unwrap<T>: Termination {
        fn unwrap(self, context: &str) -> &'static T;
    }

    impl<T: 'static, R: StaticallyBorrow<T = &'static T>> Unwrap<T> for ReportSuccess<R> {
        fn unwrap(self, _context: &str) -> &'static T {
            self.static_borrow()
        }
    }

    impl<T, R: Unwrap<T>, E: Debug> Unwrap<T> for Result<R, E> {
        fn unwrap(self, context: &str) -> &'static T {
            match self {
                Ok(v) => v.unwrap(context),
                Err(e) => panic!("{} failed: {:?}", context, e),
            }
        }
    }

    /// A helper struct to unify non-`Result` fixtures types
    pub struct Fixer<T>(pub T);
    impl<T: MakeResultRef> Fixer<T> {
        pub fn fix(self) -> T::Output {
            self.0.make()
        }
    }

    /// A helper trait to unify non-`Result` fixtures types

    pub trait Fix {
        type Fixed;
        fn fix(self) -> Self::Fixed;
    }

    impl<T: 'static> Fix for Fixer<T> {
        type Fixed = Result<ReportSuccess<T>, Infallible>;
        fn fix(self) -> Self::Fixed {
            Ok(ReportSuccess(self.0))
        }
    }

    /// A helper function to get fixtures from test functions
    pub fn unwrap<T, R, F>(f: F) -> &'static T
    where
        T: 'static,
        R: Unwrap<T>,
        F: FnOnce() -> R,
    {
        let context = core::any::type_name::<F>();
        f().unwrap(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct HeavySetup(u32);

    impl HeavySetup {
        fn build(v: u32) -> Self {
            HeavySetup(v)
        }
    }

    #[tested_fixture(
        /// This is a test fixture
        pub(crate) SETUP_0
    )]
    fn setup_with_attributes() -> HeavySetup {
        HeavySetup::build(1)
    }

    #[tested_fixture(SETUP_1)]
    fn setup() -> HeavySetup {
        HeavySetup::build(1)
    }

    #[tested_fixture(SETUP_2: HeavySetup)]
    fn try_setup() -> Result<HeavySetup, &'static str> {
        Ok(HeavySetup::build(2))
    }

    #[tested_fixture(SETUP_3: HeavySetup)]
    #[ignore = "fails"]
    fn fail_setup() -> Result<HeavySetup, &'static str> {
        Err("failed due to reticulated splines")
    }

    #[tested_fixture(SETUP_4)]
    #[ignore = "fails"]
    fn panic_setup() -> HeavySetup {
        panic!("failed due to normalized social network")
    }

    #[test]
    fn combine_setup() {
        let _ = HeavySetup::build(SETUP_1.0 + SETUP_2.0);
    }

    #[test]
    #[should_panic(
        expected = r#"tested_fixture::tests::fail_setup failed: "failed due to reticulated splines""#
    )]
    fn combine_fail() {
        let _ = HeavySetup::build(SETUP_1.0 + SETUP_3.0);
    }

    #[test]
    #[should_panic(expected = r#"tested_fixture::tests::panic_setup failed: "panicked""#)]
    fn combine_panic() {
        let _ = HeavySetup::build(SETUP_1.0 + SETUP_4.0);
    }
}
