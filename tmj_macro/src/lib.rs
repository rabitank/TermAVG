use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Expr, Ident, Lit, Meta, MetaNameValue};

/// 过程宏：根据全大写的标识符生成一个同名的字符串常量（值为全小写）
/// 用法：const_script_str!(FADE_IN);
/// 展开为：
///     pub const FADE_IN: &str = "fade_in";
#[proc_macro]
pub fn lower_str(input: TokenStream) -> TokenStream {
    // 解析输入为一个标识符
    let ident = parse_macro_input!(input as Ident);

    // 将标识符转换为小写字符串
    let lower = ident.to_string().to_lowercase();

    // 生成常量定义
    let expanded = quote! {
            pub const #ident: &str = #lower;
    inventory::submit! {
                crate::utils::ConstInfo {
                    module: module_path!(),
                    value: #lower,
                }
            }
        }; // 更好的方式：在生成的代码中调用 module_path!()，因为它在调用点展开

    expanded.into()
}

/// 自动实现 Typename 特征
/// 用法：
// 使用默认名称（类型名小写）
///```
///#[derive(TypeName)]
///struct MyStruct;
///
///// 自定义名称
///#[derive(TypeName)]
///#[type_name = "custom_name"]
///struct AnotherStruct;
///```
///
#[proc_macro_derive(TypeName, attributes(type_name))]
pub fn derive_type_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // 查找 #[type_name = "custom"] 属性
    let type_name_str = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("type_name"))
        .and_then(|attr| {
            if let Meta::NameValue(MetaNameValue { value, .. }) = &attr.meta {
                if let Expr::Lit(expr_lit) = value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
            None
        })
        .unwrap_or_else(|| name.to_string().to_lowercase()); // 默认：类型名小写

    let expanded = quote! {
        impl #impl_generics TypeName for #name #ty_generics #where_clause {
            const TYPE_NAME: &'static str = #type_name_str;
        }
    };

    TokenStream::from(expanded)
}
