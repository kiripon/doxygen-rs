use crate::emojis;
use crate::parser::{parse, GrammarItem, ParseError};

#[derive(Clone, Copy, Default)]
struct GenState {
    pub already_added_params: bool,
    pub already_added_returns: bool,
    pub already_added_throws: bool,
    pub already_added_pre: bool,
    pub already_added_post: bool,
    pub already_added_see: bool,
}

/// Creates a Rustdoc string from a Doxygen string.
///
/// # Errors
///
/// This function can error if there are missing parts of a given Doxygen annotation (like `@param`
/// missing the variable name)
pub fn rustdoc(input: String) -> Result<String, ParseError> {
    let parsed = parse(input)?;
    let mut result = String::new();
    let mut gen_state: GenState = GenState::default();
    let mut group_started = false;

    for item in parsed {
        result += &match item {
            GrammarItem::Notation { meta, params, tag } => {
                let (str, new_gen_state) = generate_notation(tag, meta, params, gen_state)?;
                if new_gen_state.already_added_params {
                    gen_state.already_added_params = true;
                }

                if new_gen_state.already_added_returns {
                    gen_state.already_added_returns = true;
                }

                if new_gen_state.already_added_throws {
                    gen_state.already_added_throws = true;
                }

                if new_gen_state.already_added_pre {
                    gen_state.already_added_pre = true;
                }

                if new_gen_state.already_added_post {
                    gen_state.already_added_post = true;
                }

                str
            }
            GrammarItem::Text(v) => if group_started {
                v.replacen("*", "", 1)
            } else {
                v
            },
            // See <https://stackoverflow.com/a/40354789>
            GrammarItem::GroupStart => {
                group_started = true;
                String::from("# ")
            },
            GrammarItem::GroupEnd => {
                group_started = false;
                continue
            },
            GrammarItem::Url(url) => ["<", &url, ">"].concat(),
        };
    }

    Ok(result)
}

