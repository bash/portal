extern crate proc_macro;

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    braced, parenthesized, parse_macro_input, Arm, Error, Expr, FnArg, Ident, Token, Type,
    Visibility,
};

#[proc_macro]
pub fn states(input: TokenStream) -> TokenStream {
    let StatesEnum {
        visibility,
        enum_token,
        ident,
        states,
    } = parse_macro_input!(input as StatesEnum);

    let state_variants: Punctuated<_, Token![,]> = states.iter().map(quote_enum_variant).collect();
    let next_impl = quote_next_impl(&ident, &states);
    let new_fns = quote_new_fns(&ident, &states);
    let expanded_enum = quote! {
        #visibility #enum_token #ident {
            #state_variants
        }

        impl #ident {
            #next_impl
            #new_fns
        }
    };

    TokenStream::from(expanded_enum)
}

fn quote_enum_variant(
    State {
        ident,
        fields,
        async_,
    }: &State,
) -> TokenStream2 {
    let promise_field = async_.as_ref().map(
        |AsyncState {
             output: execute_output,
             ..
         }| quote! { ::poll_promise::Promise<#execute_output>, },
    );
    let quoted_fields: Punctuated<_, Token![,]> =
        fields.iter().map(|StateField { ty, .. }| ty).collect();
    quote! { #ident(#promise_field #quoted_fields) }
}

fn quote_next_impl(ident: &Ident, states: &[State]) -> TokenStream2 {
    let next_match_arms: Punctuated<_, Token![,]> = states
        .iter()
        .filter_map(|state| {
            state
                .async_
                .as_ref()
                .map(|async_| quote_state_next_impl(state, async_))
        })
        .collect();
    quote! {
        fn next(&mut self, ui: &mut ::egui::Ui) {
            use #ident::*;
            ::replace_with::replace_with(self, ::std::default::Default::default, |__state| {
                match __state {
                    #next_match_arms,
                    _ => __state,
                }
            });
        }
    }
}

fn quote_state_next_impl(
    State { ident, fields, .. }: &State,
    AsyncState { next_arms, .. }: &AsyncState,
) -> TokenStream2 {
    let fields_quoted: Punctuated<_, Token![,]> = fields
        .iter()
        .map(|StateField { ident, .. }| ident.to_token_stream())
        .collect();
    let next_arms_quoted: TokenStream2 =
        next_arms.iter().map(|arm| arm.to_token_stream()).collect();
    quote! {
        #ident(__state_promise, #fields_quoted) => match __state_promise.try_take() {
            Ok(__state_promise_ok) => match __state_promise_ok { #next_arms_quoted },
            Err(__state_promise) => #ident(__state_promise, #fields_quoted),
        }
    }
}

fn quote_new_fns(ident: &Ident, states: &[State]) -> TokenStream2 {
    states
        .iter()
        .filter_map(|state| {
            state
                .async_
                .as_ref()
                .map(|async_| quote_state_new_impl(ident, state, async_))
        })
        .collect()
}

fn quote_state_new_impl(
    enum_ident: &Ident,
    State { ident, fields, .. }: &State,
    AsyncState {
        new_inputs,
        output,
        new_expr,
        ..
    }: &AsyncState,
) -> TokenStream2 {
    let mut new_ident: Ident = Ident::new(
        &format!("new_{}", ident.to_string().to_snake_case()),
        ident.span(),
    );
    new_ident.set_span(ident.span());
    let params: Punctuated<_, Token![,]> = new_inputs.iter().collect();
    let field_args: Punctuated<_, Token![,]> = fields
        .iter()
        .map(|StateField { ident, .. }| Ident::new(&format!("__new_result_{ident}"), ident.span()))
        .collect();
    quote! {
        #[allow(clippy::too_many_arguments)]
        fn #new_ident(ui: &mut Ui, #params) -> Self {
            let (__future, #field_args) = #new_expr;
            #enum_ident::#ident(
                ui.ctx().spawn_async::<#output>(__future),
                #field_args
            )
        }
    }
}

struct StatesEnum {
    visibility: Visibility,
    enum_token: Token![enum],
    ident: Ident,
    states: Vec<State>,
}

impl Parse for StatesEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let visibility: Visibility = input.parse()?;
        let enum_token: Token![enum] = input.parse()?;
        let ident: Ident = input.parse()?;
        input.parse::<Token![;]>()?;
        let mut states = Vec::new();
        while !input.is_empty() {
            states.push(input.parse()?);
        }
        Ok(StatesEnum {
            visibility,
            enum_token,
            ident,
            states,
        })
    }
}

struct State {
    ident: Ident,
    fields: Punctuated<StateField, Token![,]>,
    async_: Option<AsyncState>,
}

impl Parse for State {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let is_async = input.peek(Token![async]);
        if is_async {
            input.parse::<Token![async]>()?;
        }

        parse_custom_keyword(input, "state")?;

        let ident: Ident = input.parse()?;

        let fields_unparsed;
        parenthesized!(fields_unparsed in input);
        let fields: Punctuated<StateField, Token![,]> =
            Punctuated::parse_terminated(&fields_unparsed)?;

        let async_: Option<AsyncState> = if is_async {
            Some(input.parse()?)
        } else {
            input.parse::<Token![;]>()?;
            None
        };

        Ok(State {
            ident,
            fields,
            async_,
        })
    }
}

struct StateField {
    ident: Ident,
    ty: Type,
}

impl Parse for StateField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        Ok(StateField { ident, ty })
    }
}

struct AsyncState {
    new_inputs: Punctuated<FnArg, Token![,]>,
    output: Type,
    new_expr: Expr,
    next_arms: Vec<Arm>,
}

impl Parse for AsyncState {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![->]>()?;

        let output: Type = input.parse()?;

        let async_block;
        braced!(async_block in input);

        parse_custom_keyword(&async_block, "new")?;
        let execute_inputs_unparsed;
        parenthesized!(execute_inputs_unparsed in &async_block);
        let new_inputs: Punctuated<FnArg, Token![,]> =
            Punctuated::parse_terminated(&execute_inputs_unparsed)?;
        let new_expr: Expr = async_block.parse()?;

        parse_custom_keyword(&async_block, "next")?;

        let next_arms_unparsed;
        braced!(next_arms_unparsed in &async_block);
        let mut next_arms: Vec<Arm> = Vec::new();
        while !next_arms_unparsed.is_empty() {
            next_arms.push(next_arms_unparsed.parse()?);
        }

        Ok(AsyncState {
            new_inputs,
            output,
            new_expr,
            next_arms,
        })
    }
}

fn parse_custom_keyword(input: ParseStream, name: &str) -> syn::Result<Ident> {
    let name_token: Ident = input.parse()?;
    if name_token != name {
        Err(Error::new(name_token.span(), format!("expected `{name}`")))
    } else {
        Ok(name_token)
    }
}
