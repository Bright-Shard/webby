use {
    crate::Cow,
    std::{fmt::Write, path::Path},
};

#[derive(PartialEq, Eq)]
enum ParserState {
    Text,
    List,
    Preformatted,
}

/// Escapes characters from an input string so valid Gemtext doesn't get
/// misinterpreted as HTML.
//
// This should prevent any form of HTML injection... but other programs filter
// more characters than are being filtered here, which should be looked into...
//
// Cases covered by Canvas LMS:
//     '&' => *out += "&amp;",
//     '<' => *out += "&lt;",
//     '>' => *out += "&gt;",
//     '"' => *out += "&quot;",
//     '\'' => *out += "&#x27;",
//     '/' => *out += "&#x2F;",
//     '`' => *out += "&#x60;",
//     '=' => *out += "&#x3D;",
// From https://github.com/instructure/canvas-lms/blob/master/packages/html-escape/index.js#L85
fn html_escape_into(input: &str, out: &mut String) {
    for char in input.chars() {
        match char {
            '<' => *out += "&lt;",
            '>' => *out += "&gt;",
            '"' => *out += "&quot;",
            '&' => *out += "&amp;",
            other => out.push(other),
        }
    }
}

pub fn translate_gemtext(source_path: &Path, source: &str) -> Result<String, Cow<'static>> {
    let mut output = String::new();
    let mut state = ParserState::Text;
    output += "<p>";

    for (line_num, line) in source.lines().enumerate() {
        if state == ParserState::Preformatted {
            if line.starts_with("```") {
                state = ParserState::Text;
                output += "</pre>";
                continue;
            }

            html_escape_into(line, &mut output);
            continue;
        }

        if let Some(list_line) = line.strip_prefix("* ") {
            if state != ParserState::List {
                state = ParserState::List;
                output += "<ul>";
            }
            output += "<li>";
            html_escape_into(list_line, &mut output);
            output += "</li>";
            continue;
        } else if state == ParserState::List {
            state = ParserState::Text;
            output += "</ul>";
        }

        if let Some(link_line) = line.strip_prefix("=>") {
            let mut line = link_line.split_whitespace();
            let link = line.next().ok_or(Cow::Owned(format!(
                "Expected URL in link at {source_path:?}:{line_num}"
            )))?;

            output += "<a href=\"";
            html_escape_into(link, &mut output);
            output += "\">";

            if let Some(link_text) = line.next() {
                html_escape_into(link_text, &mut output);
            } else {
                html_escape_into(link, &mut output);
            }

            output += "</a><br>";
        } else if let Some(alt) = line.strip_prefix("```") {
            output += "<pre alt=\"";
            html_escape_into(alt, &mut output);
            output += "\">";
            state = ParserState::Preformatted;
        } else if let Some(quote) = line.strip_prefix("> ") {
            output += "<blockquote><p>";
            html_escape_into(quote, &mut output);
            output += "</p></blockquote>";
        } else if line.starts_with('#') {
            let mut chars = line.bytes();
            let mut level = 0;
            while chars.next() == Some(b'#') {
                level += 1;
            }

            write!(output, "<h{level}>").unwrap();
            html_escape_into(line[level..].trim_start(), &mut output);
            write!(output, "</h{level}>").unwrap();
        } else {
            output += line;
        }
    }

    if state == ParserState::List {
        output += "</ul>";
    }

    output += "</p>";
    Ok(output)
}
