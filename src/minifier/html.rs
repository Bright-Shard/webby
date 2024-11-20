use crate::{line_number_of_offset, minifier, Cow};

macro_rules! log {
    ($($t:tt)*) => {
        #[cfg(any(test, feature = "log"))]
        println!($($t)*);
    };
}

pub fn minify_html(
    source_path: &str,
    source: &str,
    original: &str,
) -> Result<String, Cow<'static>> {
    let mut result = String::new();
    let mut handled_bytes = 0;

    while handled_bytes < source.len() {
        let (tag, bytes) = handle_tag(
            source_path,
            &source[handled_bytes..],
            (original, handled_bytes),
        )?;
        result += &tag;
        handled_bytes += bytes + 1;
    }

    Ok(result)
}

/// Parses an individual HTML tag and minifies it.
///
/// This is a rough overview of the strategy this function uses to parse HTML,
/// handle its edge cases, and then minify it:
///
/// DISCLAIMER: The above is what this minifier will do when it's complete. See
/// the TODO at the bottom for what's not yet implemented.
///
/// 1. Tags begin with a <, then have the tag type. If there is whitespace after
///    the <, it's not considered a tag.
/// 2. If the tag's type is `!--`, it is a comment and will be removed from
///    the resulting HTML.
/// 3. The tag may have properties in the format `name=value`, with optional
///    whitespace around the `=` and optional quotes around the value. The tag
///    may also have properties in the format `name`. Properties are never
///    minimised except to remove whitespace around the `=`.
/// 4. The tag may be closed with either a `/>`, a closing tag, or may not be
///    closed properly at all.
/// 5. If the tag is closed with a closing tag, this function will classify the
///    tag as either a *text* tag or a *content* tag. Text tags store text (`p`,
///    `a`, `h1`, etc), while content tags store other HTMl elements (`head`,
///    `body`, etc). If the tag is not closed, this function just returns the
///    tag. If the tag is a style tag, it will be run through the CSS minifier.
///    Script tags will only be minified with the `js-minify` feature enabled.
/// 6. If the tag is a text tag, the only minification that will occur is
///    removing newlines around tags inside it. Any content tags inside the text
///    tag will be minified as normal for a content tag.
/// 7. If the tag is a content tag, newlines will be stripped from it.
///    Whitespace that isn't in a tag property's value will also be stripped.
///    Any nested tags inside that content tag will be re-run through this
///    minifier.
/// 8. If the tag is a <![CDATA[]]> tag, it will not be minimised. Content
///    should be explicitly minimised with the #!MINIMISE macro.
///
/// # TODO
/// - Find a decent JS minifier, add it as a dep, and feature flag it. JS is
///   too complicated to write a minifier for, when I don't even use it.
fn handle_tag<'a>(
    source_path: &'a str,
    source: &'a str,
    error_meta: (&'a str, usize),
) -> Result<(Cow<'a>, usize), Cow<'static>> {
    if source
        .chars()
        .next()
        .map(|char| char.is_whitespace())
        .unwrap_or(false)
    {
        return Ok((Cow::Borrowed("<"), 1));
    } else if source.starts_with("<!--") {
        let Some(ending) = source.find("-->") else {
            return Err(Cow::Owned(format!(
                "HTML error: Unclosed HTML comment at {source_path}:{}",
                line_number_of_offset(error_meta.0, error_meta.1)
            )));
        };
        return Ok((Cow::Borrowed(""), ending + 2));
    }

    let mut output = String::from("<");
    let mut chars = source.char_indices().peekable();
    chars.next(); // discard opening <

    let tag_name_end;
    let tag_closed;
    loop {
        let Some((byte_idx, char)) = chars.next() else {
            return Ok((Cow::Owned(output), source.len() - 1));
        };

        if char == '/' && chars.peek().map(|(_, char)| *char) == Some('>') {
            output += "/>";
            chars.next();
            let mut end = byte_idx + 1;
            if chars.peek().map(|(_, char)| *char) == Some('\n') {
                while chars
                    .peek()
                    .map(|(_, c)| c.is_whitespace())
                    .unwrap_or(false)
                {
                    end = chars.next().unwrap().0;
                }
            }

            return Ok((Cow::Owned(output), end));
        } else if char == '>' {
            tag_name_end = byte_idx;
            tag_closed = true;
            break;
        } else if char.is_whitespace() {
            tag_name_end = byte_idx;
            tag_closed = false;

            while chars
                .peek()
                .map(|(_, c)| c.is_whitespace())
                .unwrap_or(false)
            {
                chars.next();
            }

            output.push(' ');
            break;
        }

        output.push(char);
    }

    let tag_name = &source[1..tag_name_end];
    log!("Parsing tag `{tag_name}`");

    if tag_name == "![CDATA[" {
        let Some(end) = source[tag_name_end..].find("]]>") else {
            return Err(Cow::Owned(format!(
                "Unclosed CDATA tag at {source_path:?}{}",
                line_number_of_offset(error_meta.0, error_meta.1 + tag_name_end)
            )));
        };
        return Ok((Cow::Borrowed(&source[..end + "]]>".len()]), end));
    }

    if tag_closed {
        log!("  Opening tag closed w/o properties");
        output.push('>');
    } else {
        // Each loop parses 1 property
        'parse_properties: loop {
            let Some((byte_idx, char)) = chars.next() else {
                return Ok((Cow::Owned(output), source.len() - 1));
            };
            log!("  'parse_properties: Found `{char}`");

            if char == '/' && chars.peek().map(|(_, char)| *char) == Some('>') {
                output += "/>";
                chars.next();
                let mut end = byte_idx + 1;
                if chars.peek().map(|(_, char)| *char) == Some('\n') {
                    while chars
                        .peek()
                        .map(|(_, c)| c.is_whitespace())
                        .unwrap_or(false)
                    {
                        end = chars.next().unwrap().0;
                    }
                }

                return Ok((Cow::Owned(output), end));
            } else if char == '>' {
                log!("  Opening tag closed in 'parsed_properties");
                output.push('>');
                break 'parse_properties;
            } else if char == '\n' {
                continue 'parse_properties;
            } else if char.is_whitespace() {
                while chars
                    .peek()
                    .map(|(_, c)| c.is_whitespace())
                    .unwrap_or(false)
                {
                    chars.next();
                }
                output.push(' ');
                continue 'parse_properties;
            }

            // Parse property's name
            output.push(char);
            'parse_property_name: loop {
                let Some((byte_idx, char)) = chars.next() else {
                    return Ok((Cow::Owned(output), source.len() - 1));
                };

                if char == '=' {
                    break 'parse_property_name;
                } else if char == '/' && chars.peek().map(|(_, char)| *char) == Some('>') {
                    output += "/>";
                    chars.next();
                    let mut end = byte_idx + 1;
                    if chars.peek().map(|(_, char)| *char) == Some('\n') {
                        while chars
                            .peek()
                            .map(|(_, c)| c.is_whitespace())
                            .unwrap_or(false)
                        {
                            end = chars.next().unwrap().0;
                        }
                    }

                    return Ok((Cow::Owned(output), end));
                } else if char == '>' {
                    output += ">";
                    break 'parse_properties;
                } else if char.is_whitespace() {
                    while chars
                        .peek()
                        .map(|(_, c)| c.is_whitespace())
                        .unwrap_or(false)
                    {
                        chars.next();
                    }

                    if chars.peek().map(|(_, char)| *char) == Some('=') {
                        // Whitespace followed by an `=` - should break name
                        // parsing and start parsing the value
                        chars.next();
                        break 'parse_property_name;
                    } else {
                        // Whitespace followed by other characters - this
                        // property didn't have a value and we should go parse
                        // the next one
                        output.push(' ');
                        continue 'parse_properties;
                    }
                }

                output.push(char);
            }

            // If we get to this point, the property has a value. The chars
            // iterator will pick up after the =.
            output.push('=');

            while chars
                .peek()
                .map(|(_, c)| c.is_whitespace())
                .unwrap_or(false)
            {
                chars.next();
            }

            // Parse property's value
            let Some((idx, char)) = chars.next() else {
                return Ok((Cow::Owned(output), source.len() - 1));
            };
            match char {
                '\'' | '"' => {
                    output.push(char);
                    loop {
                        let Some((_, next)) = chars.next() else {
                            return Err(Cow::Owned(format!(
                                "Unclosed quotation in HTML property at {source_path}:{}",
                                line_number_of_offset(error_meta.0, error_meta.1 + idx)
                            )));
                        };
                        if next == '\\' {
                            if let Some((_, next)) = chars.next() {
                                output.push(next);
                            }
                            continue;
                        }

                        output.push(next);

                        if next == char {
                            break;
                        }
                    }
                }
                _ => {
                    output.push(char);
                    loop {
                        let Some((idx, char)) = chars.next() else {
                            return Err(Cow::Owned(format!(
                                "Unclosed quotation in HTML property at {source_path}:{}",
                                line_number_of_offset(error_meta.0, error_meta.1 + idx)
                            )));
                        };

                        if char == '/' && chars.peek().map(|(_, char)| *char) == Some('>') {
                            output += "/>";
                            chars.next();
                            let mut end = idx + 1;
                            if chars.peek().map(|(_, char)| *char) == Some('\n') {
                                while chars
                                    .peek()
                                    .map(|(_, c)| c.is_whitespace())
                                    .unwrap_or(false)
                                {
                                    end = chars.next().unwrap().0;
                                }
                            }

                            return Ok((Cow::Owned(output), end));
                        } else if char == '>' {
                            output += ">";
                            break 'parse_properties;
                        } else if char.is_whitespace() {
                            while chars
                                .peek()
                                .map(|(_, c)| c.is_whitespace())
                                .unwrap_or(false)
                            {
                                chars.next();
                            }
                            output.push(char);

                            continue 'parse_properties;
                        } else {
                            output.push(char);
                        }
                    }
                }
            }
        }
    }

    // By now the > of the opening tag has been reached
    // We need to find the closing tag, then minify the contents of the tag
    // as needed
    debug_assert!(output.ends_with('>'), "output is: `{output}`");

    if chars.peek().map(|(_, char)| *char) == Some('\n') {
        while chars
            .peek()
            .map(|(_, char)| char.is_whitespace())
            .unwrap_or(false)
        {
            chars.next();
        }
    }

    if tag_name == "script" || tag_name == "style" {
        // TODO: Actually parse and minify JS
        let closing_tag = if tag_name == "script" {
            "</script>"
        } else {
            "</style>"
        };
        let Some(search_start_idx) = chars.next().map(|(idx, _)| idx) else {
            return Ok((Cow::Owned(output), source.len() - 1));
        };

        let Some(closing) = source[search_start_idx..].find(closing_tag) else {
            return Err(Cow::Owned(format!(
                "Unclosed style or script tag at {source_path}:{}",
                line_number_of_offset(error_meta.0, error_meta.1 + search_start_idx)
            )));
        };
        let closing = search_start_idx + closing + closing_tag.len();

        if tag_name == "style" {
            output += &minifier::minify_css(&source[search_start_idx..closing]);
        } else {
            output += &source[search_start_idx..closing];
        }

        return Ok((Cow::Owned(output), closing));
    }

    let textual_tag = matches!(
        tag_name,
        "a" | "abbr"
            | "acronym"
            | "aside"
            | "b"
            | "bdi"
            | "bdo"
            | "big"
            | "blockquote"
            | "button"
            | "caption"
            | "cite"
            | "code"
            | "dd"
            | "del"
            | "details"
            | "dfn"
            | "dt"
            | "em"
            | "figcaption"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "i"
            | "ins"
            | "kbd"
            | "label"
            | "legend"
            | "li"
            | "mark"
            | "marquee"
            | "meter"
            | "nobr"
            | "option"
            | "output"
            | "p"
            | "pre"
            | "progress"
            | "q"
            | "rb"
            | "rp"
            | "rt"
            | "s"
            | "sample"
            | "small"
            | "span"
            | "strong"
            | "sub"
            | "summary"
            | "sup"
            | "td"
            | "textarea"
            | "th"
            | "time"
            | "title"
            | "u"
            | "var"
    );
    let preformatted = tag_name == "pre";
    log!("  Textual tag? {textual_tag}");

    while let Some((byte_idx, char)) = chars.next() {
        if char == '<' {
            let Some((next_idx, next_char)) = chars.peek().copied() else {
                return Ok((Cow::Owned(output), source.len() - 1));
            };

            if next_char == '/' {
                chars.next();

                if !preformatted {
                    let mut trim = false;
                    let mut chars_rev = source[..byte_idx].chars();
                    while let Some(char) = chars_rev.next_back() {
                        if !char.is_whitespace() {
                            break;
                        } else if char == '\n' {
                            trim = true;
                        }
                    }
                    if trim {
                        output = output.trim_end().to_string();
                    }
                }

                output += "</";

                loop {
                    let Some((idx, char)) = chars.next() else {
                        return Err(Cow::Owned(format!(
                            "Unclosed HTML closing tag at {source_path}:{}",
                            line_number_of_offset(error_meta.0, error_meta.1 + next_idx)
                        )));
                    };

                    if char == '>' {
                        let mut end = idx;
                        if !preformatted && chars.peek().map(|(_, char)| *char) == Some('\n') {
                            while chars
                                .peek()
                                .map(|(_, c)| c.is_whitespace())
                                .unwrap_or(false)
                            {
                                end = chars.next().unwrap().0;
                            }
                        }

                        if output.ends_with(tag_name) {
                            output.push('>');
                            return Ok((Cow::Owned(output), end));
                        } else {
                            output.push('>');
                            break;
                        }
                    }

                    if !char.is_whitespace() {
                        output.push(char);
                    }
                }
            } else if !next_char.is_whitespace() {
                let (subtag, used) = handle_tag(source_path, &source[byte_idx..], error_meta)?;
                log!("  Found subtag `{subtag}`. Ends at {used}, current char is {next_idx}.");
                let used = used + byte_idx;

                loop {
                    let (next_idx, _) = chars.next().unwrap();
                    if next_idx == used {
                        break;
                    }
                }

                output += &subtag;
            } else {
                output.push('<');
            }
        } else if !preformatted {
            match char {
                '\n' => {}
                _ if char.is_whitespace() => {
                    while chars
                        .peek()
                        .map(|(_, char)| char.is_whitespace())
                        .unwrap_or(false)
                    {
                        chars.next();
                    }
                    if textual_tag {
                        output.push(' ');
                    }
                }
                _ => output.push(char),
            }
        } else {
            output.push(char);
        }
    }

    Ok((Cow::Owned(output), source.len() - 1))
}

