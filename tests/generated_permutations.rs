mod common;
use common::{assert_pair, assert_exported};

// Macro to generate tests for single chars as keys
macro_rules! tests_for_chars {
    ($($char:ident),*) => {
        paste::paste! {
            $(
                #[test]
                #[allow(non_snake_case)]
                fn [<test_key_char_ $char>]() {
                    let c = stringify!($char);
                    let input = format!("{}=v", c);
                    assert_pair(&input, c, "v");
                }
            )*
        }
    }
}

tests_for_chars!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z
);

// 0-9 for value content
macro_rules! tests_for_val_digits {
    ($($d:literal),*) => {
        paste::paste! {
            $(
                #[test]
                fn [<test_val_digit_ $d>]() {
                    let s = stringify!($d);
                    let input = format!("K={}", s);
                    assert_pair(&input, "K", s);
                }
            )*
        }
    }
}

tests_for_val_digits!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9);

// Special chars
macro_rules! tests_for_specials {
    ($name:ident, $sym:expr) => {
        paste::paste! {
            #[test]
            fn [<test_special_single_ $name>]() {
                 let input = format!("K='{}'", $sym);
                 assert_pair(&input, "K", $sym);
            }
            #[test]
            fn [<test_special_double_ $name>]() {
                 let input = format!("K=\"{}\"", $sym);
                 assert_pair(&input, "K", $sym);
            }
        }
    }
}

tests_for_specials!(exclamation, "!");
tests_for_specials!(at, "@");
tests_for_specials!(hash, "#");
tests_for_specials!(percent, "%");
tests_for_specials!(caret, "^");
tests_for_specials!(ampersand, "&");
tests_for_specials!(asterisk, "*");
tests_for_specials!(lparen, "(");
tests_for_specials!(rparen, ")");
tests_for_specials!(minus, "-");
tests_for_specials!(plus, "+");
tests_for_specials!(equals, "=");
tests_for_specials!(lbracket, "[");
tests_for_specials!(rbracket, "]");
tests_for_specials!(lbrace, "{");
tests_for_specials!(rbrace, "}");
tests_for_specials!(pipe, "|");
tests_for_specials!(colon, ":");
tests_for_specials!(semicolon, ";");
tests_for_specials!(less, "<");
tests_for_specials!(greater, ">");
tests_for_specials!(comma, ",");
tests_for_specials!(dot, ".");
tests_for_specials!(question, "?");
tests_for_specials!(slash, "/");
tests_for_specials!(tilde, "~");
tests_for_specials!(backtick, "`");

// Whitespace padding variations
macro_rules! gen_padding_tests {
    ($($n:literal),*) => {
        paste::paste! {
            $(
                #[test]
                fn [<test_pad_key_ $n>]() {
                    let pad = " ".repeat($n);
                    let input = format!("{}K=v", pad);
                    assert_pair(&input, "K", "v");
                }
                #[test]
                fn [<test_pad_val_ $n>]() {
                    let pad = " ".repeat($n);
                    let input = format!("K=v{}", pad);
                    assert_pair(&input, "K", "v");
                }
                #[test]
                fn [<test_pad_export_ $n>]() {
                    let pad = " ".repeat($n);
                    let input = format!("{}export K=v", pad);
                    assert_exported(&input, "K", "v");
                }
            )*
        }
    }
}

gen_padding_tests!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20);
gen_padding_tests!(21, 22, 23, 24, 25, 26, 27, 28, 29, 30);

// Duplicate key counts
macro_rules! gen_duplicate_tests {
    ($($n:literal),*) => {
        paste::paste! {
            $(
                #[test]
                fn [<test_dup_ $n>]() {
                    let mut input = String::new();
                    for i in 0..$n {
                        input.push_str(&format!("K=v{}\n", i));
                    }
                    let entries = korni::parse(&input);
                    assert_eq!(entries.len(), $n); 
                }
            )*
        }
    }
}

gen_duplicate_tests!(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30);

// Inline comment padding
macro_rules! gen_comment_pad_tests {
    ($($n:literal),*) => {
        paste::paste! {
            $(
                #[test]
                fn [<test_comment_pad_ $n>]() {
                    let pad = " ".repeat($n);
                    let input = format!("K=v{}# comment", pad);
                    assert_pair(&input, "K", "v");
                }
            )*
        }
    }
}
gen_comment_pad_tests!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20);

// Quoted Empty Padding
macro_rules! gen_quoted_pad_tests {
    ($($n:literal),*) => {
        paste::paste! {
            $(
                #[test]
                fn [<test_quoted_pad_ $n>]() {
                    let pad = " ".repeat($n);
                    let input = format!("K=\"{}\"", pad);
                    assert_pair(&input, "K", &pad);
                }
            )*
        }
    }
}
gen_quoted_pad_tests!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19);

// Mixed case keys
macro_rules! tests_for_mixed_case {
    ($($char:ident),*) => {
        paste::paste! {
            $(
                #[test]
                #[allow(non_snake_case)]
                fn [<test_mixed_key_ $char>]() {
                    let c = stringify!($char); 
                    let input = format!("A{}z=v", c); // A<char>z
                    assert_pair(&input, &format!("A{}z", c), "v");
                }
            )*
        }
    }
}
tests_for_mixed_case!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
