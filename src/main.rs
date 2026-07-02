use chumsky::input::Input;
use chumsky::Parser;
use std::{env, fs, process};

mod ast;
mod lexer;
mod parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <source_file>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    let src = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file: {err}");
        process::exit(1);
    });

    // 1. Lexing: text -> Vec<(Token, Span)>
    let (tokens, lex_errs) = lexer::lexer().parse(src.as_str()).into_output_errors();

    for e in &lex_errs {
        eprintln!("Lex error: {e}");
    }

    let Some(tokens) = tokens else {
        process::exit(1);
    };

    // 2. Parsing: Vec<(Token, Span)> -> Program (AST)
    // `map` here turns the token vec into something chumsky can treat as
    // an `Input` in its own right, complete with per-token spans, so the
    // token-level parser never has to look at raw source text again.
    let token_stream = tokens
        .as_slice()
        .map((src.len()..src.len()).into(), |(t, s)| (t, s));

    let (ast, parse_errs) = parser::program_parser()
        .parse(token_stream)
        .into_output_errors();

    for e in &parse_errs {
        eprintln!("Parse error: {e}");
    }

    match ast {
        Some(program) if lex_errs.is_empty() && parse_errs.is_empty() => {
            println!("{program:#?}");
        }
        _ => process::exit(1),
    }
}
