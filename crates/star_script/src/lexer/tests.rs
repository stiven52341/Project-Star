use super::*;
use TokenKind::{
    DotDot, EndOfFile, Equal, Float, Identifier, Integer, LeftBrace, LeftParen, Newline,
    RightBrace, RightParen,
};

fn kinds(source: &str) -> Vec<TokenKind> {
    tokenize(source)
        .expect("source has no lexical errors")
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

fn error_kinds(source: &str) -> Vec<LexErrorKind> {
    tokenize(source)
        .expect_err("source has lexical errors")
        .into_iter()
        .map(|error| error.kind)
        .collect()
}

fn identifier(name: &str) -> TokenKind {
    Identifier(name.to_owned())
}

fn keyword(keyword: Keyword) -> TokenKind {
    TokenKind::Keyword(keyword)
}

#[test]
fn empty_source_only_has_end_of_file() {
    assert_eq!(kinds(""), [EndOfFile]);
}

#[test]
fn words_are_keywords_or_identifiers() {
    assert_eq!(
        kinds("class Player extends Sprite_2"),
        [
            keyword(Keyword::Class),
            identifier("Player"),
            keyword(Keyword::Extends),
            identifier("Sprite_2"),
            EndOfFile,
        ]
    );
}

#[test]
fn reserved_keywords_cannot_be_names() {
    assert_eq!(kinds("match"), [keyword(Keyword::Match), EndOfFile]);
}

#[test]
fn basic_type_names_are_identifiers() {
    assert_eq!(kinds("int"), [identifier("int"), EndOfFile]);
}

#[test]
fn longest_operator_wins() {
    use TokenKind::*;

    assert_eq!(
        kinds("+ += - -= -> * *= / /= % %= = == ! != < <= > >= && || . .. ? ?. ??"),
        [
            Plus,
            PlusEqual,
            Minus,
            MinusEqual,
            Arrow,
            Star,
            StarEqual,
            Slash,
            SlashEqual,
            Percent,
            PercentEqual,
            Equal,
            EqualEqual,
            Bang,
            BangEqual,
            Less,
            LessEqual,
            Greater,
            GreaterEqual,
            AndAnd,
            OrOr,
            Dot,
            DotDot,
            Question,
            QuestionDot,
            QuestionQuestion,
            EndOfFile,
        ]
    );
}

#[test]
fn operators_need_no_spaces() {
    assert_eq!(
        kinds("x=-1"),
        [
            identifier("x"),
            Equal,
            TokenKind::Minus,
            Integer(1),
            EndOfFile
        ]
    );
}

#[test]
fn integers_in_every_base() {
    assert_eq!(
        kinds("42 1_000_000 0xFF 0b1010"),
        [
            Integer(42),
            Integer(1_000_000),
            Integer(255),
            Integer(10),
            EndOfFile
        ]
    );
}

#[test]
fn decimal_numbers() {
    assert_eq!(
        kinds("3.25 1e3 2.5e-3 4E+2"),
        [
            Float(3.25),
            Float(1000.0),
            Float(0.0025),
            Float(400.0),
            EndOfFile
        ]
    );
}

#[test]
fn ranges_are_not_decimal_numbers() {
    assert_eq!(kinds("0..10"), [Integer(0), DotDot, Integer(10), EndOfFile]);
}

#[test]
fn methods_can_be_called_on_integers() {
    assert_eq!(
        kinds("3.abs"),
        [Integer(3), TokenKind::Dot, identifier("abs"), EndOfFile]
    );
}

#[test]
fn malformed_numbers_are_reported() {
    assert_eq!(
        error_kinds("0x 1e 10px 0b12 99999999999999999999 1e999"),
        [
            LexErrorKind::MissingDigits,
            LexErrorKind::MissingDigits,
            LexErrorKind::InvalidCharacterInNumber('p'),
            LexErrorKind::InvalidCharacterInNumber('2'),
            LexErrorKind::IntegerTooLarge,
            LexErrorKind::FloatTooLarge,
        ]
    );
}

#[test]
fn strings_decode_their_escapes() {
    assert_eq!(
        kinds(r#""línea\n\t\"cita\" \\ \u{1F680}""#),
        [
            TokenKind::String("línea\n\t\"cita\" \\ \u{1F680}".to_owned()),
            EndOfFile
        ]
    );
}

#[test]
fn string_mistakes_are_reported() {
    assert_eq!(
        error_kinds(r#""\q" "\u{110000}" "\u1" "abierta"#),
        [
            LexErrorKind::InvalidEscape('q'),
            LexErrorKind::InvalidUnicodeEscape,
            LexErrorKind::InvalidUnicodeEscape,
            LexErrorKind::UnterminatedString,
        ]
    );
}

#[test]
fn strings_cannot_span_lines() {
    assert_eq!(
        error_kinds("\"uno\ndos\""),
        [
            LexErrorKind::UnterminatedString,
            LexErrorKind::UnterminatedString
        ]
    );
}

#[test]
fn line_breaks_end_statements_once() {
    assert_eq!(
        kinds("\n\na\n\n\nb\n"),
        [
            identifier("a"),
            Newline,
            identifier("b"),
            Newline,
            EndOfFile
        ]
    );
}

#[test]
fn line_breaks_inside_parentheses_are_ignored() {
    assert_eq!(
        kinds("f(\n1\n)"),
        [
            identifier("f"),
            LeftParen,
            Integer(1),
            RightParen,
            EndOfFile
        ]
    );
}

#[test]
fn line_breaks_count_again_inside_braces_within_parentheses() {
    assert_eq!(
        kinds("f({\na\n})"),
        [
            identifier("f"),
            LeftParen,
            LeftBrace,
            Newline,
            identifier("a"),
            Newline,
            RightBrace,
            RightParen,
            EndOfFile,
        ]
    );
}

#[test]
fn windows_line_endings_work() {
    assert_eq!(
        kinds("a\r\nb"),
        [identifier("a"), Newline, identifier("b"), EndOfFile]
    );
}

#[test]
fn comments_are_skipped() {
    assert_eq!(
        kinds("a // hello\nb /* inline */ c"),
        [
            identifier("a"),
            Newline,
            identifier("b"),
            identifier("c"),
            EndOfFile
        ]
    );
}

#[test]
fn block_comments_nest() {
    assert_eq!(
        kinds("a /* outer /* inner */ still outer */ b"),
        [identifier("a"), identifier("b"), EndOfFile]
    );
}

#[test]
fn multiline_block_comment_ends_the_statement() {
    assert_eq!(
        kinds("a /* one\ntwo */ b"),
        [identifier("a"), Newline, identifier("b"), EndOfFile]
    );
}

#[test]
fn unterminated_block_comment_is_reported() {
    assert_eq!(
        error_kinds("a /* /* */"),
        [LexErrorKind::UnterminatedBlockComment]
    );
}

#[test]
fn unknown_characters_are_reported() {
    assert_eq!(
        error_kinds("a @ b & c | d"),
        [
            LexErrorKind::UnexpectedCharacter('@'),
            LexErrorKind::UnexpectedCharacter('&'),
            LexErrorKind::UnexpectedCharacter('|'),
        ]
    );
}

#[test]
fn non_ascii_names_get_a_specific_error() {
    assert_eq!(
        error_kinds("var año = 1"),
        [LexErrorKind::NonAsciiIdentifier('ñ')]
    );
}

#[test]
fn all_errors_are_reported_together() {
    let errors = tokenize("var a = @\nvar b = \"x\nvar c = 0x").expect_err("has errors");

    assert_eq!(errors.len(), 3);
}

#[test]
fn byte_order_mark_is_ignored() {
    assert_eq!(kinds("\u{FEFF}a"), [identifier("a"), EndOfFile]);
}

#[test]
fn spans_point_at_the_token_text() {
    let source = "var speed = 2.5";
    let tokens = tokenize(source).expect("valid");

    let texts: Vec<&str> = tokens.iter().map(|token| token.span.text(source)).collect();

    assert_eq!(texts, ["var", "speed", "=", "2.5", ""]);
}

#[test]
fn error_spans_point_at_the_mistake() {
    let source = "var x = 10px";
    let errors = tokenize(source).expect_err("has errors");

    assert_eq!(errors[0].span.text(source), "10px");
}

#[test]
fn example_script_has_no_lexical_errors() {
    let tokens = tokenize(include_str!("../../examples/scripts/player.star"))
        .expect("example script is valid");

    assert!(tokens.len() > 50);
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == keyword(Keyword::Override))
    );
}

#[test]
fn unmatched_closing_brackets_do_not_break_line_handling() {
    assert_eq!(
        kinds(")\na"),
        [RightParen, Newline, identifier("a"), EndOfFile]
    );
    assert_eq!(
        kinds("{ ( }\na"),
        [LeftBrace, LeftParen, RightBrace, identifier("a"), EndOfFile]
    );
}

#[test]
fn scans_note_where_comments_are() {
    let source = "a // one\n/* two\n */ b";

    let found = scan(source);

    let comments: Vec<&str> = found
        .comments
        .iter()
        .map(|span| span.text(source))
        .collect();
    assert_eq!(comments, ["// one", "/* two\n */"]);
    assert_eq!(found.errors, []);
}

#[test]
fn scans_keep_going_after_mistakes() {
    let source = "\"open\n@ 0x";

    let found = scan(source);

    let tokens: Vec<(TokenKind, &str)> = found
        .tokens
        .iter()
        .map(|token| (token.kind.clone(), token.span.text(source)))
        .collect();
    assert_eq!(
        tokens,
        [
            (TokenKind::String(String::new()), "\"open"),
            (Newline, "\n"),
            (Integer(0), "0x"),
            (EndOfFile, ""),
        ]
    );
    assert_eq!(found.errors.len(), 3);
}

#[test]
fn unterminated_comments_are_still_comments() {
    let found = scan("a /* never closed");

    assert_eq!(found.comments.len(), 1);
    assert_eq!(found.errors[0].kind, LexErrorKind::UnterminatedBlockComment);
}
