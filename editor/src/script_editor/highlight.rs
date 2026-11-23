//! The colors of StarScript code: each piece of the source gets a
//! [`Highlight`] from the compiler's own lexer, which keeps going after
//! mistakes, so code being written is colored too.

use std::ops::Range;

use eframe::egui;
use star_script::semantic::BASIC_TYPE_NAMES;
use star_script::{Keyword, LexErrorKind, Span, Token, TokenKind, scan};

/// What a piece of code is, for its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Highlight {
    /// Names of variables, symbols and spaces.
    Plain,
    Keyword,
    /// `true`, `false` and `null`.
    Literal,
    /// Class names and basic types.
    Type,
    /// A name that is called.
    Function,
    Number,
    String,
    Comment,
    /// Characters the language does not have.
    Mistake,
}

/// The colors of each [`Highlight`], for dark and light themes.
struct Palette {
    plain: egui::Color32,
    keyword: egui::Color32,
    literal: egui::Color32,
    ty: egui::Color32,
    function: egui::Color32,
    number: egui::Color32,
    string: egui::Color32,
    comment: egui::Color32,
    mistake: egui::Color32,
}

const DARK: Palette = Palette {
    plain: egui::Color32::from_rgb(212, 212, 212),
    keyword: egui::Color32::from_rgb(255, 120, 150),
    literal: egui::Color32::from_rgb(255, 160, 110),
    ty: egui::Color32::from_rgb(120, 220, 190),
    function: egui::Color32::from_rgb(110, 180, 255),
    number: egui::Color32::from_rgb(180, 230, 140),
    string: egui::Color32::from_rgb(240, 210, 130),
    comment: egui::Color32::from_rgb(120, 130, 140),
    mistake: egui::Color32::from_rgb(255, 90, 80),
};

const LIGHT: Palette = Palette {
    plain: egui::Color32::from_rgb(40, 40, 40),
    keyword: egui::Color32::from_rgb(190, 30, 90),
    literal: egui::Color32::from_rgb(200, 90, 20),
    ty: egui::Color32::from_rgb(20, 130, 110),
    function: egui::Color32::from_rgb(30, 90, 200),
    number: egui::Color32::from_rgb(60, 130, 30),
    string: egui::Color32::from_rgb(160, 110, 0),
    comment: egui::Color32::from_rgb(120, 125, 130),
    mistake: egui::Color32::from_rgb(210, 30, 30),
};

impl Palette {
    const fn of(dark_mode: bool) -> &'static Self {
        if dark_mode { &DARK } else { &LIGHT }
    }

    const fn color(&self, highlight: Highlight) -> egui::Color32 {
        match highlight {
            Highlight::Plain => self.plain,
            Highlight::Keyword => self.keyword,
            Highlight::Literal => self.literal,
            Highlight::Type => self.ty,
            Highlight::Function => self.function,
            Highlight::Number => self.number,
            Highlight::String => self.string,
            Highlight::Comment => self.comment,
            Highlight::Mistake => self.mistake,
        }
    }
}

/// `source` laid out in `font`, colored for a dark or a light theme.
pub(super) fn colored(source: &str, font: &egui::FontId, dark_mode: bool) -> egui::text::LayoutJob {
    let palette = Palette::of(dark_mode);
    let mut job = egui::text::LayoutJob::default();
    for (range, highlight) in pieces(source) {
        let format = egui::TextFormat::simple(font.clone(), palette.color(highlight));
        job.append(&source[range], 0.0, format);
    }
    job
}

/// The pieces of `source`, in order and covering all of it, each with how
/// it is colored.
pub(super) fn pieces(source: &str) -> Vec<(Range<usize>, Highlight)> {
    let mut marked = marked_spans(source);
    marked.sort_by_key(|(span, _)| span.start());
    let mut pieces = Vec::with_capacity(marked.len() * 2 + 1);
    let mut covered = 0;
    for (span, highlight) in marked {
        // A mistake inside a string or comment keeps their color.
        if span.start() < covered || span.end() > source.len() {
            continue;
        }
        if span.start() > covered {
            pieces.push((covered..span.start(), Highlight::Plain));
        }
        pieces.push((span.start()..span.end(), highlight));
        covered = span.end();
    }
    if covered < source.len() {
        pieces.push((covered..source.len(), Highlight::Plain));
    }
    pieces
}

