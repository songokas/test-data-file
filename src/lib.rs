use std::path::Path;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::parse::Result;
use syn::{meta::ParseNestedMeta, parse_macro_input, FnArg, ItemFn, LitStr, Pat};

const SUPPORTED_KINDS: [&str; 7] = ["csv", "json", "yaml", "ron", "toml", "list", "sql"];

#[allow(clippy::test_attr_in_doctest)]
/// Provide sample data from a file to your test function
///
/// # Arguments
///
/// * path - path to the sample
/// * kind - optional file format (if extension is not specified)
///
/// # Example
///
/// ```
/// use test_data_file::test_data_file;
/// #[test_data_file(path = "tests/samples/test_me.yaml")]
/// #[test]
/// fn test_is_name_above_max_size(name: Option<String>, max_size: usize, is_above: bool) {
///     assert_eq!(
///         name.map(|n| n.len()) > Some(max_size),
///         is_above,
///         "failed for {max_size}"
///     );
/// }
/// ```
///
#[proc_macro_attribute]
pub fn test_data_file(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    let mut attrs = TestFileDataAttributes::default();

    let test_file_dat_parser = syn::meta::parser(|meta| attrs.parse(meta));
    parse_macro_input!(args with test_file_dat_parser);

    let path = attrs
        .path
        .unwrap_or_else(|| panic!("'path' attribute is required"));
    let kind = attrs
        .kind
        .unwrap_or_else(|| panic!("'kind' attribute is required"));

    let generated = impl_test_data_file(&func, path, kind);

    let mut input = proc_macro2::TokenStream::from(generated);
    func.attrs.retain(|attr| {
        !(attr.path().is_ident("test")
            || attr.path().is_ident("should_panic")
            || attr
                .path()
                .segments
                .first()
                .map(|s| s.ident == "tokio")
                .unwrap_or(false))
    });
    func.sig.ident = Ident::new(&format!("_{}", &func.sig.ident), func.sig.ident.span());
    func.to_tokens(&mut input);
    input.into()
}

#[derive(Default)]
struct TestFileDataAttributes {
    kind: Option<LitStr>,
    path: Option<LitStr>,
}