#[cfg(test)]
mod tests {
    use crate::minifier::minify_html;

    struct Tester {
        name: &'static str,
        source: &'static str,
        expected: &'static str,
    }
    impl Tester {
        fn test(self) {
            log!("\nSTARTING TEST '{}'", self.name);
            let result = minify_html("test/path", self.source, self.source).unwrap();
            assert_eq!(&result, self.expected, "Test name: {}", self.name);
        }
    }

    #[test]
    fn test() {
        let cases = [
            Tester {
                name: "Trim whitespace between tags",
                source: "<body>    <p>hi</p></body>",
                expected: "<body><p>hi</p></body>",
            },
            Tester {
                name: "Trim comments",
                source: "<body><!--commentcomment--><p>hi</p></body>",
                expected: "<body><p>hi</p></body>",
            },
            Tester {
                name: "Minimises whitespace in textual tags",
                source: "<p> This has   weird whitespace!!!\n</p>",
                expected: "<p> This has weird whitespace!!!</p>",
            },
            Tester {
                name: "Includes whitespace in preformatted tags",
                source: "<pre> This has   weird whitespace!!!\n</pre>",
                expected: "<pre> This has   weird whitespace!!!\n</pre>",
            },
            Tester {
                name: "Element properties",
                source: "<p string1='string1 string2' string2=\"string1 string2\" number=1 singleword=hi eek>hewwo</p>",
                expected: "<p string1='string1 string2' string2=\"string1 string2\" number=1 singleword=hi eek>hewwo</p>",
            },
            Tester {
                name: "Element properties 2",
                source: "<p string='string1 string2n\'t'>hewwo</p>",
                expected: "<p string='string1 string2n\'t'>hewwo</p>",
            },
            Tester {
                name: "Unclosed Elements",
                source: "<body>  <br/><img src='https://example.com/img.png'><p>hello</p>\n<br/></body>",
                expected: "<body><br/><img src='https://example.com/img.png'><p>hello</p><br/></body>"
            }
        ];

        for case in cases {
            case.test();
        }
    }
}
