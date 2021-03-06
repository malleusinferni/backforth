extern crate ordermap;

mod parser;
mod display;

use std::collections::{VecDeque};

use ordermap::OrderMap;

use parser::{ParseErr};

pub use parser::parse;

static STDLIB: &'static str = include_str!("stdlib.\\iv");

#[derive(Clone, Debug)]
pub enum Word {
    Atom(String),
    Int(i32),
    Hex(u32),
    Str(String),
    List(VecDeque<Word>),
    Dict(OrderMap<String, Word>),
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
    IllegalStackEffect(usize, usize),
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
    dict: OrderMap<String, Binding>,
    data: VecDeque<Word>,
    code: Vec<Word>,
    restore: Vec<Env>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Builtin {
    Bye,
    Assign,
    Eval,
    Expand,
    If,
    Try,
    PopEH,
    Quote,
    Explode,
    Capture,
    Debug,
    Inspect,
    Len,
    Append,
    Push,
    Pop,
    Shift,
    Unshift,
    Parse,
    Echo,
    Prompt,
    Command,
    Load,
    Flatten,
    Pick,
    Roll,
    Drop,
    Clear,
    Strcat,
    Lines,
    Hex,
    Int,
    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
    OpNeg,
    OpEql,
    OpLt,
    OpGt,
    InfixExpr,
}

#[derive(Clone, Debug)]
enum Binding {
    Primitive(Builtin),
    Interpreted(TypeSpec, Word),
}

#[derive(Copy, Clone, Debug)]
pub struct TypeSpec {
    pub input: usize,
    pub output: usize,
    pub exact: bool,
}

struct Env {
    dict: OrderMap<String, Binding>,
    data: VecDeque<Word>,
    code: Vec<Word>,
}

impl Shell {
    pub fn new() -> Self {
        let mut shell = Shell {
            dict: Builtin::default_bindings(),
            data: VecDeque::new(),
            code: Vec::new(),
            restore: Vec::new(),
        };

        shell.load(parse(STDLIB).unwrap().into_iter());
        shell.run().unwrap();

        shell
    }

    pub fn load<P: Iterator<Item=Word>>(&mut self, program: P) {
        self.code.extend(program);
    }

    pub fn run(&mut self) -> Result<(), EvalErr> {
        while let Some(word) = self.code.pop() {
            let name = match word {
                Word::Atom(name) => name,

                other => {
                    self.push(other);
                    continue;
                },
            };

            self.lookup(&name).and_then(|def| match def {
                Binding::Primitive(op) => self.do_builtin(op),

                Binding::Interpreted(typespec, word) => {
                    if self.data.len() < typespec.input {
                        return Err(EvalErr::StackUnderflow);
                    }

                    match word {
                        Word::List(words) => self.load(words.into_iter()),
                        other => self.code.push(other),
                    };

                    Ok(())
                }
            }).or_else(|err| {
                if let Some(env) = self.restore.pop() {
                    self.recover(env);
                    self.push(format!("{} error: {}", &name, &err));
                    Ok(())
                } else {
                    Err(err)
                }
            })?;
        }

        Ok(())
    }

    pub fn capture(&self) -> VecDeque<Word> {
        self.data.clone()
    }

    fn lookup(&self, name: &str) -> Result<Binding, EvalErr> {
        self.dict.get(name).cloned().ok_or_else(|| {
            EvalErr::CantUnderstand(name.to_owned())
        })
    }

    fn infer_type(&self, def: &VecDeque<Word>) -> Result<TypeSpec, EvalErr> {
        let mut spec = TypeSpec {
            input: 0,
            output: 0,
            exact: true,
        };

        for word in def.iter().rev() {
            if let Some(next) = self.get_type(word) {
                spec.merge(next)?;
            } else {
                spec.exact = false;
            }

            if !spec.exact { break; }
        }

        Ok(spec)
    }

    fn get_type(&self, word: &Word) -> Option<TypeSpec> {
        let name = match word {
            &Word::Atom(ref name) => name,
            _ => return Some(TypeSpec::literal()),
        };

        self.dict.get(name).map(|def| match def {
            &Binding::Primitive(prim) => prim.get_type(),
            &Binding::Interpreted(spec, _) => spec,
        })
    }

    fn recover(&mut self, env: Env) {
        self.dict = env.dict;
        self.code = env.code;
        self.data = env.data;
    }

