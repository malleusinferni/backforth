use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Word {
    Atom(String),
    Closure(Vec<Word>),
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

                stack.emit(Word::Atom(word))?;
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
        if let Some(closure) = self.0.pop() {
            self.emit(Word::Closure(closure))
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

#[derive(Clone, Debug)]
pub enum Value {
    Int(i32),
    Closure(Vec<Word>),
}

pub struct Env {
    bindings: HashMap<String, Value>,
    data: Vec<Value>,
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

    pub fn run(&mut self, program: Program) -> Result<Vec<Value>, EvalErr> {
        self.code.extend(program.into_iter());

        while let Some(word) = self.code.pop() {
            match word {
                Word::Closure(words) => {
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
            }
        }

        Ok(self.data.drain(..).collect())
    }

    fn eval(&mut self, name: &str) -> Result<(), EvalErr> {
        match name {
            "if" => {
                let test = self.pop()?.as_bool()?;
                let then_clause = self.pop()?.as_closure()?;
                let else_clause = self.pop()?.as_closure()?;
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

            other => if let Ok(int) = other.parse::<i32>() {
                self.push(int);
            } else if other.ends_with("=") {
                let mut name = other.to_owned();
                name.pop(); // Remove final '='
                let value = self.pop()?;
                self.bindings.insert(name, value);
            } else if let Some(value) = self.bindings.get(other).cloned() {
                match value {
                    Value::Closure(words) => self.code.extend(words),
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
        where R: Into<Value>, F: FnOnce(i32, i32) -> Result<R, EvalErr>
    {
        let lhs = self.pop()?.as_int()?;
        let rhs = self.pop()?.as_int()?;
        self.push(op(lhs, rhs)?);
        Ok(())
    }

    fn push<T: Into<Value>>(&mut self, t: T) {
        self.data.push(t.into());
    }

    fn pop(&mut self) -> Result<Value, EvalErr> {
        self.data.pop().ok_or(EvalErr)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        match b {
            true => Value::Int(1),
            false => Value::Int(0),
        }
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i)
    }
}

impl From<Vec<Word>> for Value {
    fn from(words: Vec<Word>) -> Self {
        Value::Closure(words)
    }
}

impl Value {
    fn as_bool(self) -> Result<bool, EvalErr> {
        match self {
            Value::Int(1) => Ok(true),
            Value::Int(_) => Ok(false),
            _ => Err(EvalErr),
        }
    }

    fn as_int(self) -> Result<i32, EvalErr> {
        match self {
            Value::Int(i) => Ok(i),
            _ => Err(EvalErr),
        }
    }

    fn as_closure(self) -> Result<Vec<Word>, EvalErr> {
        match self {
            Value::Closure(words) => Ok(words),
            _ => Err(EvalErr),
        }
    }
}

mod display {
    use std::fmt;

    use super::*;

    impl fmt::Display for Value {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &Value::Int(i) => write!(f, "{}", i),

                &Value::Closure(ref words) => fmt_closure(f, &words),
            }
        }
    }

    impl fmt::Display for Word {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &Word::Atom(ref a) => write!(f, "{}", a),

                &Word::Closure(ref words) => fmt_closure(f, &words),
            }
        }
    }

    fn fmt_closure(f: &mut fmt::Formatter, words: &[Word]) -> fmt::Result {
        write!(f, "{{ {} }}", words.iter().map(|word| {
            format!("{}", word)
        }).collect::<Vec<_>>().join(" "))
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
