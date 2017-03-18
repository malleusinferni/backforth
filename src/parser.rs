use super::Word;

pub type Program = Vec<Word>;

#[derive(Copy, Clone, Debug)]
pub enum ParseErr {
    MissingOpenBrace,
    MissingCloseBrace,
    MissingEndQuote,
    BadHexLiteral,
}

pub fn parse(input: &str) -> Result<Program, ParseErr> {
    let mut stream = input.chars().peekable();
    let mut stack = Stack::with_capacity(8);
    stack.push();

    while let Some(ch) = stream.next() {
        match ch {
            '{' => stack.push(),

            '}' => stack.pop()?,

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
                let mut prev = w;
                let mut word = String::new();
                word.push(w);
                while let Some(&ch) = stream.peek() {
                    if word_break(prev, ch) {
                        break;
                    }

                    prev = ch;
                    word.extend(stream.next());
                }

                if &word == "#" || word.starts_with("#!") {
                    loop {
                        match stream.next() {
                            Some('\n') | None => break,
                            _ => continue,
                        }
                    }
                } else if word.starts_with('#') {
                    word.drain(0 .. 1);
                    stack.emit(Word::Hex(parse_hex(word)?))?;
                } else if let Ok(int) = word.parse::<i32>() {
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
        self.emit(Word::List(list.into()))
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

fn parse_hex(word: String) -> Result<u32, ParseErr> {
    //if word.len() == 3 || word.len() == 4 {
    //    let mut longer = String::with_capacity(word.len() * 2);
    //    for ch in word.chars() {
    //        longer.push(ch);
    //        longer.push(ch);
    //    }
    //    word = longer;
    //}

    u32::from_str_radix(&word, 16).map_err(|_| ParseErr::BadHexLiteral)
}

fn word_break(a: char, b: char) -> bool {
    fn is_delim(ch: char) -> bool {
        match ch {
            '{' | ';' | '}' => true,
            _ => false,
        }
    }

    match (a, b) {
        (_, s) if s.is_whitespace() => true,
        (_, s) if is_delim(s) => true,
        ('=', '=') => false,
        ('=', _) => true,
        (_, '=') => true,
        _ => false,
    }
}

#[test]
fn funky_word_breaks() {
    let inputs = vec![
        ("k= 1", "k = 1"),
        ("{}{}{}", "{ } { } { }"),
        ("{+ 1 2}", "{ + 1 2 }"),
        ("foo;bar;baz", "baz bar foo"),
    ];

    for (left, right) in inputs {
        let left: Word = parse(left).unwrap().into();
        let right: Word = parse(right).unwrap().into();

        if left != right {
            panic!("{} parsed as {:?}", &left, &left);
        }
    }
}
