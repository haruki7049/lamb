// Parser stage: consumes the token stream produced by `lexer.rs` and
// builds the AST defined in `ast.rs`.

use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast::{self, *};
use crate::lexer::{Span, Token};

// `chumsky::prelude` and `crate::ast` both export a `Spanned` name;
// pin down which one we mean explicitly.
type Spanned<T> = ast::Spanned<T>;

// Parses a single pattern, e.g. `_`, `x`, `0`, or `Node(x, xs)`.
fn pattern_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Pattern, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    recursive(|pattern| {
        let int_pat = select! { Token::Int(n) => Pattern::Int(n) };

        let ident = select! { Token::Ident(name) => name };

        let ctor_pat = ident
            .then(
                pattern
                    .separated_by(just(Token::Ctrl(',')))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')))
                    .or_not(),
            )
            .map(|(name, args)| match args {
                // `Foo` alone: either a nullary constructor or a bound
                // variable. We can't tell without name resolution, so we
                // treat lowercase-first idents as variables and the rest
                // (and `_`) specially.
                None => {
                    if name == "_" {
                        Pattern::Wildcard
                    } else if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                        Pattern::Ctor(name.to_string(), vec![])
                    } else {
                        Pattern::Var(name.to_string())
                    }
                }
                Some(args) => Pattern::Ctor(name.to_string(), args),
            });

        int_pat.or(ctor_pat).labelled("pattern")
    })
}

pub fn expr_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Spanned<Expr>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    recursive(|expr| {
        let ident = select! { Token::Ident(name) => name };

        let int = select! { Token::Int(n) => Expr::Int(n) };

        let let_ = just(Token::Let)
            .ignore_then(ident)
            .then_ignore(just(Token::Op("=")))
            .then(expr.clone())
            .then_ignore(just(Token::In))
            .then(expr.clone())
            .map(|((name, value), body)| Expr::Let {
                name: name.to_string(),
                value: Box::new(value),
                body: Box::new(body),
            });

        let if_ = just(Token::If)
            .ignore_then(expr.clone())
            .then_ignore(just(Token::Then))
            .then(expr.clone())
            .then_ignore(just(Token::Else))
            .then(expr.clone())
            .map(|((cond, then_branch), else_branch)| Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            });

        let match_arm = just(Token::Ctrl('|'))
            .ignore_then(pattern_parser())
            .then_ignore(just(Token::Arrow))
            .then(expr.clone());

        let match_ = just(Token::Match)
            .ignore_then(expr.clone())
            .then(match_arm.repeated().at_least(1).collect::<Vec<_>>())
            .map(|(scrutinee, arms)| Expr::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            });

        let atom = int
            .or(ident.map(|s| Expr::Var(s.to_string())))
            .map_with(|e, extra| (e, extra.span()))
            .or(let_.map_with(|e, extra| (e, extra.span())))
            .or(if_.map_with(|e, extra| (e, extra.span())))
            .or(match_.map_with(|e, extra| (e, extra.span())))
            .or(expr
                .clone()
                .delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')'))))
            .boxed();

        // Function calls / constructor application, e.g. `f(a, b)`.
        let call = atom.foldl_with(
            expr.clone()
                .separated_by(just(Token::Ctrl(',')))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')))
                .repeated(),
            |f, args, e| (Expr::Call(Box::new(f), args), e.span()),
        );

        let product_op = just(Token::Op("*"))
            .to(BinOp::Mul)
            .or(just(Token::Op("/")).to(BinOp::Div));
        let product = call
            .clone()
            .foldl_with(product_op.then(call).repeated(), |a, (op, b), e| {
                (Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
            });

        let sum_op = just(Token::Op("+"))
            .to(BinOp::Add)
            .or(just(Token::Op("-")).to(BinOp::Sub));
        let sum = product
            .clone()
            .foldl_with(sum_op.then(product).repeated(), |a, (op, b), e| {
                (Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
            });

        let cmp_op = just(Token::Op("=="))
            .to(BinOp::Eq)
            .or(just(Token::Op("!=")).to(BinOp::NotEq))
            .or(just(Token::Op("<")).to(BinOp::Lt))
            .or(just(Token::Op(">")).to(BinOp::Gt));
        let compare = sum
            .clone()
            .foldl_with(cmp_op.then(sum).repeated(), |a, (op, b), e| {
                (Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
            });

        compare.labelled("expression")
    })
}

fn type_decl_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, TypeDecl, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    let ident = select! { Token::Ident(name) => name };

    let ctor = ident.then(
        ident
            .separated_by(just(Token::Ctrl(',')))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')))
            .or_not(),
    );

    just(Token::Type)
        .ignore_then(ident)
        .then(ident.repeated().collect::<Vec<_>>())
        .then_ignore(just(Token::Op("=")))
        .then(
            ctor.separated_by(just(Token::Ctrl('|')))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|((name, type_params), ctors)| TypeDecl {
            name: name.to_string(),
            type_params: type_params.into_iter().map(|s| s.to_string()).collect(),
            ctors: ctors
                .into_iter()
                .map(|(name, fields)| CtorDecl {
                    name: name.to_string(),
                    fields: fields
                        .unwrap_or_default()
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                })
                .collect(),
        })
}

fn fun_decl_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, FunDecl, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    let ident = select! { Token::Ident(name) => name };

    let params = ident
        .separated_by(just(Token::Ctrl(',')))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')));

    just(Token::Fun)
        .ignore_then(ident)
        .then(params)
        .then_ignore(just(Token::Op("=")))
        .then(expr_parser())
        .map(|((name, params), body)| FunDecl {
            name: name.to_string(),
            params: params.into_iter().map(|s| s.to_string()).collect(),
            body,
        })
}

pub fn program_parser<'tokens, 'src: 'tokens, I>(
) -> impl Parser<'tokens, I, Program, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    let item = type_decl_parser()
        .map(Item::Type)
        .or(fun_decl_parser().map(Item::Fun));

    item.repeated().collect::<Vec<_>>()
}
