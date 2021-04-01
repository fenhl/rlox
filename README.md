This is `rlox`, a [Rust](https://rust-lang.org/) implementation of [the Lox programming language](https://craftinginterpreters.com/the-lox-language.html) from [Crafting Interpreters](https://craftinginterpreters.com/).

# Implementation

This implementation is still a work in progress (it currently passes 47 out of 246 tests), but implementation details include:

* Overall structure similar to clox, with a bytecode VM.
* Uses existing code from Rust's ecosystem rather than reimplementing things like [hashtables](https://doc.rust-lang.org/std/collections/struct.HashMap.html) or [garbage collection](https://docs.rs/gc).
    * Uses [lalrpop](https://lalrpop.github.io/lalrpop) to parse an AST. This should avoids the rather large number of jumps that clox uses to implement `for` loops, once I get around to implementing them.
* The bytecode format is serializable. The compiler and VM should are callable separately.
    * The serialized bytecode format starts with a `0xc0` byte, which does not occur in valid UTF-8, so the interpreter can run both source code and bytecode without having to be passed any additional command-line options.
    * To make the implementation of the remaining Lox features easier, the bytecode format is not yet stable across versions.

These implementation choices are mostly motivated by the fact that I'm using this as practice, so to speak, for another project I'm planning.
