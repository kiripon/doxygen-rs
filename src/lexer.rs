#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum LexItem {
    At(String),
    Paren(char),
    Word(String),
    Url(String),
    Space,
    NewLine,
}

pub(crate) fn lex(input: String) -> Vec<LexItem> {
    let mut result = vec![];

    let mut remains = input.as_str();
    loop {
        let Some(c) = remains.chars().next() else {
            break;
        };
        remains = &remains[c.len_utf8()..];
        match c {
            '@' => {
                result.push(LexItem::At(c.into()));
            }
            '\\' => {
                if let Some(value) = result.last_mut() {
                    match value {
                        LexItem::At(v) => {
                            if v == "\\" {
                                *v += "\\"
                            } else {
                                result.push(LexItem::At(c.into()))
                            }
                        }
                        _ => result.push(LexItem::At(c.into())),
                    }
                } else {
                    result.push(LexItem::At(c.into()));
                }
            }
            '{' | '}' => {
                result.push(LexItem::Paren(c));
            }
            ' ' => {
                if let Some(v) = result.last_mut() {
                    if !matches!(v, LexItem::Space) {
                        result.push(LexItem::Space);
                    }
                }
            }
            '\n' => {
                result.push(LexItem::NewLine);
            }
            '<' => {
                let html_pattern = regex::Regex::new("(/?[a-zA-Z]+)>").unwrap();
                if let Some(captures) = html_pattern.captures(remains) {
                    let s = &captures[1];
                    match s {
                        "br" => {
                            result.push(LexItem::Word(["<br>"].concat()));
                        }
                        _ => {
                            // otherwise, all tags are escaped
                            result.push(LexItem::Word(["\\<", s, "\\>"].concat()))
                        }
                    }
                    remains = &remains[captures[0].len() - 1..];
                } else {
                    result.push(LexItem::Word("<".into()))
                }
            }
            'h' if remains.starts_with("ttp://") || remains.starts_with("ttps://") => {
                let len = consume_url_chars(remains);
                let str = &remains[..len];
                remains = &remains[len..];
                result.push(LexItem::Url(c.to_string() + str));
                continue;
            }
            _ => {
                if let Some(v) = result.last_mut() {
                    match v {
                        LexItem::Word(v) => *v += &c.to_string(),
                        _ => result.push(LexItem::Word(String::from(c))),
                    }
                } else {
                    result.push(LexItem::Word(String::from(c)))
                }
            }
        }
    }

    result
}

fn consume_url_chars(chars: &str) -> usize {
    for (i, c) in chars.chars().enumerate() {
        if c.is_alphanumeric() || ":/-_,.#%?[]@!$&'*+;=".contains(c) {
            continue;
        }
        return i;
    }
    return chars.len();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_notation() {
        let result = lex("@name Memory Management".into());
        assert_eq!(
            result,
            vec![
                LexItem::At("@".into()),
                LexItem::Word("name".into()),
                LexItem::Space,
                LexItem::Word("Memory".into()),
                LexItem::Space,
                LexItem::Word("Management".into())
            ]
        );

        let result = lex("\\name Memory Management".into());
        assert_eq!(
            result,
            vec![
                LexItem::At("\\".into()),
                LexItem::Word("name".into()),
                LexItem::Space,
                LexItem::Word("Memory".into()),
                LexItem::Space,
                LexItem::Word("Management".into())
            ]
        );

        let result = lex("\\\\name Memory Management".into());
        assert_eq!(
            result,
            vec![
                LexItem::At("\\\\".into()),
                LexItem::Word("name".into()),
                LexItem::Space,
                LexItem::Word("Memory".into()),
                LexItem::Space,
                LexItem::Word("Management".into())
            ]
        );
    }

    #[test]
    fn basic_groups() {
        let result = lex("@{\n* @name Memory Management\n@}".into());
        assert_eq!(
            result,
            vec![
                LexItem::At("@".into()),
                LexItem::Paren('{'),
                LexItem::NewLine,
                LexItem::Word("*".into()),
                LexItem::Space,
                LexItem::At("@".into()),
                LexItem::Word("name".into()),
                LexItem::Space,
                LexItem::Word("Memory".into()),
                LexItem::Space,
                LexItem::Word("Management".into()),
                LexItem::NewLine,
                LexItem::At("@".into()),
                LexItem::Paren('}')
            ]
        );
    }
}
