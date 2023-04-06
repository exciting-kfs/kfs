extern crate alloc;

use alloc::{format, string::ToString};
use proc_macro::{TokenStream};
use quote::quote;
use syn::{parse_macro_input, ItemFn, Ident};

const PREFIX: &'static str = "kernel_test";

#[proc_macro_attribute]
pub fn kernel_test(attr: TokenStream, input: TokenStream) -> TokenStream {
	let mut input_fn  = parse_macro_input!(input as ItemFn);
	let attr = attr.to_string();

	let prefix = match attr.is_empty() {
		true => format!("{}_", PREFIX),
		false => format!("{}_{}_", PREFIX, attr),
	};

	let new_name = format!("{}{}", prefix, input_fn.sig.ident.to_string());
	input_fn.sig.ident = Ident::new(&new_name, input_fn.sig.ident.span());

	let expanded = quote!{
		#[no_mangle]
		#input_fn
	};

	let output = TokenStream::from(expanded);
	output
}