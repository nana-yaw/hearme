//! Deterministic spoken-punctuation commands: "period" → ".", "new line" →
//! a line break, with article guards so "the period" stays literal. Runs as a
//! pure text pass — zero latency, no LLM — after custom-word correction.

/// Words that make the following command literal: "my period", "a comma",
/// "the question mark" are content, not commands.
const GUARD_WORDS: &[&str] = &[
    "a", "an", "the", "my", "your", "his", "her", "its", "our", "their", "this", "that", "each",
    "every", "one", "per", "first", "second", "last", "next", "previous",
];

/// Punctuation that attaches to the end of the previous word.
fn attaching_symbol(command: &str) -> Option<&'static str> {
    match command {
        "period" | "fullstop" => Some("."),
        "comma" => Some(","),
        "colon" => Some(":"),
        "semicolon" => Some(";"),
        "hyphen" | "dash" => Some("-"),
        "ellipsis" => Some("..."),
        _ => None,
    }
}

/// Two-word commands. Returns (symbol, attaches_to_previous, starts_new_sentence).
fn two_word_symbol(first: &str, second: &str) -> Option<(&'static str, bool, bool)> {
    match (first, second) {
        ("question", "mark") => Some(("?", true, true)),
        ("exclamation", "mark") | ("exclamation", "point") => Some(("!", true, true)),
        ("new", "line") => Some(("\n", true, true)),
        ("new", "paragraph") => Some(("\n\n", true, true)),
        ("open", "quote") => Some(("\"", false, false)),
        ("close", "quote") => Some(("\"", true, false)),
        ("open", "paren") | ("open", "parenthesis") | ("open", "bracket") => {
            Some(("(", false, false))
        }
        ("close", "paren") | ("close", "parenthesis") | ("close", "bracket") => {
            Some((")", true, false))
        }
        _ => None,
    }
}

/// Lowercased word with surrounding punctuation stripped, for command matching
/// ("Period." still matches "period").
fn normalized(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Replace spoken punctuation commands in `text`. Whitespace is normalized to
/// single spaces (matching ASR output); line breaks from "new line"/"new
/// paragraph" are preserved and the following word is capitalized, as it is
/// after any sentence-ending symbol.
pub fn apply_spoken_punctuation(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut capitalize_next = false;
    let mut pending_prefix: Option<&'static str> = None;
    let mut i = 0;

    while i < words.len() {
        let guard = i > 0 && GUARD_WORDS.contains(&normalized(words[i - 1]).as_str());
        let current = normalized(words[i]);

        // Two-word commands first ("question mark" before "question").
        let two = if !guard && i + 1 < words.len() {
            two_word_symbol(&current, &normalized(words[i + 1]))
        } else {
            None
        };
        if let Some((symbol, attaches, sentence_end)) = two {
            if attaches {
                match out.last_mut() {
                    Some(last) => last.push_str(symbol),
                    None => out.push(symbol.to_string()),
                }
            } else {
                pending_prefix = Some(symbol);
            }
            capitalize_next = capitalize_next || sentence_end;
            i += 2;
            continue;
        }

        if !guard {
            if let Some(symbol) = attaching_symbol(&current) {
                match out.last_mut() {
                    Some(last) => last.push_str(symbol),
                    None => out.push(symbol.to_string()),
                }
                if symbol == "." {
                    capitalize_next = true;
                }
                i += 1;
                continue;
            }
        }

        // Literal word.
        let mut word = words[i].to_string();
        if capitalize_next {
            word = capitalize_first(&word);
            capitalize_next = false;
        }
        if let Some(prefix) = pending_prefix.take() {
            word = format!("{}{}", prefix, word);
        }
        out.push(word);
        i += 1;
    }

    // Line breaks swallow the following join-space.
    out.join(" ").replace("\n ", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_sentence_enders_attach_and_capitalize() {
        assert_eq!(
            apply_spoken_punctuation("send it tomorrow period thanks for waiting"),
            "send it tomorrow. Thanks for waiting"
        );
        assert_eq!(
            apply_spoken_punctuation("are you coming question mark yes exclamation mark"),
            "are you coming? Yes!"
        );
    }

    #[test]
    fn article_guard_keeps_content_literal() {
        assert_eq!(
            apply_spoken_punctuation("the period was difficult"),
            "the period was difficult"
        );
        assert_eq!(
            apply_spoken_punctuation("add a comma here comma then continue"),
            "add a comma here, then continue"
        );
        assert_eq!(
            apply_spoken_punctuation("my question mark stays literal"),
            "my question mark stays literal"
        );
    }

    #[test]
    fn line_breaks_and_paragraphs() {
        assert_eq!(
            apply_spoken_punctuation("first item new line second item"),
            "first item\nSecond item"
        );
        assert_eq!(
            apply_spoken_punctuation("greetings new paragraph the details follow"),
            "greetings\n\nThe details follow"
        );
    }

    #[test]
    fn quotes_and_parens_wrap_correctly() {
        assert_eq!(
            apply_spoken_punctuation("she said open quote hello close quote softly"),
            "she said \"hello\" softly"
        );
        assert_eq!(
            apply_spoken_punctuation("the total open paren before tax close paren is fine"),
            "the total (before tax) is fine"
        );
    }

    #[test]
    fn asr_trailing_punctuation_on_command_still_matches() {
        assert_eq!(
            apply_spoken_punctuation("done for today Period. see you"),
            "done for today. See you"
        );
    }

    #[test]
    fn text_without_commands_passes_through() {
        assert_eq!(
            apply_spoken_punctuation("nothing to replace in this sentence"),
            "nothing to replace in this sentence"
        );
        assert_eq!(apply_spoken_punctuation(""), "");
    }

    #[test]
    fn command_as_first_word_does_not_panic() {
        assert_eq!(apply_spoken_punctuation("period"), ".");
        assert_eq!(apply_spoken_punctuation("comma then text"), ", then text");
    }
}
