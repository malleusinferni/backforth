use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Word {
    Atom(String),
    Int(i32),
    List(Vec<Word>),
}

pub type Program = Vec<Word>;

#[derive(Copy, Clone, Debug)]
pub struct ParseErr;

pub fn parse(input: &str) -> Result<Program, ParseErr> {
    let input = input.lines().rev().collect::<Vec<_>>().join("\n");

    let mut stream = input.chars().peekable();
    let mut stack = Stack::with_capacity(8);
    stack.push();

    while let Some(ch) = stream.next() {
        match ch {
            '{' => stack.push(),

            '}' => stack.pop()?,

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

    stack.flatten()
}

struct Stack(Vec<Program>);

impl Stack {
    fn with_capacity(n: usize) -> Self {
        Stack(Vec::with_capacity(n))
    }

    fn push(&mut self) {
        self.0.push(Vec::with_capacity(16));
    }

    fn pop(&mut self) -> Result<(), ParseErr> {
        if let Some(list) = self.0.pop() {
            self.emit(Word::List(list))
        } else {
            Err(ParseErr)
        }
    }

    fn emit(&mut self, word: Word) -> Result<(), ParseErr> {
        if let Some(program) = self.0.iter_mut().last() {
            program.push(word);
            Ok(())
        } else {
            Err(ParseErr)
        }
    }

    fn flatten(mut self) -> Result<Program, ParseErr> {
        if let Some(program) = self.0.pop() {
            if self.0.is_empty() {
                return Ok(program);
            }
        }

        Err(ParseErr)
    }
}

pub struct EvalErr;

pub struct Env {
    bindings: HashMap<String, Word>,
    data: Vec<Word>,
    code: Vec<Word>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            bindings: HashMap::new(),
            data: Vec::new(),
            code: Vec::new(),
        }
    }

    pub fn run(&mut self, program: Program) -> Result<Vec<Word>, EvalErr> {
        self.code.extend(program.into_iter());

        while let Some(word) = self.code.pop() {
            match word {
                Word::List(words) => {
                    self.push(words);
                    continue;
                },

                Word::Atom(name) => match self.eval(&name) {
                    Ok(()) => continue,

                    Err(err) => {
                        self.code.clear();
                        self.data.clear();
                        return Err(err);
                    },
                },

                other => self.push(other),
            }
        }

        Ok(self.data.drain(..).collect())
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

            "echo" => println!("{}", self.pop()?),

            "drop" => { let _ = self.pop()?; },

            "+" => self.int_binop(|x, y| Ok(x + y))?,
            "-" => self.int_binop(|x, y| Ok(x - y))?,
            "*" => self.int_binop(|x, y| Ok(x * y))?,
            "/" => self.int_binop(|x, y| x.checked_div(y).ok_or(EvalErr))?,

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
                println!("Can't understand {}", other);
                return Err(EvalErr);
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
        self.data.pop().ok_or(EvalErr)
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

impl Word {
    fn as_bool(self) -> Result<bool, EvalErr> {
        match self {
            Word::Int(0) => Ok(false),
            Word::Int(_) => Ok(true),
            _ => Err(EvalErr),
        }
    }

    fn as_int(self) -> Result<i32, EvalErr> {
        match self {
            Word::Int(i) => Ok(i),
            _ => Err(EvalErr),
        }
    }

    fn as_list(self) -> Result<Vec<Word>, EvalErr> {
        match self {
            Word::List(words) => Ok(words),
            _ => Err(EvalErr),
        }
    }
}

mod display {
    use std::fmt;

    use super::*;

    impl fmt::Display for Word {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &Word::Int(i) => write!(f, "{}", i),

                &Word::Atom(ref a) => write!(f, "{}", a),

                &Word::List(ref words) => {
                    write!(f, "{{ {} }}", words.flatten(" "))
                },
            }
        }
    }
}

pub trait Flattenable {
    fn flatten(&self, &str) -> String;
}

impl Flattenable for Vec<Word> {
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
