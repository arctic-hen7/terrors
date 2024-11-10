# terrors - the Rust error **handling** library

Handling errors means taking a set of possible error
types, removing the ones that are locally addressible,
and then if the set of errors is not within those local
concerns, propagating the remainder to a caller. The
caller should not receive the local errors of the callee.

# Principles

* Error types should be precise.
  * `terrors::OneOf` solves this by making precise sets of possible errors:
    * low friction to specify
    * low friction to narrow by specific error handlers
    * low friction to broaden to pass up the stack
* Error handling should follow the single responsibility principle
    * if every error in a system is spread everywhere else, there
      is no clear responsibility for where it needs to be handled.
* No macros.
    * Users should not have to learn some new DSL for error handling that every macro entails.

# Examples

```rust
use terrors::OneOf;

let one_of_3: OneOf<(String, u32, Vec<u8>)> = OneOf::new(5);

let narrowed_res: Result<u32, OneOf<(String, Vec<u8>)>> =
    one_of_3.narrow();

assert_eq!(5, narrowed_res.unwrap());
```

OneOf can also be broadened to a superset, checked at compile-time.

```rust
use terrors::OneOf;

struct Timeout;
struct AllocationFailure;
struct RetriesExhausted;

fn allocate_box() -> Result<Box<u8>, OneOf<(AllocationFailure,)>> {
    Err(AllocationFailure.into())
}

fn send() -> Result<(), OneOf<(Timeout,)>> {
    Err(Timeout.into())
}

fn allocate_and_send() -> Result<(), OneOf<(AllocationFailure, Timeout)>> {
    let boxed_byte: Box<u8> = allocate_box().map_err(OneOf::broaden)?;
    send().map_err(OneOf::broaden)?;

    Ok(())
}

fn retry() -> Result<(), OneOf<(AllocationFailure, RetriesExhausted)>> {
    for _ in 0..3 {
        let Err(err) = allocate_and_send() else {
            return Ok(());
        };

        // keep retrying if we have a Timeout,
        // but punt allocation issues to caller.
        match err.narrow::<Timeout, _>() {
            Ok(_timeout) => {},
            Err(one_of_others) => return Err(one_of_others.broaden()),
        }
    }

    Err(OneOf::new(RetriesExhausted))
}
```

`OneOf` also implements `Clone`, `Debug`, `Display`, `Send`, `Sync` and/or `std::error::Error` if all types in the type set do as well:

```rust
use std::error::Error;
use std::io;
use terrors::OneOf;

let o_1: OneOf<(u32, String)> = OneOf::new(5_u32);

// Debug is implemented if all types in the type set implement Debug
dbg!(&o_1);

// Display is implemented if all types in the type set implement Display
println!("{}", o_1);

let cloned = o_1.clone();

type E = io::Error;
let e = io::Error::new(io::ErrorKind::Other, "wuaaaaahhhzzaaaaaaaa");

let o_2: OneOf<(E,)> = OneOf::new(e);

// std::error::Error is implemented if all types in the type set implement it
dbg!(o_2.description());
```

OneOf can also be turned into an owned or referenced enum form:

```rust
use terrors::{OneOf, E2};

let o_1: OneOf<(u32, String)> = OneOf::new(5_u32);

match o_1.as_enum() {
    E2::A(u) => {
        println!("handling reference {u}: u32")
    }
    E2::B(s) => {
        println!("handling reference {s}: String")
    }
}

match o_1.to_enum() {
    E2::A(u) => {
        println!("handling owned {u}: u32")
    }
    E2::B(s) => {
        println!("handling owned {s}: String")
    }
}
```

### Motivation

