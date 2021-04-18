//! Proc macro to match regexes in const fns. The regex must be a string literal, but the bytes
//! matched can be any value.
//!
//! The macro expects an `&[u8]`, but you can easily use `str::as_bytes`.
//!
//! ```
//! const fn this_crate(bytes: &[u8]) -> bool {
//!     const_regex::match_regex!("^(meta-)*regex matching", bytes)
//! }
//!
//! assert!(this_crate(b"meta-meta-regex matching"));
//! assert!(!this_crate(b"a good idea"));
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use regex_automata::{dense, DFA};
use std::collections::{BTreeSet, HashMap};
use std::ops::RangeInclusive;
use syn::{parse::*, *};

type RegexDfa = dense::Standard<Vec<usize>, usize>;

#[derive(Clone, PartialEq)]
enum State {
    Match,
    Dead,
    Transitions(HashMap<usize, BTreeSet<u8>>),
}

fn range_to_tokens(range: RangeInclusive<u8>) -> TokenStream {
    let (start, end) = range.into_inner();
    if start == end {
        quote!(#start)
    } else {
        quote!(#start..=#end)
    }
}

impl State {
    fn from_regex(regex: &RegexDfa, state: usize) -> Self {
        if regex.is_match_state(state) {
            Self::Match
        } else if regex.is_dead_state(state) {
            Self::Dead
        } else {
            let mut transitions = HashMap::new();

            for byte in 0..=255 {
                let next = regex.next_state(state, byte);
                transitions
                    .entry(next)
                    .or_insert_with(BTreeSet::new)
                    .insert(byte);
            }

            Self::Transitions(transitions)
        }
    }

    fn handle(&self, byte: &Ident, states: &HashMap<usize, State>) -> Expr {
        match self {
            Self::Match => parse_quote!(return true),
            Self::Dead => parse_quote!(return false),
            Self::Transitions(transitions) => {
                let branches = transitions.iter().map(|(target, bytes)| {
                    let mut ranges = vec![];
                    let mut range: Option<RangeInclusive<u8>> = None;
                    for &byte in bytes {
                        if let Some(range) = &mut range {
                            if *range.end() == byte - 1 {
                                *range = *range.start()..=byte;
                                continue;
                            } else {
                                ranges.push(range_to_tokens(range.clone()));
                            }
                        }
                        range = Some(byte..=byte);
                    }

                    if let Some(range) = range {
                        ranges.push(range_to_tokens(range));
                    }

                    let handler = match states[target] {
                        Self::Match => quote!(return true),
                        Self::Dead => quote!(return false),
                        _ => quote!(#target),
                    };

                    quote!(#(#ranges)|* => #handler)
                });

                parse_quote! {
                    match #byte {
                        #(#branches),*
                    }
                }
            }
        }
    }
}

struct Dfa {
    start: usize,
    states: HashMap<usize, State>,
}

impl Dfa {
    fn add_states(&mut self, regex: &RegexDfa, id: usize) {
        let state = State::from_regex(regex, id);

        self.states.insert(id, state.clone());

        if let State::Transitions(transitions) = &state {
            for target in transitions.keys() {
                if !self.states.contains_key(target) {
                    self.add_states(regex, *target);
                }
            }
        }
    }

    fn from_regex(regex: &RegexDfa) -> Self {
        let start = regex.start_state();
        let mut dfa = Self {
            start,
            states: HashMap::new(),
        };

        dfa.add_states(regex, start);

        dfa
    }

    fn handle(&self, input: &Ident) -> Expr {
        let byte = parse_quote!(byte);
        let start = self.start;

        let branches = self.states.iter().map(|(id, state)| {
            let body = state.handle(&byte, &self.states);
            quote!(#id => #body)
        });

        parse_quote! {{
            let mut i = 0;
            let mut state = #start;

            while i < #input.len() {
                let #byte = #input[i];

                state = match state {
                    #(#branches,)*
                    #[allow(unconditional_panic)]
                    _ => [][0],
                };

                i += 1;
            }

            return false;
        }}
    }
}

fn build_dfa(regex: &str) -> RegexDfa {
    let (regex, anchored) = if let Some(regex) = regex.strip_prefix('^') {
        (regex, true)
    } else {
        (regex, false)
    };

    let dfa = dense::Builder::new()
        .byte_classes(false)
        .premultiply(false)
        .minimize(true)
        .anchored(anchored)
        .build(regex)
        .unwrap();

    if let dense::DenseDFA::Standard(dfa) = dfa {
        dfa
    } else {
        unreachable!()
    }
}

struct Args {
    regex: String,
    expr: Expr,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let regex_lit: LitStr = input.parse()?;
        let _comma_token: Token![,] = input.parse()?;
        let expr = input.parse()?;

        Ok(Self {
            regex: regex_lit.value(),
            expr,
        })
    }
}

/// See crate documentation.
#[proc_macro]
pub fn match_regex(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as Args);
    let regex = build_dfa(&args.regex);
    let dfa = Dfa::from_regex(&regex);
    let input_token = parse_quote!(input);
    let block = dfa.handle(&input_token);
    let input_expr = args.expr;

    let tokens = quote! {{
        const fn match_regex(#input_token: &[u8]) -> bool {
            #block
        }

        match_regex(#input_expr)
    }};

    tokens.into()
}
