use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Nothing, parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn ktest(attr: TokenStream, input: TokenStream) -> TokenStream {
	parse_macro_input!(attr as Nothing);

	let test_func = parse_macro_input!(input as ItemFn);

	let ident = &test_func.sig.ident;
	let name = ident.to_string();
	let static_name = format_ident!("__TEST_CASE_{}", name.to_uppercase());

	let expanded = quote! {
		#[cfg(ktest)]
		#[link_section = ".test_array"]
		static #static_name: crate::test::TestCase = crate::test::TestCase::new(
			concat!(module_path!(), "::", #name),
			#ident,
		);
		#test_func
	};

	TokenStream::from(expanded)
}
