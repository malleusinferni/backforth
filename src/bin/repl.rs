extern crate backforth;

fn main() {
    use std::io::{Write, stdin, stdout};

    use backforth::Flattenable;

    let mut env = backforth::Env::new();
    let mut inbuf = String::new();

    let bye = backforth::parse("bye").unwrap();

    loop {
        inbuf.clear();

        print!("> ");
        stdout().flush().unwrap();

        stdin().read_line(&mut inbuf).unwrap();

        let program = match backforth::parse(&inbuf) {
            Ok(program) => program,
            Err(err) => {
                println!("{}", err);
                continue;
            },
        };

        if program == bye { return; }

        match env.run(program) {
            Ok(result) => if result.len() > 0 {
                println!("\t-> {}", result.flatten(" "));
            },

            Err(err) => println!("{}", err),
        }
    }
}
