wsl cargo build --target=x86_64-unknown-linux-gnu
wsl zsh -c "cd /opt/git/github.com/munificent/craftinginterpreters/master && dart tool/bin/test.dart clox --interpreter=/mnt/c/Users/fenhl/git/github.com/fenhl/rlox/stage/target/x86_64-unknown-linux-gnu/debug/rlox"