fn generate_notation(
    tag: String,
    meta: Vec<String>,
    params: Vec<String>,
    gen_state: GenState,
) -> Result<(String, GenState), ParseError> {
    let mut new_state = GenState::default();

    Ok((
        match tag.as_str() {
            "param" => {
                let param = params.get(0);
                new_state.already_added_params = true;
                let mut str = if !gen_state.already_added_params {
                    "# Arguments\n\n ".into()
                } else {
                    String::new()
                };

                str += &if let Some(param) = param {
                    if meta.is_empty() {
                        format!("* `{param}` -")
                    } else {
                        if let Some(second) = meta.get(1) {
                            format!(
                                "* `{}` (direction {}, {}) -",
                                param,
                                meta.get(0).unwrap(),
                                second
                            )
                        } else {
                            format!("* `{}` (direction {}) -", param, meta.get(0).unwrap())
                        }
                    }
                } else {
                    String::new()
                };

                str
            }
            "a" | "e" | "em" => {
                let word = params
                    .get(0)
                    .expect("@a/@e/@em doesn't contain a word to style");
                format!("_{word}_")
            }
            "b" => {
                let word = params.get(0).expect("@b doesn't contain a word to style");
                format!("**{word}**")
            }
            "c" | "p" => {
                let word = params
                    .get(0)
                    .expect("@c/@p doesn't contain a word to style");
                format!("`{word}`")
            }
            "emoji" => {
                let word = params.get(0).expect("@emoji doesn't contain an emoji");
                emojis::EMOJIS
                    .get(&word.replace(':', ""))
                    .expect("invalid emoji")
                    .to_string()
            }
            "sa" | "see" => {
                let mut str = String::new();
                if !gen_state.already_added_see {
                    str += "# See also\n\n ";
                    new_state.already_added_see = true;
                }

                if let Some(code_ref) = params.get(0) {
                    str += &format!("[`{code_ref}`]");
                }
                str
            }
            "retval" => {
                let var = params.get(0).expect("@retval doesn't contain a parameter");
                new_state.already_added_returns = true;
                let mut str = if !gen_state.already_added_returns {
                    "# Returns\n\n ".into()
                } else {
                    String::new()
                };

                str += &format!("* `{var}` -");
                str
            }
            "returns" | "return" | "result" => {
                new_state.already_added_returns = true;
                if !gen_state.already_added_returns {
                    "# Returns\n\n ".into()
                } else {
                    String::new()
                }
            }
            "throw" | "throws" | "exception" => {
                new_state.already_added_throws = true;
                let exception = params.get(0).expect("@param doesn't contain a parameter");

                let mut str = if !gen_state.already_added_throws {
                    "# Throws\n\n ".into()
                } else {
                    String::new()
                };

                str += &format!("* [`{exception}`] -");
                str
            }
            "note" => String::from("> **Note:** "),
            "since" => String::from("> Available since: "),
            "deprecated" => String::from("> **Deprecated** "),
            "remark" | "remarks" => String::from("> "),
            "par" => String::from("# "),
            "pre" => {
                new_state.already_added_pre = true;

                let mut str = if !gen_state.already_added_pre {
                    String::from("# Precondition\n\n ")
                } else {
                    String::new()
                };
                if let Some(precondition) = params.get(0) {
                    str += &format!("* {precondition}");
                }
                str
            }
            "post" => {
                new_state.already_added_post = true;

                let mut str = if !gen_state.already_added_post {
                    String::from("# Postcondition\n\n ")
                } else {
                    String::new()
                };
                if let Some(postcondition) = params.get(0) {
                    str += &format!("* {postcondition}");
                }
                str
            }
            "details" => String::from("\n\n "),
            "brief" | "short" => String::new(),
            _ => String::new(),
        },
        new_state,
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test_rustdoc {
        ($input:literal, $expected:literal) => {
            let result = $crate::generator::rustdoc($input.into()).unwrap();
            assert_eq!(result, $expected);
        };
    }

    #[test]
    fn unknown_annotation() {
        test_rustdoc!("@thisdoesntexist Example doc", "Example doc");
    }

    #[test]
    fn param_with_direction() {
        test_rustdoc!(
            "@param[in] example This insane thing.",
            "# Arguments\n\n* `example` (direction in) - This insane thing."
        );

        test_rustdoc!(
            "@param[in,out] example This insane thing.",
            "# Arguments\n\n* `example` (direction in, out) - This insane thing."
        );

        test_rustdoc!(
            "@param[out,in] example This insane thing.",
            "# Arguments\n\n* `example` (direction in, out) - This insane thing."
        );
    }

    #[test]
    fn param_without_direction() {
        test_rustdoc!(
            "@param example This is definitively an example!",
            "# Arguments\n\n* `example` - This is definitively an example!"
        );
    }

    #[test]
    fn multiple_params() {
        test_rustdoc!(
            "@param example1 This is the first example\n@param[out] example2 This is the second example\n@param[in] example3 This is the third example.",
            "# Arguments\n\n* `example1` - This is the first example\n* `example2` (direction out) - This is the second example\n* `example3` (direction in) - This is the third example."
        );
    }

    #[test]
    fn italics() {
        test_rustdoc!(
            "This @a thing is without a doubt @e great. @em And you won't tell me otherwise.",
            "This _thing_ is without a doubt _great._ _And_ you won't tell me otherwise."
        );
    }

    #[test]
    fn bold() {
        test_rustdoc!("This is a @b bold claim.", "This is a **bold** claim.");
    }

    #[test]
    fn code_inline() {
        test_rustdoc!(
            "@c u8 is not the same as @p u32",
            "`u8` is not the same as `u32`"
        );
    }

    #[test]
    fn emoji() {
        test_rustdoc!("@emoji :relieved: @emoji :ok_hand:", "😌 👌");
    }

    #[test]
    fn text_styling() {
        test_rustdoc!(
            "This is from @a Italy. ( @b I @c hope @emoji :pray: )",
            "This is from _Italy._ ( **I** `hope` 🙏 )"
        );
    }

    #[test]
    fn brief() {
        test_rustdoc!(
            "@brief This function does things.\n@short This function also does things.",
            "This function does things.\nThis function also does things."
        );
    }

    #[test]
    fn see_also() {
        test_rustdoc!(
            "@sa random_thing @see random_thing_2",
            "[`random_thing`] [`random_thing_2`]"
        );
    }

    #[test]
    fn deprecated() {
        test_rustdoc!(
            "@deprecated This function is deprecated!\n@param example_1 Example 1.",
            "> **Deprecated** This function is deprecated!\n# Arguments\n\n* `example_1` - Example 1."
        );
    }

    #[test]
    fn details() {
        test_rustdoc!(
            "@brief This function is insane!\n@details This is an insane function because its functionality and performance is quite astonishing.",
            "This function is insane!\n\n\nThis is an insane function because its functionality and performance is quite astonishing."
        );
    }

    #[test]
    fn paragraph() {
        test_rustdoc!(
            "@par Interesting fact about this function\nThis is a function.",
            "# Interesting fact about this function\nThis is a function."
        );
    }

    #[test]
    fn remark() {
        test_rustdoc!(
            "@remark This things needs to be\n@remark remarked.",
            "> This things needs to be\n> remarked."
        );
    }

    #[test]
    fn returns() {
        test_rustdoc!(
            "@returns A value that should be\n@return used with caution.\n@result And if it's @c -1 ... run.",
            "# Returns\n\nA value that should be\nused with caution.\nAnd if it's `-1` ... run."
        );
    }

    #[test]
    fn return_value() {
        test_rustdoc!(
            "@retval example1 This return value is great!",
            "# Returns\n\n* `example1` - This return value is great!"
        );
    }

    #[test]
    fn returns_and_return_value() {
        test_rustdoc!(
            "@returns Great values!\n@retval example1 Is this an example?\n@return Also maybe more things (?)",
            "# Returns\n\nGreat values!\n* `example1` - Is this an example?\nAlso maybe more things (?)"
        );

        test_rustdoc!(
            "@returns Great values!\n@return Also maybe more things (?)\n@retval example1 Is this an example?",
            "# Returns\n\nGreat values!\nAlso maybe more things (?)\n* `example1` - Is this an example?"
        );

        test_rustdoc!(
            "@retval example1 Is this an example?\n@returns Great values!\n@return Also maybe more things (?)",
            "# Returns\n\n* `example1` - Is this an example?\nGreat values!\nAlso maybe more things (?)"
        );
    }

    #[test]
    fn since() {
        test_rustdoc!(
            "@since The bite of '87",
            "> Available since: The bite of '87"
        );
    }

    #[test]
    fn throws() {
        test_rustdoc!(
            "@throw std::io::bonk This is thrown when INSANE things happen.\n@throws std::net::meow This is thrown when BAD things happen.\n@exception std::fs::no This is thrown when NEFARIOUS things happen.",
            "# Throws\n\n* [`std::io::bonk`] - This is thrown when INSANE things happen.\n* [`std::net::meow`] - This is thrown when BAD things happen.\n* [`std::fs::no`] - This is thrown when NEFARIOUS things happen."
        );
    }

    #[test]
    fn can_parse_example() {
        let example = include_str!("../tests/assets/example-bindgen.rs");
        println!("{}", rustdoc(example.into()).unwrap());
    }

    #[test]
    fn precondition() {
        test_rustdoc!(
            "@pre precondition\n@pre precondition2\n@pre precondition3",
            "# Precondition\n\n* precondition\n* precondition2\n* precondition3"
        );
    }
    #[test]
    fn postcondition() {
        test_rustdoc!(
            "@post postcondition\n@post postcondition2\n@post postcondition3",
            "# Postcondition\n\n* postcondition\n* postcondition2\n* postcondition3"
        );
    }

}
