use std::fmt;

use super::*;

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Word::Int(i) => write!(f, "{}", i),

            &Word::Hex(h) => write!(f, "#{:x}", h),

            &Word::Str(ref s) => write!(f, "\"{}\"", s),

            &Word::Atom(ref a) => write!(f, "{}", a),

            &Word::List(ref words) => if words.is_empty() {
                write!(f, "{{}}")
            } else {
                write!(f, "{{ {} }}", words.flatten(" "))
            },

            &Word::Dict(ref map) => if map.len() == 0 {
                write!(f, "dict {{}}")
            } else {
                write!(f, "dict {{ {} }}", map.flatten("; "))
            },
        }
    }
}

impl fmt::Display for ParseErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &ParseErr::MissingOpenBrace => "missing {",
            &ParseErr::MissingCloseBrace => "missing }",
            &ParseErr::MissingEndQuote => "missing \"",
            &ParseErr::BadHexLiteral => "invalid hex format",
        })
    }
}

impl fmt::Display for EvalErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &EvalErr::StackUnderflow => write!(f, "stack underflow"),

            &EvalErr::DivideByZero => write!(f, "divided by zero"),

            &EvalErr::CantCoerce(ref word, ref typename) => {
                write!(f, "cannot convert {} to {}", word, typename)
            },

            &EvalErr::WrongType(ref word, ref typename) => {
                write!(f, "type of {} is not {}", word, typename)
            },

            &EvalErr::CantUnderstand(ref name) => {
                write!(f, "can't understand {}", name)
            },

            &EvalErr::BadParse(ref err) => {
                write!(f, "{}", err)
            },

            &EvalErr::EmptyList => {
                write!(f, "empty list")
            },

            &EvalErr::MacroFailed => {
                write!(f, "bad arguments for macro")
            },
        }
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            &TypeName::Atom => "atom",
            &TypeName::Int => "integer",
            &TypeName::Hex => "hex",
            &TypeName::Str => "string",
            &TypeName::List => "list",
        })
    }
}

impl Word {
    pub fn pretty_print(&self, indent_level: usize) -> Vec<String> {
        let mut lines = vec![];

        match self {
            &Word::List(ref items) => if items.is_empty() {
                lines.push("{}".to_owned());
            } else {
                lines.push("{".to_owned());

                for item in items.iter().rev() {
                    for line in item.pretty_print(indent_level + 1) {
                        lines.push(format!("    {}", line));
                    }
                }

                lines.push("}".to_owned());
            },

            other => lines.push(format!("{}", other)),
        }

        lines
    }
}
