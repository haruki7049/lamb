// Lexer stage: turns raw source text into a stream of spanned tokens.
// Kept as its own stage (instead of parsing text directly) so the
// token-level parser in `parser.rs` doesn't have to deal with whitespace
// or comments at all.

use chumsky::prelude::*;
use chumsky::span::SimpleSpan;

pub type Span = SimpleSpan;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'src> {
    Int(i64),
    Ident(&'src str),
    // Keywords
    Fun,
    Let,
    In,
    If,
    Then,
    Else,
    Match,
    Type,
    // Operators, kept as raw strings like in chumsky's nano_rust example
    Op(&'src str),
    // Punctuation: ( ) , | ->
    Ctrl(char),
    Arrow, // "->"
}

impl<'src> std::fmt::Display for Token<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Int(n) => write!(f, "{n}"),
            Token::Ident(s) => write!(f, "{s}"),
            Token::Fun => write!(f, "fun"),
            Token::Let => write!(f, "let"),
            Token::In => write!(f, "in"),
            Token::If => write!(f, "if"),
            Token::Then => write!(f, "then"),
            Token::Else => write!(f, "else"),
            Token::Match => write!(f, "match"),
            Token::Type => write!(f, "type"),
            Token::Op(s) => write!(f, "{s}"),
            Token::Ctrl(c) => write!(f, "{c}"),
            Token::Arrow => write!(f, "->"),
        }
    }
}

pub fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
    let int = text::int(10).from_str().unwrapped().map(Token::Int);

    // "->" must be tried before the single-char '-' operator.
    let arrow = just("->").to(Token::Arrow);

    let op = one_of("+-*/=<>!")
        .repeated()
        .at_least(1)
        .to_slice()
        .map(Token::Op);

    let ctrl = one_of("(),|").map(Token::Ctrl);

    let ident = text::ascii::ident().map(|ident: &str| match ident {
        "fun" => Token::Fun,
        "let" => Token::Let,
        "in" => Token::In,
        "if" => Token::If,
        "then" => Token::Then,
        "else" => Token::Else,
        "match" => Token::Match,
        "type" => Token::Type,
        _ => Token::Ident(ident),
    });

    let token = int.or(arrow).or(op).or(ctrl).or(ident);

    let comment = just("--")
        .then(any().and_is(just('\n').not()).repeated())
        .padded();

    token
        .map_with(|tok, e| (tok, e.span()))
        .padded_by(comment.repeated())
        .padded()
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect()
}