/// Every span of `source` with a color other than the plain one.
fn marked_spans(source: &str) -> Vec<(Span, Highlight)> {
    let scan = scan(source);
    let mut marked: Vec<(Span, Highlight)> = scan
        .tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            let next = scan.tokens.get(index + 1);
            highlight_of(token, next).map(|highlight| (token.span, highlight))
        })
        .collect();
    marked.extend(scan.comments.iter().map(|span| (*span, Highlight::Comment)));
    marked.extend(
        scan.errors
            .iter()
            .filter(|error| !is_covered_mistake(error.kind))
            .map(|error| (error.span, Highlight::Mistake)),
    );
    marked
}

/// Mistakes that already have a token or comment of their own.
const fn is_covered_mistake(kind: LexErrorKind) -> bool {
    !matches!(
        kind,
        LexErrorKind::UnexpectedCharacter(_) | LexErrorKind::NonAsciiIdentifier(_)
    )
}

/// How `token`, followed by `next`, is colored, if not plainly.
fn highlight_of(token: &Token, next: Option<&Token>) -> Option<Highlight> {
    let highlight = match &token.kind {
        TokenKind::Keyword(Keyword::True | Keyword::False | Keyword::Null) => Highlight::Literal,
        TokenKind::Keyword(_) => Highlight::Keyword,
        TokenKind::Integer(_) | TokenKind::Float(_) => Highlight::Number,
        TokenKind::String(_) => Highlight::String,
        TokenKind::Identifier(name) => highlight_of_name(name, next)?,
        _ => return None,
    };
    Some(highlight)
}

fn highlight_of_name(name: &str, next: Option<&Token>) -> Option<Highlight> {
    if next.is_some_and(|next| next.kind == TokenKind::LeftParen) {
        Some(Highlight::Function)
    } else if name.starts_with(char::is_uppercase) || BASIC_TYPE_NAMES.contains(&name) {
        Some(Highlight::Type)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colored_words(source: &str) -> Vec<(&str, Highlight)> {
        pieces(source)
            .into_iter()
            .map(|(range, highlight)| (source[range].trim(), highlight))
            .filter(|(text, _)| !text.is_empty())
            .collect()
    }

    #[test]
    fn pieces_cover_the_whole_source_in_order() {
        let source = "class Rock extends Node {\n  var hits: int = 3 // so far\n}\n";

        let pieces = pieces(source);

        let mut end = 0;
        for (range, _) in &pieces {
            assert_eq!(range.start, end);
            end = range.end;
        }
        assert_eq!(end, source.len());
    }

    #[test]
    fn each_kind_of_code_has_its_color() {
        let words = colored_words("var speed: float = clamp(1.5, \"x\") ?? null // why");

        assert_eq!(
            words,
            [
                ("var", Highlight::Keyword),
                ("speed:", Highlight::Plain),
                ("float", Highlight::Type),
                ("=", Highlight::Plain),
                ("clamp", Highlight::Function),
                ("(", Highlight::Plain),
                ("1.5", Highlight::Number),
                (",", Highlight::Plain),
                ("\"x\"", Highlight::String),
                (") ??", Highlight::Plain),
                ("null", Highlight::Literal),
                ("// why", Highlight::Comment),
            ]
        );
    }

    #[test]
    fn code_being_written_is_still_colored() {
        let words = colored_words("Player @ \"not closed");

        assert_eq!(
            words,
            [
                ("Player", Highlight::Type),
                ("@", Highlight::Mistake),
                ("\"not closed", Highlight::String),
            ]
        );
    }

    #[test]
    fn mistakes_inside_strings_keep_the_string_color() {
        let words = colored_words("\"bad \\q escape\"");

        assert_eq!(words, [("\"bad \\q escape\"", Highlight::String)]);
    }
}