    fn do_builtin(&mut self, builtin: Builtin) -> Result<(), EvalErr> {
        match builtin {
            Builtin::Bye => {
                self.code.clear();
            },

            Builtin::Assign => {
                let name = self.code.pop()
                    .ok_or(EvalErr::MacroFailed)?
                    .as_atom()?;

                let value = self.pop()?;

                let typespec = match &value {
                    &Word::List(ref items) => self.infer_type(items)?,
                    _ => TypeSpec::literal(),
                };

                self.dict.insert(name, {
                    Binding::Interpreted(typespec, value)
                });
            },

            Builtin::Eval => {
                match self.pop()? {
                    Word::List(words) => self.load(words.into_iter()),
                    other => self.push(other),
                }
            },

            Builtin::Expand => {
                let names = self.pop()?.as_list()?;
                let body = self.pop()?;

                let mut dict = OrderMap::new();
                for name in names.into_iter() {
                    dict.insert(name.as_atom()?, self.pop()?);
                }

                self.push(body.expand(&dict));
            },

            Builtin::If => {
                let test = self.pop()?.as_bool()?;
                let consequent = self.pop()?.as_list()?;
                let alternative = self.pop()?.as_list()?;

                if test {
                    self.load(consequent.into_iter());
                } else {
                    self.load(alternative.into_iter());
                }
            },

            Builtin::Try => {
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

            Builtin::PopEH => {
                self.restore.pop();
            },

            Builtin::Quote => {
                let word = self.code.pop().ok_or(EvalErr::MacroFailed)?;
                self.push(word);
            },

            Builtin::Explode => {
                let items = self.pop()?.as_list()?;
                self.data.extend(items.into_iter());
            },

            Builtin::Capture => {
                let capture = self.capture();
                self.push(capture);
            },

            Builtin::Debug => {
                for word in self.code.iter().rev() {
                    for line in word.pretty_print(0) {
                        println!("{}", line);
                    }
                }
            },

            Builtin::Inspect => {
                let name = self.pop()?.as_atom()?;
                let def = self.lookup(&name)?;

                match def {
                    Binding::Primitive(prim) => {
                        let spec = prim.get_type();
                        println!("{} {} = <BUILTIN>", &name, spec);
                    },

                    Binding::Interpreted(ref spec, ref def) => {
                        println!("{} {} =", &name, spec);
                        for line in def.pretty_print(0) {
                            println!("{}", line);
                        }
                    },
                }
            },

            Builtin::Len => {
                let len = self.pop()?.as_list()?.len();
                self.push(len as i32);
            },

            Builtin::Append => {
                let mut lhs = self.pop()?.as_list()?;
                let rhs = self.pop()?.as_list()?;
                lhs.extend(rhs.into_iter());
                self.push(lhs);
            },

            Builtin::Push => {
                let value = self.pop()?;
                let mut list = self.pop()?.as_list()?;
                list.push_back(value);
                self.push(list);
            },

            Builtin::Pop => {
                let mut list = self.pop()?.as_list()?;
                let value = list.pop_back().ok_or(EvalErr::EmptyList)?;
                self.push(list);
                self.push(value);
            },

            Builtin::Shift => {
                let mut list = self.pop()?.as_list()?;
                let value = list.pop_front().ok_or(EvalErr::EmptyList)?;
                self.push(list);
                self.push(value);
            },

            Builtin::Unshift => {
                let value = self.pop()?;
                let mut list = self.pop()?.as_list()?;
                list.push_front(value);
                self.push(list);
            },

            Builtin::Parse => {
                let source = self.pop()?.as_str()?;
                let program = parse(&source)?;
                self.push(program);
            },

            Builtin::Echo => {
                println!("{}", self.pop()?.into_string());
            },

            Builtin::Prompt => {
                use std::io::{stdin, stdout, Write};

                let text = self.pop()?.into_string();
                print!("{}", text);
                stdout().flush().unwrap();

                let mut inbuf = String::new();
                stdin().read_line(&mut inbuf).unwrap();
                inbuf.pop(); // Discard '\n'

                self.push(inbuf);
            },

            Builtin::Command => {
                use std::process::Command;

                let name = self.pop()?.into_string();
                let args = self.pop()?.into_list();

                let mut argv = Vec::with_capacity(args.len());

                for arg in args {
                    argv.push(arg.as_str()?);
                }

                let output = Command::new(&name)
                    .args(argv)
                    .output()
                    .unwrap();

                self.push({
                    String::from_utf8_lossy(&output.stdout).into_owned()
                });
            },

            Builtin::Load => {
                use std::fs::File;
                use std::io::Read;

                let path = self.pop()?.as_str()?;

                let mut inbuf = String::new();
                let mut file = File::open(&path).unwrap();
                file.read_to_string(&mut inbuf).unwrap();

                self.push(inbuf);
            },

            Builtin::Flatten => {
                let sep = self.pop()?.into_string();
                let list = self.pop()?.into_list();
                self.push(list.flatten(&sep));
            },

            Builtin::Pick => {
                let i = self.pop()?.into_hex()? as usize;
                let word = self.data.iter().nth(i).cloned()
                    .ok_or(EvalErr::StackUnderflow)?;
                self.push(word);
            },

            Builtin::Roll => {
                let i = self.pop()?.into_hex()? as usize;

                if let Some(word) = self.data.remove(i) {
                    self.push(word)
                } else {
                    return Err(EvalErr::StackUnderflow);
                }
            },

            Builtin::Drop => {
                let _ = self.pop()?;
            },

            Builtin::Clear => {
                self.data.clear();
            },

            Builtin::Strcat => {
                let mut lhs = self.pop()?.into_string();
                let rhs = self.pop()?.into_string();
                lhs.push_str(&rhs);
                self.push(lhs);
            },

            Builtin::Lines => {
                let string: String = self.pop()?.as_str()?;
                let mut words = VecDeque::new();

                for line in string.lines() {
                    words.push_back(Word::Str(line.into()));
                }

                self.push(words);
            },

            Builtin::Hex => {
                let hex = self.pop()?.into_hex()?;
                self.push(hex);
            },

            Builtin::Int => {
                let int = self.pop()?.into_int()?;
                self.push(int);
            },

            Builtin::OpAdd => {
                self.int_binop(|x, y| Ok(x + y))?;
            },

            Builtin::OpSub => {
                self.int_binop(|x, y| Ok(x - y))?;
            },

            Builtin::OpMul => {
                self.int_binop(|x, y| Ok(x * y))?;
            },

            Builtin::OpDiv => {
                self.int_binop(|x, y| x.checked_div(y).ok_or({
                    EvalErr::DivideByZero
                }))?;
            },

            Builtin::OpNeg => {
                let positive = self.pop()?.as_int()?;
                self.push(-positive);
            },

            Builtin::OpEql => {
                let lhs = self.pop()?;
                let rhs = self.pop()?;
                self.push(lhs == rhs);
            },

            Builtin::OpGt => {
                self.int_binop(|x, y| Ok(x > y))?;
            },

            Builtin::OpLt => {
                self.int_binop(|x, y| Ok(x < y))?;
            },

            Builtin::InfixExpr => {
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
        self.data.push_front(t.into());
    }

    fn pop(&mut self) -> Result<Word, EvalErr> {
        self.data.pop_front().ok_or(EvalErr::StackUnderflow)
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

impl From<OrderMap<String, Word>> for Word {
    fn from(dict: OrderMap<String, Word>) -> Self {
        Word::Dict(dict)
    }
}

impl PartialEq for Word {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&Word::Int(lhs), &Word::Int(rhs)) => lhs == rhs,
            (&Word::Hex(lhs), &Word::Hex(rhs)) => lhs == rhs,

            (&Word::Atom(ref lhs), &Word::Atom(ref rhs)) => lhs == rhs,
            (&Word::Str(ref lhs), &Word::Str(ref rhs)) => lhs == rhs,
            (&Word::List(ref lhs), &Word::List(ref rhs)) => lhs == rhs,

            (&Word::Dict(ref lhs), &Word::Dict(ref rhs)) => {
                for (k, v) in lhs.iter() {
                    if rhs.get(k) != Some(v) { return false; }
                }

                true
            },

            _ => false,
        }
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

    fn expand(self, dict: &OrderMap<String, Word>) -> Self {
        match self {
            Word::Atom(name) => if dict.contains_key(&name) {
                dict.get(&name).unwrap().clone()
            } else {
                Word::Atom(name)
            },

            Word::List(words) => Word::List({
                words.into_iter().map(|word| {
                    word.expand(dict)
                }).collect()
            }),

            other => other,
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

impl Flattenable for OrderMap<String, Word> {
    fn flatten(&self, sep: &str) -> String {
        self.iter().map(|(ref k, ref v)| {
            format!("{} = {}", k, v)
        }).collect::<Vec<_>>().join(sep)
    }
}

impl From<Builtin> for Binding {
    fn from(op: Builtin) -> Self {
        Binding::Primitive(op)
    }
}

impl Builtin {
    fn get_type(self) -> TypeSpec {
        let exact = |i, o| TypeSpec {
            input: i,
            output: o,
            exact: true,
        };

        let inexact = |i| TypeSpec {
            input: i,
            output: 0,
            exact: false,
        };

        use Builtin::*;

        match self {
            Bye => inexact(0),
            Assign => inexact(1),
            Eval => inexact(1),
            Expand => inexact(2),
            If => inexact(3),
            Try => inexact(2),
            PopEH => exact(0, 0),
            Quote => inexact(0),
            Explode => inexact(1),
            Capture => inexact(0),
            Debug => exact(0, 0),
            Inspect => exact(1, 0),
            Len => exact(1, 1),
            Append => exact(2, 1),
            Strcat => exact(2, 1),
            Push => exact(2, 1),
            Pop => exact(1, 2),
            Shift => exact(1, 2),
            Unshift => exact(2, 1),
            Parse => exact(1, 1),
            Echo => exact(1, 0),
            Prompt => exact(1, 1),
            Command => exact(2, 1),
            Load => exact(1, 1),
            Flatten => exact(2, 1),
            Pick => exact(2, 2),
            Roll => exact(2, 1),
            Drop => exact(1, 0),
            Clear => inexact(0),
            Lines => exact(1, 1),
            Hex => exact(1, 1),
            Int => exact(1, 1),
            OpAdd => exact(2, 1),
            OpDiv => exact(2, 1),
            OpSub => exact(2, 1),
            OpMul => exact(2, 1),
            OpNeg => exact(1, 1),
            OpEql => exact(2, 1),
            OpGt => exact(2, 1),
            OpLt => exact(2, 1),
            InfixExpr => inexact(0),
        }
    }
}

impl TypeSpec {
    fn literal() -> Self {
        TypeSpec {
            input: 0,
            output: 1,
            exact: true,
        }
    }

    fn merge(&mut self, rhs: Self) -> Result<(), EvalErr> {
        if self.output < rhs.input {
            self.input += rhs.input - self.output;
            self.output = 0;
        } else {
            self.output -= rhs.input;
        }

        self.output += rhs.output;
        self.exact = self.exact && rhs.exact;

        Ok(())
    }
}

macro_rules! order_map {
    ( $( $k:expr => $v:expr ,)* ) => {{
        let mut _hash_map = ::ordermap::OrderMap::new();
        $( _hash_map.insert($k.into(), $v.into()); )*
        _hash_map
    }};
}

impl Builtin {
    fn default_bindings() -> OrderMap<String, Binding> {
        use Builtin::*;

        order_map![
            "bye" => Bye,
            "=" => Assign,
            "eval" => Eval,
            "expand" => Expand,
            "if" => If,
            "try" => Try,
            "popeh" => PopEH,
            "quote" => Quote,
            "explode" => Explode,
            "capture" => Capture,
            "debug" => Debug,
            "inspect" => Inspect,
            "len" => Len,
            "append" => Append,
            "push" => Push,
            "pop" => Pop,
            "shift" => Shift,
            "unshift" => Unshift,
            "parse" => Parse,
            "echo" => Echo,
            "prompt" => Prompt,
            "command" => Command,
            "load" => Load,
            "flatten" => Flatten,
            "pick" => Pick,
            "roll" => Roll,
            "drop" => Drop,
            "clear" => Clear,
            "strcat" => Strcat,
            "lines" => Lines,
            "hex" => Hex,
            "int" => Int,
            "+" => OpAdd,
            "-" => OpSub,
            "*" => OpMul,
            "/" => OpDiv,
            "~" => OpNeg,
            "==" => OpEql,
            "<" => OpLt,
            ">" => OpGt,
            "))" => InfixExpr,
        ]
    }
}
