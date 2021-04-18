use proc_macro2::TokenStream;
use quote::quote;
use regex_automata::{dense, DFA};
use std::collections::{BTreeSet, HashMap};
use std::ops::RangeInclusive;
use syn::*;

type RegexDfa = dense::Standard<Vec<usize>, usize>;

#[derive(Clone)]
pub enum State {
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
    pub fn from_regex(regex: &RegexDfa, state: usize) -> Self {
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

    pub fn handle(&self, byte: &Ident) -> Expr {
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

                    quote!(#(#ranges)|* => #target)
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

pub struct Dfa {
    pub start: usize,
    pub states: HashMap<usize, State>,
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

    pub fn from_regex(regex: &RegexDfa) -> Self {
        let start = regex.start_state();
        let mut dfa = Self {
            start,
            states: HashMap::new(),
        };

        dfa.add_states(regex, start);

        dfa
    }

    pub fn handle(&self, input: Ident) -> Expr {
        let byte = parse_quote!(byte);
        let start = self.start;

        let branches = self.states.iter().map(|(id, state)| {
            let body = state.handle(&byte);
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

pub fn build_dfa(regex: &str) -> RegexDfa {
    let dfa = dense::Builder::new()
        .byte_classes(false)
        .premultiply(false)
        .minimize(true)
        .build(regex)
        .unwrap();

    if let dense::DenseDFA::Standard(dfa) = dfa {
        dfa
    } else {
        unreachable!()
    }
}

/*#[proc_macro]
pub fn const_regex(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let _input: TokenStream = input.into();
    todo!()
}*/
