use std::collections::HashMap;

static STDLIB: &'static [(&'static str, &'static str)] = &[
    ("when", "if -rot {}"),
    ("-rot", "rot rot"),
];

#[derive(Clone, Debug, PartialEq)]
pub enum Word {
    Atom(String),
    Int(i32),
    Str(String),
    List(Vec<Word>),
}

pub type Program = Vec<Word>;

#[derive(Copy, Clone, Debug)]
pub enum ParseErr {
    MissingOpenBrace,
    MissingCloseBrace,
    MissingEndQuote,
}

pub fn parse(input: &str) -> Result<Program, ParseErr> {
    let mut stream = input.chars().peekable();
    let mut stack = Stack::with_capacity(8);
    stack.push();

    while let Some(ch) = stream.next() {
        match ch {
            '{' => stack.push(),

            '}' => stack.pop()?,

            '#' => loop {
                match stream.next() {
                    Some('\n') | None => break,
                    _ => (),
                }
            },

            '"' => {
                let mut buf = String::new();
                loop {
                    match stream.next() {
                        None => return Err(ParseErr::MissingEndQuote),
                        Some('"') => break,
                        Some(ch) => buf.push(ch),
                    }
                }
                stack.emit(Word::Str(buf))?;
            },

            ';' | '\n' => {
                stack.newline()?;
            },

            s if s.is_whitespace() => continue,

            w => {
                let mut word = String::new();
                word.push(w);
                while let Some(&ch) = stream.peek() {
                    if ch.is_whitespace() || ch == '{' || ch == '}' {
                        break;
                    }

                    word.extend(stream.next());

                    if ch == '=' { break; }
                }

                if let Ok(int) = word.parse::<i32>() {
                    stack.emit(Word::Int(int))?;
                } else {
                    stack.emit(Word::Atom(word))?;
                }
            },
        }
    }

    let program = stack.flatten()?;
    if stack.0.is_empty() {
        Ok(program)
    } else {
        Err(ParseErr::MissingCloseBrace)
    }
}

struct Stack(Vec<Block>);

type Block = Vec<Line>;

type Line = Vec<Word>;

impl Stack {
    fn with_capacity(n: usize) -> Self {
        Stack(Vec::with_capacity(n))
    }

    fn push(&mut self) {
        let mut block = Vec::with_capacity(16);
        block.push(Vec::with_capacity(16));
        self.0.push(block);
    }

    fn pop(&mut self) -> Result<(), ParseErr> {
        let list = self.flatten()?;
        self.emit(Word::List(list))
    }

    fn newline(&mut self) -> Result<(), ParseErr> {
        let block = self.0.iter_mut().last()
            .ok_or(ParseErr::MissingOpenBrace)?;

        block.push(Vec::with_capacity(16));
        Ok(())
    }

    fn emit(&mut self, word: Word) -> Result<(), ParseErr> {
        if let Some(block) = self.0.iter_mut().last() {
            let line = block.iter_mut().last().unwrap();
            line.push(word);
            Ok(())
        } else {
            Err(ParseErr::MissingOpenBrace)
        }
    }

    fn flatten(&mut self) -> Result<Program, ParseErr> {
        if let Some(mut block) = self.0.pop() {
            let total_len = block.iter().map(|line| line.len()).sum();
            let mut list = Vec::with_capacity(total_len);
            while let Some(line) = block.pop() {
                list.extend(line.into_iter());
            }
            Ok(list)
        } else {
            Err(ParseErr::MissingOpenBrace)
        }
    }
}

#[derive(Clone, Debug)]
pub enum EvalErr {
    StackUnderflow,
    CantUnderstand(String),
    DivideByZero,
    WrongType(Word, TypeName),
    BadParse(ParseErr),
}

#[derive(Copy, Clone, Debug)]
pub enum TypeName {
    Atom,
    Int,
    Str,
    List,
}

