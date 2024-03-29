use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, ItemFn, ReturnType};

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

	let mut config = if !attr.is_empty() {
		quote!(#[cfg(any(ktest = "all", ktest = #attr))])
	} else {
		quote!(#[cfg(ktest = "all")])
	};

	let test = quote! {
		#[link_section = ".test_array"]
		#[used]
		static #static_name: crate::test::TestCase = crate::test::TestCase::new(
			#func_full_name,
			#ident,
		);
		#func
	};

	config.extend(test);
	TokenStream::from(config)
}

#[proc_macro_attribute]
pub fn interrupt_handler(_attr: TokenStream, input: TokenStream) -> TokenStream {
	let func_impl = parse_macro_input!(input as ItemFn);

	let sig = func_impl.sig.clone();
	let vis = func_impl.vis.clone();
	let block = func_impl.block.clone();

	quote! {
		#[no_mangle]
		#vis #sig {
			assert_eq!(crate::sync::get_lock_depth(), 0);
			let __interrupt_guard = crate::interrupt::enter_interrupt_context();
			#block;
		}
	}
	.into()
}

#[proc_macro_attribute]
pub fn log_time(attr: TokenStream, input: TokenStream) -> TokenStream {
	let func_impl = parse_macro_input!(input as ItemFn);

	let sig = func_impl.sig.clone();
	let name = sig.ident.clone().to_string();

	let vis = func_impl.vis.clone();
	let block = func_impl.block.clone();

	let feature_name = format!("time-{}", attr.to_string());
	let feature_name = syn::LitStr::new(&feature_name, sig.span());

	quote! {
		#vis #sig {
			let log_time_start = crate::driver::hpet::get_timestamp_micro();
			let ret = #block;
			let log_time_end = crate::driver::hpet::get_timestamp_micro();

			crate::trace_feature!(#feature_name, "{}: {} us", #name, log_time_end - log_time_start);

			ret
		}
	}
	.into()
}
