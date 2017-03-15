extern crate backforth;

use backforth::*;

macro_rules! valid {
    ( $name:ident, $( $value:expr ),* ) => {
        #[test]
        fn $name() {
            let source = {
                include_str!(concat!("valid/", stringify!($name), ".b4"))
            };

            let mut env = Shell::new();
            env.load(parse(&source).unwrap().into_iter());
            env.run().unwrap();
            assert_eq!(env.view(), &[ $( $value ),* ]);
        }
    };
}

valid!(factorial, Word::Int(120));
