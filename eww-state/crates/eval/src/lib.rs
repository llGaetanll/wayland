//! `eval!` — drive a state machine through a conversation.
//!
//! A machine test is a slice of events in and a `Vec<Action>` out. Written by
//! hand that shape disappears into `Vec::new()`, length checks and index
//! arithmetic, and the conversation the test is *about* is buried in it. This
//! macro is the conversation and nothing else:
//!
//! ```ignore
//! eval!(&mut state,
//!     < Event::FromUi(EventUi::MenuToggled { menu: MenuId::Wifi }),
//!     > Action::ToShell { action: ActionShell::WifiScan, .. },
//!     > Action::ToProbe { action: ActionProbe::Sample { .. }, .. },
//! );
//! ```
//!
//! `<` is an event going in, `>` is an action expected out. Adjacent `<` lines
//! are one batch: they reach `handle_events` together, and the `>` lines after
//! them are exactly what that call must produce — in order, nothing extra,
//! nothing missing. A `<` after a `>` starts the next batch, and a trailing
//! batch with no `>` asserts silence.
//!
//! The `>` lines are patterns, not values, so they expand to `matches!` and
//! need no `PartialEq` on any type they touch.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;
use syn::Expr;
use syn::Pat;
use syn::Token;

/// One `handle_events` call: the events that go in, the actions that must
/// come out.
struct Batch {
    events: Vec<Expr>,
    actions: Vec<Pat>,
}

struct Eval {
    state: Expr,
    batches: Vec<Batch>,
}

impl Parse for Eval {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let state: Expr = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut batches: Vec<Batch> = Vec::new();
        while !input.is_empty() {
            if input.peek(Token![<]) {
                input.parse::<Token![<]>()?;
                // A `<` on a batch that already has actions closes it. That is
                // the whole of the batching logic, and it happens here rather
                // than at runtime.
                let open = matches!(batches.last(), Some(b) if b.actions.is_empty());
                if !open {
                    batches.push(Batch {
                        events: Vec::new(),
                        actions: Vec::new(),
                    });
                }
                let Some(batch) = batches.last_mut() else {
                    unreachable!("a batch was just pushed if none was open")
                };
                batch.events.push(input.parse()?);
            } else if input.peek(Token![>]) {
                input.parse::<Token![>]>()?;
                let pat = Pat::parse_multi_with_leading_vert(input)?;
                let Some(batch) = batches.last_mut() else {
                    return Err(input.error("an expected action needs an event before it"));
                };
                batch.actions.push(pat);
            } else {
                return Err(input.error("expected `<` (an event in) or `>` (an action out)"));
            }
            // The separator is mandatory: without it `< a > b` parses as one
            // comparison expression rather than an event and an action.
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Eval { state, batches })
    }
}

/// `quote!` puts a space around every token, so a message would otherwise read
/// `Action :: ToShell { action : … }`. Cosmetic, and the only thing anyone
/// reads when a test breaks.
fn pattern_text(pat: &Pat) -> String {
    quote!(#pat)
        .to_string()
        .replace(" :: ", "::")
        .replace(" : ", ": ")
        .replace(" , ", ", ")
}

#[proc_macro]
pub fn eval(input: TokenStream) -> TokenStream {
    let Eval { state, batches } = parse_macro_input!(input as Eval);

    let steps = batches.iter().enumerate().map(|(i, batch)| {
        let events = &batch.events;
        let expected = batch.actions.len();
        // Unrolled rather than looped, so a failure can name the batch, the
        // position, the pattern source and what actually arrived.
        let checks = batch.actions.iter().enumerate().map(|(k, pat)| {
            let text = pattern_text(pat);
            quote! {
                assert!(
                    matches!(__actions[#k], #pat),
                    "eval! batch {} action {}: expected `{}`, got:\n{:#?}",
                    #i, #k, #text, __actions[#k],
                );
            }
        });
        quote! {
            {
                let __events = [#(#events),*];
                let mut __actions = ::std::vec::Vec::new();
                __state.handle_events(&__events, &mut __actions);
                // Count first: a missing or extra action reports as that,
                // rather than as a mismatch three lines further down.
                assert_eq!(
                    __actions.len(), #expected,
                    "eval! batch {}: expected {} action(s), got {}:\n{:#?}",
                    #i, #expected, __actions.len(), __actions,
                );
                #(#checks)*
            }
        }
    });

    quote! {{
        let __state = #state;
        #(#steps)*
    }}
    .into()
}
