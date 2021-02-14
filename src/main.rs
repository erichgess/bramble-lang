#![allow(dead_code)]
#![feature(box_syntax, box_patterns)]

mod ast;
mod compiler;
mod diagnostics;
mod lexer;
mod parser;
mod semantics;

use crate::ast::path::Path;
use ast::{ty::Type};
use clap::{App, Arg};
use compiler::compiler::*;
use diagnostics::config::TracingConfig;
use lexer::tokens::Token;
use semantics::type_resolver::*;

fn main() {
    let config = configure_cli().get_matches();

    let input = config
        .value_of("input")
        .expect("Expected an input source file to compile");
    let text = std::fs::read_to_string(input).expect("Failed to read input file");

    let trace_lexer = TracingConfig::parse(config.value_of("trace-lexer"));
    let mut lexer = crate::lexer::lexer::Lexer::new(&text);
    lexer.set_tracing(trace_lexer);
    let tokens = lexer.tokenize();
    let tokens: Vec<Token> = tokens
        .into_iter()
        .filter(|t| match t {
            Ok(_) => true,
            Err(msg) => {
                println!("{}", msg);
                false
            }
        })
        .map(|t| t.unwrap())
        .collect();

    let trace_parser = TracingConfig::parse(config.value_of("trace-parser"));
    parser::parser::set_tracing(trace_parser);
    let ast = match parser::parser::parse(tokens) {
        Ok(Some(ast)) => ast,
        Ok(None) => {
            println!("Critical: no AST was generated by the parser");
            std::process::exit(ERR_NO_AST);
        }
        Err(msg) => {
            println!("Error: {}", msg);
            std::process::exit(ERR_PARSER_ERROR);
        }
    };

    // Type Check
    let trace_semantic_analysis = TracingConfig::parse(config.value_of("trace-symbol-table"));
    let trace_path = TracingConfig::parse(config.value_of("trace-path"));
    let imported = configure_imported_functions();

    let semantic_ast =
        match resolve_types_with_imports(&ast, &imported, trace_semantic_analysis, trace_path) {
            Ok(ast) => ast,
            Err(msg) => {
                println!("Error: {}", msg);
                std::process::exit(ERR_TYPE_CHECK);
            }
        };

    // Configure the compiler
    let target_platform = config
        .value_of("platform")
        .expect("Must provide a target platform")
        .into();
    let output_target = config.value_of("output").unwrap_or("./target/output.asm");
    let trace_reg_assigner = TracingConfig::parse(config.value_of("trace-reg-assigner"));

    // Compile
    let program = Compiler::compile(
        semantic_ast,
        imported.iter().map(|(p, _, _)| p.clone()).collect(),
        target_platform,
        trace_reg_assigner,
    );

    // Write the resulting assembly code to the target output file
    let mut output = std::fs::File::create(output_target).expect("Failed to create output file");
    Compiler::print(&program, &mut output).expect("Failed to write assembly");
}

// Exit Codes for different types of errors
const ERR_TYPE_CHECK: i32 = 1;
const ERR_NO_AST: i32 = 2;
const ERR_PARSER_ERROR: i32 = 3;

fn configure_cli() -> clap::App<'static, 'static> {
    let app = App::new("Braid Compiler")
        .version("0.1.0")
        .author("Erich Ess")
        .about("Compiles Braid language files into x86 assembly for use by the NASM assembler")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .takes_value(true)
                .required(true)
                .help("Source code file to compile"),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .required(true)
                .help("Name the output file that the assembly will be written to"),
        )
        .arg(
            Arg::with_name("platform")
                .short("p")
                .long("platform")
                .possible_values(&["linux", "machos"])
                .takes_value(true)
                .required(true)
                .help("The target Operation System that this will be compiled for: Linux or Mac (Mac is still unreliable and being worked on)"),
        )
        .arg(
            Arg::with_name("trace-parser")
                .long("trace-parser")
                .takes_value(true)
                .help("Prints out a trace of all the steps the parser follows as it converts the token vector into an AST.  The current token is printed next to the step.
                This is for debugging the parser when adding new syntactical elements.")
        )
        .arg(
            Arg::with_name("trace-reg-assigner")
                .long("trace-reg-assigner")
                .takes_value(true)
                .help("Prints out a trace of all the nodes in the AST and their register assignment annotation data.")
        )
        .arg(
            Arg::with_name("trace-lexer")
                .long("trace-lexer")
                .takes_value(true)
                .help("Prints out a trace of all the steps the lexer follows as it converts the token vector into an AST.  The current token is printed next to the step.
                This is for debugging the lexer when adding new tokens.")
        )
        .arg(
            Arg::with_name("trace-symbol-table")
                .long("trace-symbol-table")
                .takes_value(true)
                .help("Prints out a trace of the value of the symbol table at each node in the AST.  You can specify specify to only trace specific lines in the source code file.")
        )
        .arg(
            Arg::with_name("trace-path")
                .long("trace-path")
                .takes_value(true)
                .help("Prints out the current module path at the current line of code.")
        );
    app
}

fn configure_imported_functions() -> Vec<(Path, Vec<Type>, Type)> {
    vec![
        (
            vec!["root", "std", "io", "write"].into(),
            vec![Type::StringLiteral],
            Type::Unit,
        ),
        (
            vec!["root", "std", "io", "readi64"].into(),
            vec![],
            Type::I64,
        ),
        (
            vec!["root", "std", "io", "writei64"].into(),
            vec![Type::I64],
            Type::Unit,
        ),
        (
            vec!["root", "std", "io", "writei64ln"].into(),
            vec![Type::I64],
            Type::Unit,
        ),
        (
            vec!["root", "std", "io", "writei32"].into(),
            vec![Type::I32],
            Type::Unit,
        ),
        (
            vec!["root", "std", "io", "writei32ln"].into(),
            vec![Type::I32],
            Type::Unit,
        ),
        (
            vec!["root", "std", "io", "writebool"].into(),
            vec![Type::Bool],
            Type::Unit,
        ),
        (
            vec!["root", "std", "io", "writeboolln"].into(),
            vec![Type::Bool],
            Type::Unit,
        ),
    ]
}