The paper [Simple Testing Can Prevent Most Critical Failures: An Analysis of Production Failures in Distributed Data-intensive Systems](https://www.eecg.toronto.edu/~yuan/papers/failure_analysis_osdi14.pdf)
is goldmine of fascinating statistics that illuminate the
software patterns that tend to correspond to system failures.
This is one of my favorites:

```no_compile
almost all (92%) of the catastrophic system failures
are the result of incorrect handling of non-fatal errors
explicitly signaled in software.
```

Our systems are falling over because we aren't handling
our errors. We're doing fine when it comes to signalling
their existence, but we need to actually handle them.

When we write Rust, we tend to encounter a variety of different
error types. Sometimes we need to put multiple possible errors
into a container that is then returned from a function, where
the caller or a transitive caller is expected to handle the
specific problem that arose.

As we grow a codebase, more of these situations pop up.
While it's not so much effort to write custom enums in
one or two places that hold the precise set of possible
errors, most people resort to one of two strategies for
minimizing the effort that goes into propagating their
error types:
* A large top-level enum that holds variants for errors
  originating across the codebase, tending to grow
  larger and larger over time, undermining the ability
  to use exhaustive pattern matching to confidently
  ensure that local concerns are not bubbling up the stack.
* A boxed trait that is easy to convert errors into, but
 then hides information about what may actually be inside.
 You don't know where it's been or where it's going.

As the number of different source error types that these
error containers hold increases, the amount of information
that the container communicates to people who encounter it
decreases. It becomes increasingly unclear what the error
container actually holds. As the precision of the type
goes down, so does a human's ability to reason about
where the appropriate place is to handle any particular
concern within it.

We have to increase the precision in our error types.

People don't write a precise enum for every function that
may only return some subset of errors because we would
end up with a ton of small enum types that only get used in
one or two places. This is the pain that drives people
to using overly-broad error enums or overly-smooth
boxed dynamic error traits, reducing their ability to
handle their errors.

### Cool stuff

This crate is built around `OneOf`, which functions as
a form of anonymous enum that can be narrowed in ways
that may be familiar for users of TypeScript etc...
Our error containers need to get smaller as individual
errors are peeled off and handled, leaving the reduced
remainder of possible error types if the local concerns
are not present.

The cool thing about it is that it is built on top of a
type-level heterogenous set of possible error types,
where there's only one actual value among the different
possibilities.

Rather than having a giant ball of mud enum or
boxed trait object that is never clear what it actually
contains, causing you to never handle individual
concerns from, the idea of this is that you can
have a minimized set of actual error types that may
thread through the stack.

The nice thing about this type-level set of possibilities
is that any specific type can be peeled off while narrowing
the rest of the types if the narrowing fails. Both narrowing
and broadening are based on compile-time error type set checking.

### The Trade-Off

Type-level programming is something that I have tried hard to avoid
for most of my career due to confusing error messages resulting
from compilation errors. These complex type checking failures
produce errors that are challenging to reason about, and can often
take several minutes to understand.

I have tried hard to avoid exposing users of `terrors` to too many
of the sharp edges in the underlying type machinery, but it is likely
that if the source and destination type sets do not satisfy the `SupersetOf`
trait in the right direction depending on whether `narrow` or
`broaden` is being called, that the error will not be particularly
pleasant to read. Just know that errors pretty much always mean
that the superset relationship does not hold as required.

Going forward, I believe most of the required traits can be implemented
in ways that expose users to errors that look more like `(A, B) does not
implement SupersetOf<(C, D), _>` instead of `Cons<A, Cons<B, End>> does
not implement SupersetOf<Cons<C, Cons<D, End>>>` by leaning into the
bidirectional type mapping that exists between the heterogenous type
set `Cons` chains and more human-friendly type tuples.

### Special Thanks

Much of the fancy type-level logic for reasoning about sets of error types
was directly inspired by [frunk](https://docs.rs/frunk/latest/frunk/).
I had been wondering for years about the feasibility of a data structure
like `OneOf`, and had often assumed it was impossible, until I finally
had an extended weekend to give it a deep dive. After many false starts,
I finally came across [an article](https://archive.is/YwDMX) written by
[lloydmeta](https://github.com/lloydmeta) (the author of frunk) about how
frunk handles several related concerns in the context of a heterogenous
list structure. Despite having used Rust for over 10 years, that article
taught me a huge amount about how the language's type system can be
used in interesting ways that addressed very practical needs. In particular,
the general perspective in that blog post about how you can implement
traits in a recursive way that is familiar from other functional languages
was the missing primitive for working with Rust that I had not realized
was possible for my first decade with the language. Thank you very
much for creating frunk and telling the world about how you did it!
