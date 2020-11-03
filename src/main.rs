#![allow(dead_code)]

mod compiler;
mod lexer;
mod parser;
mod type_checker;

use clap::{App, Arg};
use compiler::*;
use lexer::Token;
use parser::*;
use type_checker::*;

fn main() {
    let matches = App::new("Braid Compiler")
        .version("0.1.0")
        .author("Erich Ess")
        .about("Compiles Braid language files into x86 assembly for use by the NASM assembler")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .takes_value(true)
                .help("Source code file to compile"),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Name the output file that the assembly will be written to"),
        )
        .get_matches();

    let input = matches
        .value_of("input")
        .expect("Expected an input source file to compile");
    let text = std::fs::read_to_string(input).expect("Failed to read input file");

    println!("Code: {}", text);
    let tokens = lexer::tokenize(&text);
    println!("Tokens: {:?}", tokens);
    let tokens = tokens
        .into_iter()
        .filter(|t| t.is_ok())
        .map(|t| t.unwrap())
        .collect();
    let ast = Node::parse(tokens);
    println!("AST: {:?}", ast);

    let ast = ast.unwrap();
    let mut func_table = FunctionTable::generate(&ast);
    println!("FuncTable: {:?}", func_table);

    // Type Check
    match checker::type_check(&ast, &func_table) {
        Ok(_) => (),
        Err(msg) => panic!(msg),
    }

    let program = Compiler::compile(&ast, &mut func_table);
    let output_target = matches.value_of("output").unwrap_or("./target/output.asm");
    let mut output = std::fs::File::create(output_target).expect("Failed to create output file");
    //let mut output = std::io::stdout();
    program
        .print(&mut output)
        .expect("Failed to write assembly");
}
