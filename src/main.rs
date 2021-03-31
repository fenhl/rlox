use {
    std::{
        fs::File,
        io::{
            self,
            prelude::*,
        },
        path::PathBuf,
    },
    lalrpop_util::lalrpop_mod,
    structopt::StructOpt,
    crate::{
        error::Result,
        value::FunctionInner,
        vm::Vm,
    },
};

mod ast;
mod compiler;
mod error;
lalrpop_mod!(parser);
mod value;
mod vm;

fn parse(mut source: impl Read) -> Result<Vec<ast::Stmt>> {
    let buf = {
        let mut buf = String::default();
        source.read_to_string(&mut buf)?; //TODO use a lexer that works with Read
        buf
    };
    Ok(parser::ProgramParser::new().parse(&buf)?)
}

fn compile(source: impl Read) -> Result<FunctionInner> {
    compiler::compile(parse(source)?)
}

#[derive(StructOpt)]
struct Args {
    #[structopt(parse(from_os_str))]
    script: Option<PathBuf>,
}

#[wheel::main]
fn main(args: Args) -> Result {
    let mut vm = Vm::new();
    if let Some(script) = args.script {
        vm.interpret(compile(File::open(script)?)?)?;
    } else {
        // repl
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        loop {
            print!("> ");
            stdout.flush()?;
            let mut line = String::default();
            stdin.read_line(&mut line)?;
            if line.trim().is_empty() { break }
            vm.interpret(compile(line.as_bytes())?)?;
        }
    }
    Ok(())
}
