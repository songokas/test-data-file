use std::path::Path;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::parse::Result;
use syn::{meta::ParseNestedMeta, parse_macro_input, FnArg, ItemFn, LitStr, Pat};

const SUPPORTED_KINDS: [&str; 6] = ["csv", "json", "yaml", "ron", "toml", "list"];

#[proc_macro_attribute]
pub fn test_file_data(args: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the function input and attributes
    let mut func = parse_macro_input!(item as ItemFn);
    let mut attrs = TestFileDataAttributes::default();

    // Parse the custom attributes passed to the macro
    let test_file_dat_parser = syn::meta::parser(|meta| attrs.parse(meta));
    parse_macro_input!(args with test_file_dat_parser);

    // Check if required attributes are present
    let path = attrs
        .path
        .unwrap_or_else(|| panic!("'path' attribute is required"));
    let kind = attrs
        .kind
        .unwrap_or_else(|| panic!("'kind' attribute is required"));

    // Generate the test function based on the parsed attributes
    let generated = impl_test_file_data(&func, path, kind);

    // Update the function's attributes and signature
    let mut input = proc_macro2::TokenStream::from(generated);
    if let Some(pos) = func
        .attrs
        .iter()
        .position(|attr| attr.path().is_ident("test"))
    {
        func.attrs.remove(pos);
    }
    func.sig.ident = Ident::new(&format!("_{}", &func.sig.ident), Span::call_site());
    func.to_tokens(&mut input);

    // Return the generated code
    input.into()
}

#[derive(Default)]
struct TestFileDataAttributes {
    kind: Option<LitStr>,
    path: Option<LitStr>,
}

impl TestFileDataAttributes {
    // Parse the nested meta attributes
    fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
        if meta.path.is_ident("kind") {
            let kind: LitStr = meta.value()?.parse()?;
            if !SUPPORTED_KINDS.contains(&kind.value().as_str()) {
                return Err(meta.error("unsupported kind"));
            }
            self.kind = kind.into();
        } else if meta.path.is_ident("path") {
            let path: LitStr = meta.value()?.parse()?;
            let path_str = path.value();
            let file_path = Path::new(&path_str);
            if !file_path.exists() {
                return Err(meta.error("file does not exist"));
            }
            if !file_path.is_file() {
                return Err(meta.error("path must be a file"));
            }
            if let (true, Some(ext)) = (
                self.kind.is_none(),
                file_path.extension().and_then(|s| s.to_str()),
            ) {
                if SUPPORTED_KINDS.contains(&ext) {
                    self.kind = LitStr::new(ext, path.span()).into();
                }
            }
            self.path = path.into();
        } else {
            return Err(meta.error("unsupported property"));
        }
        Ok(())
    }
}

// Function that generates the implementation of the test data parsing
fn impl_test_file_data(item: &ItemFn, path: LitStr, kind: LitStr) -> TokenStream {
    // Extract function name and argument details
    let name = &item.sig.ident;
    let call_ident = Ident::new(&format!("_{}", &item.sig.ident), Span::call_site());

    // Collect field names and types
    let (field_names, field_types): (Vec<_>, Vec<_>) = item
        .sig
        .inputs
        .iter()
        .filter_map(|field| match field {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat_type) => {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    Some((&pat_ident.ident, &pat_type.ty))
                } else {
                    None
                }
            }
        })
        .unzip();

    let kind_str = kind.value();

    // Define the logic for generating code based on the file kind (e.g., CSV or other format)
    let gen = if kind_str == "csv" {
        quote! {
            #[test]
            fn #name() {
                #[derive(Debug, serde::Deserialize)]
                struct _Data {
                    #(#field_names: #field_types,)*
                }
                let file_path = #path;

                // Reading CSV data
                let mut rdr = csv::ReaderBuilder::new()
                    .from_path(file_path)
                    .unwrap();
                let mut executed = false;
                for result in rdr.deserialize() {
                    let record: _Data = result.unwrap();
                    executed = true;
                    let _Data { #(#field_names,)* } = record;
                    #call_ident(#(#field_names,)*);
                }
                if !executed {
                    panic!("Empty test data provided in {file_path}");
                }
            }
        }
    } else if kind_str == "list" {
        quote! {
            #[test]
            fn #name() {
                use std::io::BufRead;
                let file_path = #path;
                let f = std::fs::File::open(file_path).unwrap();
                let lines = std::io::BufReader::new(f).lines();
                let mut executed = false;

                for (n, line) in lines.enumerate() {
                    if n == 0 {
                        continue;
                    }
                    executed = true;
                    let line = line.unwrap();
                    let mut iter = line.split(' ').filter(|f| !f.is_empty());
                    let mut column = 0;
                    #(
                        let field = iter.next().unwrap();
                        let #field_names = field.parse().map_err(|e| format!("Invalid value in row={n} column={column} {file_path} {e}")).unwrap();
                        column += 1;
                    )*
                    #call_ident(#(#field_names,)*);
                }
                if !executed {
                    panic!("Empty test data provided in {file_path}");
                }
            }
        }
    } else {
        let kind = Ident::new(&kind_str, kind.span());
        let serde_read = match kind_str.as_str() {
            "yaml" | "json" => {
                let kind = Ident::new(&format!("serde_{kind_str}"), kind.span());
                quote! {
                    #kind::from_reader(std::fs::File::open(#path).unwrap()).unwrap()
                }
            }
            "toml" => quote! {
                #kind::from_str(&std::fs::read_to_string(#path).unwrap()).unwrap()
            },
            _ => quote! {
                #kind::de::from_reader(std::fs::File::open(#path).unwrap()).unwrap()
            },
        };

        quote! {
            #[test]
            fn #name() {
                #[derive(Debug, serde::Deserialize)]
                struct _Data {
                    #(#field_names: #field_types,)*
                }

                #[derive(Debug, serde::Deserialize)]
                #[serde(untagged)]
                enum Collection {
                    Index(Vec<_Data>),
                    Map(std::collections::HashMap<String, _Data>)
                }

                let file_path = #path;

                let values: Collection = #serde_read;
                let values = match values {
                    Collection::Index(v) => v,
                    Collection::Map(m) => m.into_iter().map(|(_, v)| v).collect(),
                };

                if values.is_empty() {
                    panic!("Empty test data provided in {file_path}");
                }

                for test_data in values {
                    let _Data { #(#field_names,)* } = test_data;
                    #call_ident(#(#field_names,)*);
                }

            }
        }
    };

    gen.into()
}

// #[test]
// fn t() {
//     #[derive(Debug, serde::Deserialize)]
//     struct _Data {}
//     #[derive(Debug, serde::Deserialize)]
//     #[serde(untagged)]
//     enum Collection {
//         Index(Vec<_Data>),
//         Map(std::collections::HashMap<String, _Data>),
//     }
//     let values: Collection =
//         toml::from_str(&std::fs::read_to_string("tests/samples/test_me.toml").unwrap()).unwrap();
//     let values = match values {
//         Collection::Index(v) => v,
//         Collection::Map(m) => m.into_iter().map(|(_, v)| v).collect(),
//     };
//     for test_data in iter {
//         let _Data {
//             max_value,
//             supported,
//         } = test_data;
//         _test_test_me_with_toml(max_value, supported);
//     }
// }
