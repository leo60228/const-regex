use const_regex::*;
use quote::ToTokens;
use syn::parse_quote;

fn main() {
    let regex = build_dfa("m | [tn]|b");
    let dfa = Dfa::from_regex(&regex);
    let ast = dfa.handle(parse_quote!(input));
    println!("{}", ast.into_token_stream());
}
