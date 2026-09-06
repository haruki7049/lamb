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

pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> (Vec<Token<'_>>, Vec<Rich<'_, char, Span>>) {
        let (tokens, errors) = lexer().parse(src).into_output_errors();
        (
            tokens
                .unwrap_or_default()
                .into_iter()
                .map(|(t, _)| t)
                .collect(),
            errors,
        )
    }

    fn lex_spanned<'a>(src: &'a str) -> Vec<(Token<'a>, std::ops::Range<usize>)> {
        let (tokens, _) = lexer().parse(src).into_output_errors();
        tokens
            .unwrap_or_default()
            .into_iter()
            .map(|(t, s)| (t, s.into_range()))
            .collect()
    }

    #[test]
    fn test_token_display() {
        assert_eq!(Token::Int(42).to_string(), "42");
        assert_eq!(Token::Ident("my_var").to_string(), "my_var");
        assert_eq!(Token::Fun.to_string(), "fun");
        assert_eq!(Token::Let.to_string(), "let");
        assert_eq!(Token::In.to_string(), "in");
        assert_eq!(Token::If.to_string(), "if");
        assert_eq!(Token::Then.to_string(), "then");
        assert_eq!(Token::Else.to_string(), "else");
        assert_eq!(Token::Match.to_string(), "match");
        assert_eq!(Token::Type.to_string(), "type");
        assert_eq!(Token::Op("+=").to_string(), "+=");
        assert_eq!(Token::Ctrl('(').to_string(), "(");
        assert_eq!(Token::Arrow.to_string(), "->");
    }

    #[test]
    fn test_integers() {
        let (tokens, errs) = lex("0 42 12345");
        assert!(errs.is_empty());
        assert_eq!(
            tokens,
            vec![Token::Int(0), Token::Int(42), Token::Int(12345)]
        );
    }

    #[test]
    fn test_keywords() {
        let (tokens, errs) = lex("fun let in if then else match type");
        assert!(errs.is_empty());
        assert_eq!(
            tokens,
            vec![
                Token::Fun,
                Token::Let,
                Token::In,
                Token::If,
                Token::Then,
                Token::Else,
                Token::Match,
                Token::Type,
            ]
        );
    }

    #[test]
    fn test_keyword_prefixes() {
        let (tokens, errs) = lex("funny letter inside ifff thenn elsen matching typee");
        assert!(errs.is_empty());
        assert_eq!(
            tokens,
            vec![
                Token::Ident("funny"),
                Token::Ident("letter"),
                Token::Ident("inside"),
                Token::Ident("ifff"),
                Token::Ident("thenn"),
                Token::Ident("elsen"),
                Token::Ident("matching"),
                Token::Ident("typee"),
            ]
        );
    }

    #[test]
    fn test_identifiers() {
        let (tokens, errs) = lex("x foo bar_baz CamelCase _wildcard var123");
        assert!(errs.is_empty());
        assert_eq!(
            tokens,
            vec![
                Token::Ident("x"),
                Token::Ident("foo"),
                Token::Ident("bar_baz"),
                Token::Ident("CamelCase"),
                Token::Ident("_wildcard"),
                Token::Ident("var123"),
            ]
        );
    }

    #[test]
    fn test_operators() {
        let (tokens, errs) = lex("+ - * / = < > ! == != <= >= ->");
        assert!(errs.is_empty());
        assert_eq!(
            tokens,
            vec![
                Token::Op("+"),
                Token::Op("-"),
                Token::Op("*"),
                Token::Op("/"),
                Token::Op("="),
                Token::Op("<"),
                Token::Op(">"),
                Token::Op("!"),
                Token::Op("=="),
                Token::Op("!="),
                Token::Op("<="),
                Token::Op(">="),
                Token::Arrow,
            ]
        );
    }

    #[test]
    fn test_arrow_vs_minus() {
        let (tokens, errs) = lex("-> - ->>");
        assert!(errs.is_empty());
        assert_eq!(
            tokens,
            vec![Token::Arrow, Token::Op("-"), Token::Arrow, Token::Op(">"),]
        );
    }

    #[test]
    fn test_control_chars() {
        let (tokens, errs) = lex("( ) , |");
        assert!(errs.is_empty());
        assert_eq!(
            tokens,
            vec![
                Token::Ctrl('('),
                Token::Ctrl(')'),
                Token::Ctrl(','),
                Token::Ctrl('|'),
            ]
        );
    }

    #[test]
    fn test_comments() {
        let src = r#"
        -- This is a single line comment
        let x = 10 -- comment at end of line
        -- another comment
        in x -- final comment without newline"#;
        let (tokens, errs) = lex(src);
        assert!(errs.is_empty());
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Ident("x"),
                Token::Op("="),
                Token::Int(10),
                Token::In,
                Token::Ident("x"),
            ]
        );
    }

    #[test]
    fn test_spans() {
        let tokens = lex_spanned("let x = 42");
        assert_eq!(
            tokens,
            vec![
                (Token::Let, 0..3),
                (Token::Ident("x"), 4..5),
                (Token::Op("="), 6..7),
                (Token::Int(42), 8..10),
            ]
        );
    }

    #[test]
    fn test_complex_expressions() {
        let src = r#"
        type List a = Nil | Cons(a, List a)

        fun map(f, xs) = match xs
            | Nil -> Nil
            | Cons(y, ys) -> Cons(f(y), map(f, ys))

        let main = if x == 0 then 1 else 0
        "#;
        let (tokens, errs) = lex(src);
        assert!(errs.is_empty());
        assert!(!tokens.is_empty());
        assert_eq!(tokens[0], Token::Type);
        assert_eq!(tokens[1], Token::Ident("List"));
    }

    #[test]
    fn test_lexer_error_recovery() {
        let (tokens, errs) = lex("let @ x = 1");
        assert_eq!(errs.len(), 1);
        assert_eq!(
            tokens,
            vec![Token::Let, Token::Ident("x"), Token::Op("="), Token::Int(1),]
        );
    }
}
