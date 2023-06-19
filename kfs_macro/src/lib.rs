use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, ReturnType};

#[proc_macro_attribute]
pub fn ktest(attr: TokenStream, input: TokenStream) -> TokenStream {
	let attr = attr.to_string();

	let func = parse_macro_input!(input as ItemFn);

	let sig = &func.sig;
	let ident = &sig.ident;

	let input = &sig.inputs;
	let output = &sig.output;
	let is_output_unit = match output {
		ReturnType::Default => true,
		_ => false,
	};

	if !input.is_empty() || !is_output_unit {
		panic!("The type of test function must be `fn()`");
	}

	let func_name = ident.to_string();
	let func_full_name = quote!(concat!(module_path!(), "::", #func_name));
	let static_name = format_ident!("__TEST_CASE_{}", func_name.to_uppercase());

	let expanded = /*if attr == "develop"*/ {
		quote! {
			#[cfg(any(ktest = "develop", ktest = "maintain"))]
			#[cfg(ktest)]
			#[link_section = ".test_array"]
			static #static_name: crate::test::TestCase = crate::test::TestCase::new(
				#func_full_name,
				#ident,
			);
			#func
		}
	};
	//  else {
	// 	quote! {
	// 		#[cfg(ktest = "maintain")]
	// 		#[link_section = ".test_array"]
	// 		static #static_name: crate::test::TestCase = crate::test::TestCase::new(
	// 			#func_full_name,
	// 			#ident,
	// 		);
	// 		#func
	// 	}
	// };

	TokenStream::from(expanded)
}
