//use proc_macro2::TokenStream;
use quote::quote;
use regex_automata::{dense, DFA};
use std::collections::HashMap;
use syn::*;

type RegexDfa = dense::Standard<Vec<usize>, usize>;

#[derive(Clone)]
pub enum State {
    Match,
    Dead,
    Transitions(HashMap<usize, Vec<u8>>),
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
                transitions.entry(next).or_insert_with(Vec::new).push(byte);
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
                    quote! {
                        #(#bytes)|* => #target
                    }
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
                    _ => unreachable!()
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
