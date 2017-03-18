mod parser;
mod display;

use std::collections::{HashMap, VecDeque};

use parser::{ParseErr};

pub use parser::parse;

static STDLIB: &'static [(&'static str, &'static str)] = &[
    ("when", "if -rot {}"),
    ("loop", "eval append { loop } unshift dup"),
    ("-rot", "rot rot"),
    ("first", "drop swap shift"),
    ("last", "drop swap pop"),
];

#[derive(Clone, Debug, PartialEq)]
pub enum Word {
    Atom(String),
    Int(i32),
    Hex(u32),
    Str(String),
    List(VecDeque<Word>),
    Dict(HashMap<String, Word>),
}

#[derive(Clone, Debug)]
pub enum EvalErr {
    StackUnderflow,
    CantUnderstand(String),
    DivideByZero,
    CantCoerce(Word, TypeName),
    WrongType(Word, TypeName),
    BadParse(ParseErr),
    EmptyList,
    MacroFailed,
}

#[derive(Copy, Clone, Debug)]
pub enum TypeName {
    Atom,
    Int,
    Hex,
    Str,
    List,
}

pub struct Shell {
    dict: HashMap<String, Word>,
    data: Vec<Word>,
    code: Vec<Word>,
    restore: Vec<Env>,
}

struct Env {
    dict: HashMap<String, Word>,
    data: Vec<Word>,
    code: Vec<Word>,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            dict: STDLIB.iter().map(|&(ref k, ref v)| {
                ((*k).to_owned(), Word::List(parse(v).unwrap().into()))
            }).collect(),
            data: Vec::new(),
            code: Vec::new(),
            restore: Vec::new(),
        }
    }

    pub fn load<P: Iterator<Item=Word>>(&mut self, program: P) {
        self.code.extend(program);
    }

    pub fn run(&mut self) -> Result<(), EvalErr> {
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
                        if let Some(env) = self.restore.pop() {
                            self.dict = env.dict;
                            self.data = env.data;
                            self.code = env.code;
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
                    self.load(then_clause.into_iter());
                } else {
                    self.load(else_clause.into_iter());
                }
            },

            "try" => {
                /*
                 * popeh eval pusheh ... swap
                 */
                let body = self.pop()?.as_list()?;
                let catch = self.pop()?.as_list()?;

                let mut restore = Env {
                    dict: self.dict.clone(),
                    code: self.code.clone(),
                    data: self.data.clone(),
                };

                restore.code.extend(catch);
                self.restore.push(restore);

                self.code.push(Word::atom("popeh"));
                self.load(body.into_iter());
            },

            "popeh" => {
                self.restore.pop();
            },

            "explode" => {
                let items = self.pop()?.as_list()?;
                self.data.extend(items.into_iter());
            },

            "eval" => {
                let body = self.pop()?.as_list()?;
                self.load(body.into_iter());
            },

            "quote" => {
                let word = self.code.pop().ok_or(EvalErr::StackUnderflow)?;
                self.data.push(word);
            },

            "capture" => {
                let dump = self.view().iter().cloned().collect::<Vec<_>>();
                self.push(dump);
            },

            "bindings" => {
                let dict = self.dict.clone();
                self.push(dict);
            },

            "debug" => {
                for word in self.code.iter().rev() {
                    for line in word.pretty_print(0) {
                        println!("{}", line);
                    }
                }
            },

            "inspect" => {
                let name = self.pop()?.as_atom()?;
                let def = self.dict.get(&name)
                    .ok_or(EvalErr::CantUnderstand(name))?;

                for line in def.pretty_print(0) {
                    println!("{}", line);
                }
            },

            "len" => {
                let len = self.pop()?.as_list()?.len();
                self.push(len as i32);
            },

            "append" => {
                let mut lhs = self.pop()?.as_list()?;
                let rhs = self.pop()?.as_list()?;
                lhs.extend(rhs.into_iter());
                self.push(lhs);
            },

            "push" => {
                let value = self.pop()?;
                let mut list = self.pop()?.as_list()?;
                list.push_back(value);
                self.push(list);
            },

            "pop" => {
                let mut list = self.pop()?.as_list()?;
                let value = list.pop_back().ok_or(EvalErr::EmptyList)?;
                self.push(list);
                self.push(value);
            },

            "shift" => {
                let mut list = self.pop()?.as_list()?;
                let value = list.pop_front().ok_or(EvalErr::EmptyList)?;
                self.push(list);
                self.push(value);
            },

            "unshift" => {
                let value = self.pop()?;
                let mut list = self.pop()?.as_list()?;
                list.push_front(value);
                self.push(list);
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

            "load" => {
                use std::fs::File;
                use std::io::Read;

                let path = self.pop()?.as_str()?;

                let mut inbuf = String::new();
                let mut file = File::open(&path).unwrap();
                file.read_to_string(&mut inbuf).unwrap();

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

            "strcat" => {
                let mut lhs = self.pop()?.into_string();
                let rhs = self.pop()?.into_string();
                lhs.push_str(&rhs);
                self.push(lhs);
            },

            "hex" => {
                let hex = self.pop()?.into_hex()?;
                self.push(hex);
            },

            "int" => {
                let int = self.pop()?.into_int()?;
                self.push(int);
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

            "==" => self.int_binop(|x, y| Ok(x == y))?,
            ">" => self.int_binop(|x, y| Ok(x > y))?,
            "<" => self.int_binop(|x, y| Ok(x < y))?,

            "=" => {
                let value = self.pop()?;
                let name = self.code.pop()
                    .ok_or(EvalErr::MacroFailed)?
                    .as_atom()?;
                self.dict.insert(name, value);
            },

            "))" => {
                let rhs = self.code.pop().ok_or(EvalErr::MacroFailed)?;
                let op = self.code.pop().ok_or(EvalErr::MacroFailed)?;
                let lhs = self.code.pop().ok_or(EvalErr::MacroFailed)?;

                if let Some(Word::Atom(name)) = self.code.pop() {
                    if &name == "((" {
                        self.code.push(op);
                        self.code.push(lhs);
                        self.code.push(rhs);
                        return Ok(());
                    }
                }

                return Err(EvalErr::MacroFailed);
            },

            other => if let Some(value) = self.dict.get(other).cloned() {
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
        let lhs = self.pop()?.into_int()?;
        let rhs = self.pop()?.into_int()?;
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

impl From<u32> for Word {
    fn from(h: u32) -> Self {
        Word::Hex(h)
    }
}

impl From<Vec<Word>> for Word {
    fn from(words: Vec<Word>) -> Self {
        Word::List(words.into())
    }
}

impl From<VecDeque<Word>> for Word {
    fn from(words: VecDeque<Word>) -> Self {
        Word::List(words)
    }
}

impl From<String> for Word {
    fn from(string: String) -> Self {
        Word::Str(string)
    }
}

impl From<HashMap<String, Word>> for Word {
    fn from(dict: HashMap<String, Word>) -> Self {
        Word::Dict(dict)
    }
}

impl Word {
    fn atom(name: &str) -> Self {
        Word::Atom(name.to_owned())
    }

    fn as_atom(self) -> Result<String, EvalErr> {
        match self {
            Word::Atom(name) => Ok(name),
            val => Err(EvalErr::WrongType(val, TypeName::Atom)),
        }
    }

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

    fn as_list(self) -> Result<VecDeque<Word>, EvalErr> {
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

    fn into_int(self) -> Result<i32, EvalErr> {
        match self {
            Word::Int(i) => Ok(i),
            Word::Hex(h) if h <= i32::max_value() as u32 => Ok(h as i32),
            other => Err(EvalErr::CantCoerce(other, TypeName::Int)),
        }
    }

    fn into_hex(self) -> Result<u32, EvalErr> {
        match self {
            Word::Hex(h) => Ok(h),
            Word::Int(i) if i >= 0 => Ok(i as u32),
            other => Err(EvalErr::CantCoerce(other, TypeName::Hex)),
        }
    }

    fn into_string(self) -> String {
        match self {
            Word::Str(s) => s,
            other => format!("{}", other),
        }
    }

    fn into_list(self) -> VecDeque<Word> {
        match self {
            Word::List(list) => list,
            other => vec![other].into(),
        }
    }
}

impl From<ParseErr> for EvalErr {
    fn from(err: ParseErr) -> Self {
        EvalErr::BadParse(err)
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

impl Flattenable for VecDeque<Word> {
    fn flatten(&self, sep: &str) -> String {
        self.iter().map(|word| {
            format!("{}", word)
        }).collect::<Vec<_>>().join(sep)
    }
}

impl Flattenable for HashMap<String, Word> {
    fn flatten(&self, sep: &str) -> String {
        self.iter().map(|(ref k, ref v)| {
            format!("{} = {}", k, v)
        }).collect::<Vec<_>>().join(sep)
    }
}
