use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, ReturnType, Visibility};

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
	let mut inner = parse_macro_input!(input as ItemFn);

	// backup some stuff about inner function for making outer function.
	let ident = inner.sig.ident.clone();
	let vis = inner.vis.clone();
	let param = inner.sig.inputs.clone();
	let ret = inner.sig.output.clone();
	let abi = inner.sig.abi.clone();
	let unsafety = inner.sig.unsafety.clone();

	// add prefix '__inner_' and restrict visibility of inner function.
	let inner_name = format!("__inner_{}", inner.sig.ident.to_string());
	inner.sig.ident = format_ident!("{}", inner_name);
	inner.vis = Visibility::Inherited; // private
	let call_inner = inner.sig.ident.clone();

	let mut inner_param = quote!();
	inner.sig.inputs.clone().into_iter().for_each(|arg| {
		inner_param.extend(match arg {
			FnArg::Receiver(_) => panic!("kfs_macro: context: not supported"),
			FnArg::Typed(pat_type) => {
				let pat = pat_type.pat;
				quote!(#pat,)
			}
		})
	});

	let attr = attr.to_string();
	let to_context = match attr.as_str() {
		"nmi" => quote!(InContext::NMI),
		"hw_irq" => quote!(InContext::HwIrq),
		"kernel" => quote!(InContext::Kernel),
		"irq_disabled" => quote!(InContext::IrqDisabled),
		_ => panic!("kfs_macro: context: invalid context"),
	};

	let no_mangle = match attr.as_str() {
		"nmi" | "hw_irq" => quote!(#[no_mangle]),
		"kernel" => quote!(),
		"irq_disabled" => quote!(),
		_ => panic!("kfs_macro: context: invalid context"),
	};

	let new_func = quote! {
		#no_mangle
		#vis #unsafety #abi fn #ident(#param) #ret {
			#inner

			let backup = context_switch(#to_context);
			let ret = #call_inner(#inner_param);
			context_switch(backup);
			ret
		}
	};
	TokenStream::from(new_func)
}
