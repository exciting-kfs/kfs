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
	let mut func_impl = parse_macro_input!(input as ItemFn);

	// backup some stuff about func_impl for making outer function.
	let ident = func_impl.sig.ident.clone();
	let vis = func_impl.vis.clone();
	let param = func_impl.sig.inputs.clone();
	let ret = func_impl.sig.output.clone();
	let abi = func_impl.sig.abi.clone();
	let unsafety = func_impl.sig.unsafety.clone();
	let asyncness = func_impl.sig.asyncness.clone();
	let constness = func_impl.sig.constness.clone();
	let generics = func_impl.sig.generics.clone();


	let mut call_param = quote!();
	func_impl.sig.inputs.clone().into_iter().for_each(|arg| {
		call_param.extend(match arg {
			FnArg::Receiver(_) => quote!(),
			FnArg::Typed(pat_type) => {
				let pat = pat_type.pat;
				quote!(#pat,)
			}
		})
	});

	// edit name and restrict visibility.
	let impl_name = format!("__inner_{}_impl", func_impl.sig.ident.to_string());
	func_impl.sig.ident = format_ident!("{}", impl_name);
	func_impl.vis = Visibility::Inherited; // private
	let call_impl = func_impl.sig.ident.clone();
	let call_impl = func_impl.sig.inputs.first().and_then(
		|s| match s {
			FnArg::Receiver(r) => Some(r),
			_ => None
		}).map_or(quote!(#call_impl), |_| quote!(self.#call_impl));


	let attr = attr.to_string();
	let to_context = match attr.as_str() {
		"nmi" => quote!(InContext::NMI),
		"hw_irq" => quote!(InContext::HwIrq),
		"kernel" => quote!(InContext::Kernel),
		"irq_disabled" => quote!(InContext::IrqDisabled),
		"preempt_disabled" => quote!(InContext::PreemptDisabled),
		_ => panic!("kfs_macro: context: invalid context"),
	};

	let no_mangle = match attr.as_str() {
		"nmi" | "hw_irq" => quote!(#[no_mangle]),
		"kernel" | "irq_disabled" | "preempt_disabled" => quote!(),
		_ => panic!("kfs_macro: context: invalid context"),
	};

	let new_func = quote! {
		#[inline(always)]
		#func_impl

		#no_mangle
		#vis #constness #asyncness #unsafety #abi fn #ident #generics (#param) #ret {
			use crate::process::context::InContext;
			let backup = crate::process::context::context_switch(#to_context);
			let ret = #call_impl(#call_param);
			crate::process::context::context_switch(backup);
			ret
		}
	};
	TokenStream::from(new_func)
}