pub struct Env {
    bindings: HashMap<String, Word>,
    data: Vec<Word>,
    code: Vec<Word>,
    restore: Option<Box<Env>>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            bindings: STDLIB.iter().map(|&(ref k, ref v)| {
                ((*k).to_owned(), Word::List(parse(v).unwrap()))
            }).collect(),
            data: Vec::new(),
            code: Vec::new(),
            restore: None,
        }
    }

    pub fn run(&mut self, program: Program) -> Result<(), EvalErr> {
        self.code.extend(program.into_iter());

        while let Some(word) = self.code.pop() {
            match word {
                Word::List(words) => {
                    self.push(words);
                    continue;
                },

                Word::Atom(name) => {
                    if &name == "bye" {
                        return Ok(());
                    } else if let Err(err) = self.eval(&name) {
                        if let Some(env) = self.restore.take() {
                            *self = *env;
                            self.push(format!("{} error: {}", name, err));
                        } else {
                            return Err(err);
                        }
                    }
                },

                other => self.push(other),
            }
        }

        Ok(())
    }

    pub fn view(&self) -> &[Word] {
        &self.data
    }

    fn eval(&mut self, name: &str) -> Result<(), EvalErr> {
        match name {
            "if" => {
                let test = self.pop()?.as_bool()?;
                let then_clause = self.pop()?.as_list()?;
                let else_clause = self.pop()?.as_list()?;
                if test {
                    self.code.extend(then_clause.into_iter());
                } else {
                    self.code.extend(else_clause.into_iter());
                }
            },

            "try" => {
                let body = self.pop()?.as_list()?;
                let catch = self.pop()?.as_list()?;

                let mut restore = Env {
                    bindings: self.bindings.clone(),
                    code: self.code.clone(),
                    data: self.data.clone(),
                    restore: self.restore.take(),
                };

                restore.code.extend(catch);

                self.code.push(Word::Atom("end try".into()));
                self.code.extend(body);
                self.restore = Some(Box::new(restore));
            },

            "end try" => {
                if let Some(restore) = self.restore.take() {
                    self.restore = restore.restore;
                }
            },

            "eval" => {
                let body = self.pop()?.as_list()?;
                self.code.extend(body.into_iter());
            },

            "loop" => {
                let body = self.pop()?.as_list()?;
                self.code.push(Word::Atom("loop".into()));
                self.code.push(Word::List(body.clone()));
                self.code.extend(body.into_iter());
            },

            "view" => {
                let dump = self.view().iter().cloned().collect::<Vec<_>>();
                self.push(dump);
            },

            "len" => {
                let len = self.data.len() as i32;
                self.push(len);
            },

            "parse" => {
                let source = self.pop()?.as_str()?;
                let program = parse(&source)?;
                self.push(program);
            },

            "echo" => println!("{}", self.pop()?.into_string()),

            "prompt" => {
                use std::io::{stdin, stdout, Write};

                let text = self.pop()?.into_string();
                print!("{}", text);
                stdout().flush().unwrap();

                let mut inbuf = String::new();
                stdin().read_line(&mut inbuf).unwrap();
                inbuf.pop(); // Discard '\n'

                self.push(inbuf);
            },

            "flatten" => {
                let sep = self.pop()?.into_string();
                let list = self.pop()?.into_list();
                self.push(list.flatten(&sep));
            },

            "swap" => {
                let a = self.pop()?;
                let b = self.pop()?;
                self.push(a);
                self.push(b);
            },

            "rot" => {
                let a = self.pop()?;
                let b = self.pop()?;
                let c = self.pop()?;
                self.push(b);
                self.push(a);
                self.push(c);
            },

            "dup" => {
                let val = self.pop()?;
                self.push(val.clone());
                self.push(val);
            },

            "drop" => { let _ = self.pop()?; },

            "clear" => self.data.clear(),

            "concat" => {
                let mut lhs = self.pop()?.into_string();
                let rhs = self.pop()?.into_string();
                lhs.push_str(&rhs);
                self.push(lhs);
            },

            "+" => self.int_binop(|x, y| Ok(x + y))?,
            "-" => self.int_binop(|x, y| Ok(x - y))?,
            "*" => self.int_binop(|x, y| Ok(x * y))?,
            "/" => self.int_binop(|x, y| x.checked_div(y).ok_or({
                EvalErr::DivideByZero
            }))?,

            "~" => {
                let positive = self.pop()?.as_int()?;
                self.push(-positive);
            },

            "=" => self.int_binop(|x, y| Ok(x == y))?,
            ">" => self.int_binop(|x, y| Ok(x > y))?,
            "<" => self.int_binop(|x, y| Ok(x < y))?,

            other => if other.ends_with("=") {
                let mut name = other.to_owned();
                name.pop(); // Remove final '='
                let value = self.pop()?;
                self.bindings.insert(name, value);
            } else if let Some(value) = self.bindings.get(other).cloned() {
                match value {
                    Word::List(words) => self.code.extend(words),
                    other => self.push(other),
                }
            } else {
                return Err(EvalErr::CantUnderstand(other.to_owned()));
            },
        }

        Ok(())
    }

    fn int_binop<R, F>(&mut self, op: F) -> Result<(), EvalErr>
        where R: Into<Word>, F: FnOnce(i32, i32) -> Result<R, EvalErr>
    {
        let lhs = self.pop()?.as_int()?;
        let rhs = self.pop()?.as_int()?;
        self.push(op(lhs, rhs)?);
        Ok(())
    }

    fn push<T: Into<Word>>(&mut self, t: T) {
        self.data.push(t.into());
    }

    fn pop(&mut self) -> Result<Word, EvalErr> {
        self.data.pop().ok_or(EvalErr::StackUnderflow)
    }
}