impl TestFileDataAttributes {
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

fn impl_test_data_file(item: &ItemFn, path: LitStr, kind: LitStr) -> TokenStream {
    let name = &item.sig.ident;
    let call_ident = Ident::new(&format!("_{}", &item.sig.ident), Span::call_site());

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
    let func_attrs: Vec<_> = item.attrs.iter().collect();
    let func_async = item.sig.asyncness;
    let func_await = if func_async.is_some() {
        Some(quote! {.await})
    } else {
        None
    };

    let body = if kind_str == "csv" {
        quote! {
            #[derive(Debug, serde::Deserialize)]
            struct _Data {
                #(#field_names: #field_types,)*
            }
            let file_path = #path;

            let mut rdr = csv::ReaderBuilder::new()
                .from_path(file_path)
                .unwrap();
            let mut executed = false;
            for result in rdr.deserialize() {
                let record: _Data = result.unwrap();
                executed = true;
                let _Data { #(#field_names,)* } = record;
                #call_ident(#(#field_names,)*)#func_await;
            }
            if !executed {
                panic!("Empty test data provided in {file_path}");
            }
        }
    } else if kind_str == "list" {
        quote! {
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
                #call_ident(#(#field_names,)*)#func_await;
            }
            if !executed {
                panic!("Empty test data provided in {file_path}");
            }
        }
    } else if kind_str == "sql" {
        quote! {
            #[derive(Debug, serde::Deserialize)]
            struct _Data {
                #(#field_names: #field_types,)*
            }

            fn __test_data_file_sql_object_name(parts: &[sqlparser::ast::ObjectNamePart]) -> String {
                match parts.last() {
                    Some(sqlparser::ast::ObjectNamePart::Identifier(ident)) => ident.value.clone(),
                    _ => panic!("unsupported object name"),
                }
            }

            fn __test_data_file_sql_expr_to_json(expr: &sqlparser::ast::Expr) -> serde_json::Value {
                match expr {
                    sqlparser::ast::Expr::Value(vws) => match &vws.value {
                        sqlparser::ast::Value::Null => serde_json::Value::Null,
                        sqlparser::ast::Value::Boolean(b) => serde_json::Value::Bool(*b),
                        sqlparser::ast::Value::Number(n, _) => n
                            .parse::<i64>()
                            .map(serde_json::Value::from)
                            .or_else(|_| n.parse::<f64>().map(serde_json::Value::from))
                            .unwrap_or_else(|_| panic!("invalid numeric literal {n}")),
                        sqlparser::ast::Value::SingleQuotedString(s) => serde_json::Value::String(s.clone()),
                        other => panic!("unsupported sql literal {other:?}"),
                    },
                    other => panic!("unsupported sql expression {other:?}"),
                }
            }

            fn __test_data_file_load_sql_rows(file_path: &str, sql_text: &str, target_field_names: &[&str]) -> Vec<serde_json::Value> {
                use std::collections::{HashMap, HashSet};
                use serde_json::{Map, Value};
                use sqlparser::ast::{SetExpr, Statement, TableObject};
                use sqlparser::dialect::GenericDialect;
                use sqlparser::parser::Parser;

                let dialect = GenericDialect {};
                let statements = Parser::parse_sql(&dialect, sql_text)
                    .unwrap_or_else(|e| panic!("failed to parse sql in {file_path} {e}"));

                let mut table_columns: HashMap<String, Vec<String>> = HashMap::new();
                let mut table_rows: HashMap<String, Vec<Map<String, Value>>> = HashMap::new();
                let mut table_order: Vec<String> = Vec::new();

                for statement in statements {
                    let Statement::Insert(insert) = statement else { continue; };
                    let table_name = match &insert.table {
                        TableObject::TableName(name) => __test_data_file_sql_object_name(&name.0),
                        _ => panic!("unsupported insert target in {file_path}"),
                    };
                    if !table_order.iter().any(|t| t == &table_name) {
                        table_order.push(table_name.clone());
                    }
                    let columns: Vec<String> = insert
                        .columns
                        .iter()
                        .map(|c| __test_data_file_sql_object_name(&c.0))
                        .collect();

                    let query = insert
                        .source
                        .unwrap_or_else(|| panic!("INSERT INTO {table_name} has no VALUES source in {file_path}"));
                    let SetExpr::Values(values) = *query.body else {
                        panic!("INSERT INTO {table_name} must use VALUES in {file_path}");
                    };

                    for row in &values.rows {
                        let mut map = Map::new();
                        for (col, expr) in columns.iter().zip(row.iter()) {
                            map.insert(col.clone(), __test_data_file_sql_expr_to_json(expr));
                        }
                        table_rows.entry(table_name.clone()).or_default().push(map);
                    }
                    table_columns.entry(table_name.clone()).or_default().extend(columns);
                }

                if table_columns.is_empty() {
                    panic!("Empty test data provided in {file_path}");
                }

                let table_names: HashSet<&String> = table_columns.keys().collect();

                let mut child_of: HashMap<String, String> = HashMap::new();
                for (table, columns) in &table_columns {
                    for other in &table_names {
                        if *other == table {
                            continue;
                        }
                        let fk_col = format!("{other}_id");
                        if columns.iter().any(|c| c == &fk_col) {
                            child_of.insert(table.clone(), (*other).clone());
                        }
                    }
                }

                let root_candidates: Vec<&String> = table_columns
                    .keys()
                    .filter(|t| !child_of.contains_key(*t))
                    .collect();

                if root_candidates.len() > 1 {
                    // No single root table: none of the tables reference another.
                    // The first table in file order (that isn't itself a child)
                    // is the root; the test runs once per root row. Every table
                    // is deserialized into its own struct and embedded as a
                    // field of `_Data` under a key matching the table name.
                    // Non-root rows pair with root rows by position; a root row
                    // with no non-root row at its position leaves that key
                    // absent, so the parameter must be an `Option<_>` and
                    // deserializes to `None`.
                    let root_table = table_order
                        .iter()
                        .find(|t| !child_of.contains_key(t.as_str()))
                        .expect("more than one root candidate exists")
                        .clone();
                    let root_rows = table_rows
                        .get(&root_table)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);

                    let mut results = Vec::new();
                    for (i, root_row) in root_rows.iter().enumerate() {
                        let mut combined = Map::new();
                        combined.insert(root_table.clone(), Value::Object(root_row.clone()));
                        for table in &table_order {
                            if table == &root_table {
                                continue;
                            }
                            let rows = table_rows.get(table).map(|v| v.as_slice()).unwrap_or(&[]);
                            if let Some(row) = rows.get(i) {
                                combined.insert(table.clone(), Value::Object(row.clone()));
                            }
                        }
                        results.push(Value::Object(combined));
                    }
                    if results.is_empty() {
                        panic!("Empty test data provided in {file_path}");
                    }
                    return results;
                }

                let root_table = match root_candidates.as_slice() {
                    [only] => (*only).clone(),
                    [] => panic!(
                        "no root table found in {file_path}: every table has an outbound '<table>_id' foreign key (check for a reference cycle)"
                    ),
                    _ => unreachable!("more than one root candidate is handled above"),
                };

                let targeted_tables: HashSet<&String> = child_of.values().collect();
                let strip_root_id = targeted_tables.contains(&root_table);

                let children: Vec<&String> = child_of
                    .iter()
                    .filter(|(_, parent)| **parent == root_table)
                    .map(|(child, _)| child)
                    .collect();

                let root_rows = table_rows
                    .get(&root_table)
                    .unwrap_or_else(|| panic!("unreachable: root table {root_table} has no rows in {file_path}"));

                let mut results = Vec::new();
                for root_row in root_rows {
                    let root_id = root_row.get("id").cloned();
                    let mut out = root_row.clone();
                    if strip_root_id {
                        out.remove("id");
                    }

                    for child_table in &children {
                        let fk_col = format!("{root_table}_id");
                        let child_rows = table_rows.get(*child_table).map(|v| v.as_slice()).unwrap_or(&[]);
                        let matches: Vec<Map<String, Value>> = child_rows
                            .iter()
                            .filter(|row| row.get(&fk_col) == root_id.as_ref())
                            .map(|row| {
                                let mut row = row.clone();
                                row.remove(&fk_col);
                                row
                            })
                            .collect();
                        match matches.len() {
                            0 => {}
                            1 => {
                                out.insert((*child_table).clone(), Value::Object(matches.into_iter().next().unwrap()));
                            }
                            _ => {
                                out.insert(
                                    (*child_table).clone(),
                                    Value::Array(matches.into_iter().map(Value::Object).collect()),
                                );
                            }
                        }
                    }

                    let row_value = if target_field_names.len() == 1 && target_field_names[0] == root_table {
                        let mut wrapper = Map::new();
                        wrapper.insert(root_table.clone(), Value::Object(out));
                        Value::Object(wrapper)
                    } else {
                        Value::Object(out)
                    };
                    results.push(row_value);
                }

                results
            }

            let file_path = #path;
            let sql_text = std::fs::read_to_string(file_path).unwrap();
            let rows = __test_data_file_load_sql_rows(file_path, &sql_text, &[#(stringify!(#field_names)),*]);

            for row in rows {
                let test_data: _Data = serde_json::from_value(row)
                    .map_err(|e| format!("Failed to load data in {file_path} {e}"))
                    .unwrap();
                let _Data { #(#field_names,)* } = test_data;
                #call_ident(#(#field_names,)*)#func_await;
            }
        }
    } else {
        let kind = Ident::new(&kind_str, kind.span());
        let serde_read = match kind_str.as_str() {
            "yaml" | "json" => {
                let kind = Ident::new(&format!("serde_{kind_str}"), kind.span());
                quote! {
                    #kind::from_reader(std::fs::File::open(file_path).unwrap()).map_err(|e| format!("Failed to load data in {file_path} {e}")).unwrap()
                }
            }
            "toml" => quote! {
                #kind::from_str(&std::fs::read_to_string(file_path).unwrap()).map_err(|e| format!("Failed to load data in {file_path} {e}")).unwrap()
            },
            _ => quote! {
                #kind::de::from_reader(std::fs::File::open(file_path).unwrap()).map_err(|e| format!("Failed to load data in {file_path} {e}")).unwrap()
            },
        };

        quote! {
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
                #call_ident(#(#field_names,)*)#func_await;
            }
        }
    };

    let gen = quote! {
        #(#func_attrs)*
        #func_async fn #name() {
            #body
        }
    };
    gen.into()
}
