use crate::utils::{parse_struct_fields, parse_target_type, Field};
use proc_macro::TokenStream;
use quote::quote;

pub fn impl_asrust_macro(input: &syn::DeriveInput) -> TokenStream {
    let struct_name = &input.ident;
    let target_type = parse_target_type(&input.attrs);

    let fields = parse_struct_fields(&input.data)
        .iter()
        .map(|field| {
            let Field {
                name: field_name,
                ref field_type,
                ..
            } = field;

            if field.levels_of_indirection > 1 && !field.is_nullable {
                panic!(format!("The CReprOf, AsRust, and CDrop traits cannot be derived automatically : The field {} is a pointer field has too many levels of indirection ({} in this case).\
                \nPlease implements those traits manually.", field_name, field.levels_of_indirection))
            }

            let mut conversion = if field.is_string {
                quote!( {
                    use ffi_convert::RawBorrow;
                    unsafe { std::ffi::CStr::raw_borrow(self.#field_name) }?.as_rust()?
                })
            } else {
                if field.is_pointer {
                    quote!( {
                            let ref_to_struct = unsafe { #field_type::raw_borrow(self.#field_name)? };
                            let converted_struct = ref_to_struct.as_rust()?;
                            converted_struct
                        }
                    )
                } else {
                    quote!(self.#field_name.as_rust()?)
                }
            };

            conversion = if field.is_nullable {
                quote!(
                    #field_name: if !self.#field_name.is_null() {
                        Some(#conversion)
                    } else {
                        None
                    }
                )
            } else {
                quote!(
                    #field_name: #conversion
                )
            };
            conversion
        })
        .collect::<Vec<_>>();

    quote!(
        impl AsRust<#target_type> for #struct_name {
            fn as_rust(&self) -> Result<#target_type, ffi_convert::AsRustError> {
                Ok(#target_type {
                    #(#fields, )*
                })
            }
        }
    )
    .into()
}
