pub fn minify_css(source: &str) -> String {
    let mut out = String::new();
    let mut chars = source.chars().peekable();

    let mut function_depth = 0;
    let mut maybe_in_rule = false;

    while let Some(char) = chars.next() {
        match char {
            '/' if chars.peek().copied() == Some('*') => {
                chars.next();

                while let Some(char) = chars.next() {
                    if char == '*' && chars.peek().copied() == Some('/') {
                        chars.next();
                        break;
                    }
                }
                continue;
            }
            '\'' | '"' => {
                out.push(char);

                while let Some(subchar) = chars.next() {
                    if subchar == char {
                        out.push(char);
                        break;
                    } else if subchar == '\\' {
                        if let Some(char) = chars.next() {
                            out.push(char);
                        }
                    }

                    out.push(subchar);
                }

                continue;
            }
            '\n' => {
                while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                    chars.next();
                }

                if function_depth > 0 || maybe_in_rule {
                    out.push(' ');
                }
                continue;
            }
            '(' => {
                function_depth += 1;
                out.truncate(out.trim_end().len());
                while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                    chars.next();
                }
            }
            ')' => {
                function_depth -= 1;
                out.truncate(out.trim_end().len());
            }
            '{' | '}' | ',' => {
                out.truncate(out.trim_end().len());
                while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                    chars.next();
                }
            }
            ':' => {
                while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                    chars.next();
                }
                maybe_in_rule = true;
            }
            ';' => maybe_in_rule = false,
            _ => {}
        }
        out.push(char);
    }

    out
}
