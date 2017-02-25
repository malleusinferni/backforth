extern crate backforth;

fn main() {
    use std::io::{Read, Write, stdin, stdout};

    let mut env = backforth::Env::new();
    let mut inbuf = String::new();

    loop {
        print!("> ");
        stdout().flush();

        stdin().read_line(&mut inbuf).unwrap();

        let program = match backforth::parse(&inbuf) {
            Ok(program) => program,
            Err(err) => {
                println!("parse error");
                continue;
            },
        };

        match env.run(program) {
            Ok(result) => println!("\t-> {}", flatten(result)),
            Err(err) => println!("error"),
        }

        inbuf.clear();
    }
}

fn flatten(result: Vec<backforth::Value>) -> String {
    result.into_iter().map(|val| {
        format!("{}", val)
    }).collect::<Vec<_>>().join(" ")
}