impl From<bool> for Word {
    fn from(b: bool) -> Self {
        match b {
            true => Word::Int(1),
            false => Word::Int(0),
        }
    }
}

impl From<i32> for Word {
    fn from(i: i32) -> Self {
        Word::Int(i)
    }
}

impl From<Vec<Word>> for Word {
    fn from(words: Vec<Word>) -> Self {
        Word::List(words)
    }
}

impl From<String> for Word {
    fn from(string: String) -> Self {
        Word::Str(string)
    }
}

impl Word {
    fn as_bool(self) -> Result<bool, EvalErr> {
        match self {
            Word::Int(0) => Ok(false),
            Word::Int(_) => Ok(true),
            val => Err(EvalErr::WrongType(val, TypeName::Int)),
        }
    }

    fn as_int(self) -> Result<i32, EvalErr> {
        match self {
            Word::Int(i) => Ok(i),
            val => Err(EvalErr::WrongType(val, TypeName::Int)),
        }
    }

    fn as_list(self) -> Result<Vec<Word>, EvalErr> {
        match self {
            Word::List(words) => Ok(words),
            val => Err(EvalErr::WrongType(val, TypeName::List)),
        }
    }

    fn as_str(self) -> Result<String, EvalErr> {
        match self {
            Word::Str(s) => Ok(s),
            val => Err(EvalErr::WrongType(val, TypeName::Str)),
        }
    }

    fn into_string(self) -> String {
        match self {
            Word::Str(s) => s,
            other => format!("{}", other),
        }
    }

    fn into_list(self) -> Vec<Word> {
        match self {
            Word::List(list) => list,
            other => vec![other],
        }
    }
}

impl From<ParseErr> for EvalErr {
    fn from(err: ParseErr) -> Self {
        EvalErr::BadParse(err)
    }
}

mod display {
    use std::fmt;

    use super::*;

    impl fmt::Display for Word {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &Word::Int(i) => write!(f, "{}", i),

                &Word::Str(ref s) => write!(f, "\"{}\"", s),

                &Word::Atom(ref a) => write!(f, "{}", a),

                &Word::List(ref words) => {
                    write!(f, "{{ {} }}", words.flatten(" "))
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
            })
        }
    }

    impl fmt::Display for EvalErr {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &EvalErr::StackUnderflow => write!(f, "stack underflow"),

                &EvalErr::DivideByZero => write!(f, "divided by zero"),

                &EvalErr::WrongType(ref word, ref typename) => {
                    write!(f, "type of {} is not {}", word, typename)
                },

                &EvalErr::CantUnderstand(ref name) => {
                    write!(f, "can't understand {}", name)
                },

                &EvalErr::BadParse(ref err) => {
                    write!(f, "{}", err)
                },
            }
        }
    }

    impl fmt::Display for TypeName {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", match self {
                &TypeName::Atom => "atom",
                &TypeName::Int => "integer",
                &TypeName::Str => "string",
                &TypeName::List => "list",
            })
        }
    }
}

pub trait Flattenable {
    fn flatten(&self, &str) -> String;
}

impl Flattenable for [Word] {
    fn flatten(&self, sep: &str) -> String {
        self.iter().map(|word| {
            format!("{}", word)
        }).collect::<Vec<_>>().join(sep)
    }
}

#[test]
fn valid_parse() {
    let inputs = vec![
        "if test { + 2 2 } { + 1 3 }",
        "if test {+ 2 2} {+ 1 3}",
        "k= 1",
    ];

    for input in inputs {
        parse(input).unwrap();
    }
}
