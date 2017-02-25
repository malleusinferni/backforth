extern crate backforth;

fn main() {
    use std::io::{Read, Write, stdin, stdout};

    use backforth::Flattenable;

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
            Ok(result) => println!("\t-> {}", result.flatten(" ")),
            Err(err) => println!("error"),
        }

        inbuf.clear();
    }
}
