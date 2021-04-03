This is `rlox`, a [Rust](https://rust-lang.org/) implementation of [the Lox programming language](https://craftinginterpreters.com/the-lox-language.html) from [Crafting Interpreters](https://craftinginterpreters.com/).

# Implementation

This implementation is still a work in progress (it currently passes [79 out of 246 tests](tests.log)), but implementation details include:

* Overall structure similar to clox, with a bytecode VM.
* Uses existing code from Rust's ecosystem rather than reimplementing things like [hashtables](https://doc.rust-lang.org/std/collections/struct.HashMap.html) or [garbage collection](https://docs.rs/gc).
    * Uses [lalrpop](https://lalrpop.github.io/lalrpop) to parse an AST. This should avoids the rather large number of jumps that clox uses to implement `for` loops, once I get around to implementing them.
* The bytecode format is serializable. The compiler and VM should are callable separately.
    * The serialized bytecode format starts with a `0xc0` byte, which does not occur in valid UTF-8, so the interpreter can run both source code and bytecode without having to be passed any additional command-line options.
    * To make the implementation of the remaining Lox features easier, the bytecode format is not yet stable across versions.

These implementation choices are mostly motivated by the fact that I'm using this as practice, so to speak, for another project I'm planning.

# Things I learned

* Working around the [dangling else problem](https://en.wikipedia.org/wiki/Dangling_else) required a fair amount of code duplication in the parser. For my own language, it would be easier to avoid the problem altogether by always requiring braces like Rust and Swift do.
* Due to the test suite's rather strict requirements regarding error output, I had to implement a custom lexer. This adds line information to runtime errors but makes some of the code much more verbose. When implementing my own language, I think it would be better to omit line information from runtime errors and instead design the language to catch more errors at compile time (e.g. via strong static typing). The [tree before this change](https://github.com/fenhl/rlox/tree/68153cac768cbc6c70399354d67db5fdb989e36c) can be used as a reference.
