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

	let mut config = if !attr.is_empty() {
		quote!(#[cfg(any(ktest = "all", ktest = #attr))])
	} else {
		quote!(#[cfg(ktest = "all")])
	};

	let test = quote! {
		#[link_section = ".test_array"]
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
pub fn context(attr: TokenStream, input: TokenStream) -> TokenStream {
	let func_impl = parse_macro_input!(input as ItemFn);
	let sig = func_impl.sig.clone();
	let vis = func_impl.vis.clone();
	let block = func_impl.block.clone();

	let attr = attr.to_string();
	let no_mangle = match attr.as_str() {
		"nmi" | "hw_irq" => quote!(#[no_mangle]),
		"kernel" | "irq_disabled" | "preempt_disabled" => quote!(),
		_ => panic!("kfs_macro: context: invalid context"),
	};

	let to_context = match attr.as_str() {
		"nmi" => quote!(InContext::NMI),
		"hw_irq" => quote!(InContext::HwIrq),
		"kernel" => quote!(InContext::Kernel),
		"irq_disabled" => quote!(InContext::IrqDisabled),
		"preempt_disabled" => quote!(InContext::PreemptDisabled),
		_ => panic!("kfs_macro: context: invalid context"),
	};

	let new_func = quote! {
		#no_mangle
		#vis #sig {
			use crate::process::context::InContext;
			let __original_context = crate::process::context::context_switch_auto(#to_context);

			let __ret = #block;

			__ret
		}
	};

	TokenStream::from(new_func)
}
