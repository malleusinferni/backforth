extern crate backforth;

use backforth::*;

fn run_program(source: &str) -> Vec<Word> {
    let mut env = Shell::new();
    env.load(parse(&source).unwrap().into_iter());
    env.run().unwrap();
    env.capture().into()
}

macro_rules! sourcify {
    ( $dir:tt, $name:ident ) => {
        include_str!(concat!($dir, "/", stringify!($name), ".\\iv"))
    }
}

macro_rules! valid {
    ( $name:ident $(, $value:expr )* ) => {
        #[test]
        fn $name() {
            let result = run_program(sourcify!("valid", $name));
            assert_eq!(&result, &[ $( $value ),* ]);
        }
    };
}

macro_rules! invalid {
    ( $name:ident ) => {
        #[test]
        #[should_panic]
        fn $name() {
            let _ = run_program(sourcify!("invalid", $name));
        }
    };
}

valid!(hello);
valid!(factorial, Word::Int(120));
valid!(countdown, Word::Int(0));

invalid!(divide_by_zero);
