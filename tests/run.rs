extern crate backforth;

use backforth::*;

macro_rules! valid {
    ( $name:ident $(, $value:expr )* ) => {
        #[test]
        fn $name() {
            let source = {
                include_str!(concat!("valid/", stringify!($name), ".\\iv"))
            };

            let mut env = Shell::new();
            env.load(parse(&source).unwrap().into_iter());
            env.run().unwrap();
            assert_eq!(env.view(), &[ $( $value ),* ]);
        }
    };
}

macro_rules! invalid {
    ( $name:ident ) => {
        #[test]
        #[should_panic]
        fn $name() {
            let source = {
                include_str!(concat!("invalid/", stringify!($name), ".\\iv"))
            };

            let mut env = Shell::new();
            env.load(parse(&source).unwrap().into_iter());
            env.run().unwrap();
        }
    };
}

valid!(hello);
valid!(factorial, Word::Int(120));
valid!(countdown, Word::Int(0));

invalid!(divide_by_zero);
