use crate::extractor::RawRoute;

pub fn extract_routes_from_routes_php(source: &str) -> Vec<RawRoute> {
    let mut routes = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let mut cursor = 0usize;
        while let Some(pos) = line[cursor..].find("Route::") {
            let start = cursor + pos;
            let rest = &line[start + "Route::".len()..];

            let Some(paren) = rest.find('(') else {
                cursor = start + 1;
                continue;
            };
            let method = rest[..paren].trim();
            if method.is_empty() {
                cursor = start + 1;
                continue;
            }

            let args = &rest[paren + 1..];
            let Some((path, after_path)) = parse_php_string(args) else {
                cursor = start + 1;
                continue;
            };

            let after_path = skip_separators(after_path);
            if !after_path.starts_with(',') {
                cursor = start + 1;
                continue;
            }
            let after_comma = skip_separators(&after_path[1..]);

            let Some((handler, _after_handler)) = parse_php_string(after_comma) else {
                cursor = start + 1;
                continue;
            };

            routes.push(RawRoute {
                method: method.to_ascii_uppercase(),
                path,
                handler_name: handler,
                line: line_idx as u32 + 1,
                col: start as u32,
            });

            cursor = start + 1;
        }
    }

    routes
}

fn skip_separators(mut s: &str) -> &str {
    loop {
        let trimmed = s.trim_start_matches([' ', '\t']);
        if trimmed.len() == s.len() {
            return s;
        }
        s = trimmed;
    }
}

fn parse_php_string(input: &str) -> Option<(String, &str)> {
    let s = input.trim_start();
    let quote = s.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for (i, ch) in s[quote.len_utf8()..].char_indices() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            let after = &s[quote.len_utf8() + i + ch.len_utf8()..];
            return Some((out, after));
        }
        out.push(ch);
    }
    None
}
