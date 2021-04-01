#![deny(unused_must_use)]

use {
    std::{
        fs::File,
        io::{
            self,
            Cursor,
            SeekFrom,
            prelude::*,
        },
        path::PathBuf,
    },
    byteorder::ReadBytesExt as _,
    lalrpop_util::lalrpop_mod,
    structopt::StructOpt,
    crate::{
        error::{
            Error,
            Result,
        },
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

fn compile(mut source: impl Read + Seek) -> Result<FunctionInner> {
    match source.read_u8() {
        Ok(0xc0) => FunctionInner::read(&mut source, true),
        Ok(_) => {
            source.seek(SeekFrom::Current(-1))?; // first byte was not the bytecode magic byte, unread it
            compiler::compile(parse(source)?)
        }
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => compiler::compile(Vec::default()),
        Err(e) => Err(e.into()),
    }
}

#[derive(StructOpt)]
struct Args {
    /// Only compile the script and dump the bytecode to stdout.
    #[structopt(long)]
    compile: bool,
    /// When used with `--compile`, dump the bytecode to the given path instead of stdout.
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
    /// Print a disassembly of the bytecode before running/dumping it.
    #[structopt(long)]
    disassemble: bool,
    /// The path to a Lox script or bytecode dump that will be run. If omitted, a repl is started.
    #[structopt(parse(from_os_str))]
    script: Option<PathBuf>,
}

#[wheel::main(custom_exit)]
fn main(args: Args) -> Result {
    let mut vm = Vm::new();
    if let Some(script) = args.script {
        let bytecode = compile(File::open(script)?)?;
        if args.disassemble { bytecode.disassemble(); }
        if args.compile {
            let mut output = if let Some(out_path) = args.output {
                Box::new(File::create(out_path)?) as Box<dyn Write>
            } else {
                Box::new(io::stdout())
            };
            bytecode.write(&mut output)?;
        } else {
            vm.interpret(bytecode)?;
        }
    } else {
        if args.compile { return Err(Error::CompileRepl) }
        // repl
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        loop {
            print!("> ");
            stdout.flush()?;
            let mut line = String::default();
            stdin.read_line(&mut line)?;
            if line.trim().is_empty() { break }
            let bytecode = compile(Cursor::new(line.as_bytes()))?;
            if args.disassemble { bytecode.disassemble(); }
            vm.interpret(bytecode)?;
        }
    }
    Ok(())
}
