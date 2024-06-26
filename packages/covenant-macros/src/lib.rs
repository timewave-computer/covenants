use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DataEnum, DeriveInput};

// Merges the variants of two enums.
fn merge_variants(metadata: TokenStream, left: TokenStream, right: TokenStream) -> TokenStream {
    use syn::Data::Enum;

    let args = parse_macro_input!(metadata as AttributeArgs);
    if let Some(first_arg) = args.first() {
        return syn::Error::new_spanned(first_arg, "macro takes no arguments")
            .to_compile_error()
            .into();
    }

    let mut left: DeriveInput = parse_macro_input!(left);
    let right: DeriveInput = parse_macro_input!(right);

    if let (
        Enum(DataEnum { variants, .. }),
        Enum(DataEnum {
            variants: to_add, ..
        }),
    ) = (&mut left.data, right.data)
    {
        variants.extend(to_add);

        quote! { #left }.into()
    } else {
        syn::Error::new(left.ident.span(), "variants may only be added for enums")
            .to_compile_error()
            .into()
    }
}

#[proc_macro_attribute]
pub fn clocked(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum Clocked {
                /// Wakes the state machine up. The caller should
                /// check the sender of the tick is the clock if
                /// they'd like to pause when the clock does.
                Tick {},
            }
        )
        .into(),
    )
}

#[proc_macro_attribute]
pub fn covenant_deposit_address(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum Deposit {
                /// Returns the address a contract expects to receive funds to
                #[returns(Option<String>)]
                DepositAddress {},
            }
        )
        .into(),
    )
}

#[proc_macro_attribute]
pub fn covenant_clock_address(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum Clock {
                /// Returns the associated clock address authorized to submit ticks
                #[returns(Addr)]
                ClockAddress {},
            }
        )
        .into(),
    )
}

#[proc_macro_attribute]
pub fn covenant_remote_chain(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum RemoteChain {
                /// Returns the associated remote chain information
                #[returns(RemoteChainInfo)]
                RemoteChainInfo {},
            }
        )
        .into(),
    )
}

#[proc_macro_attribute]
pub fn covenant_ica_address(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum ICA {
                /// Returns the associated remote chain information
                #[returns(Option<String>)]
                IcaAddress {},
            }
        )
        .into(),
    )
}

#[proc_macro_attribute]
pub fn covenant_next_contract(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum NextContract {
                /// Returns the associated remote chain information
                #[returns(Option<String>)]
                NextContract {},
            }
        )
        .into(),
    )
}

#[proc_macro_attribute]
pub fn covenant_lper_withdraw(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum WithdrawMsgs {
                /// Tells the LPer to withdraw his position
                /// Should only be called by the holder of the covenant
                Withdraw { percentage: Option<Decimal> },
            }
        )
        .into(),
    )
}

#[proc_macro_attribute]
pub fn covenant_holder_distribute(metadata: TokenStream, input: TokenStream) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum DistributeMsgs {
                /// After LPer finished withdrawing from LP, it sends the funds to the holder
                /// and the holder distributes them based on its logic
                /// Should only be called by the LPer of the covenant
                Distribute {},
                /// This message is sent in case we do an IBC withdraw
                /// The withdraw can fail in async way, in case that happens we want the holder to be notified on that.
                /// In case of astroport, the withdraww + distribution is atomic, so nothing to worry there
                /// But in case  of osmosis, the withdraw is async, so the "claim" will successful happen,
                /// while the withdraw can fail, in case the withdraw fails here, we execute this message on the holder
                WithdrawFailed {},
            }
        )
        .into(),
    )
}

#[proc_macro_attribute]
pub fn covenant_holder_emergency_withdraw(
    metadata: TokenStream,
    input: TokenStream,
) -> TokenStream {
    merge_variants(
        metadata,
        input,
        quote!(
            enum EmergencyWithdrawMsgs {
                /// Allows for the emergency committee to withdraw the funds on case of an emergency
                EmergencyWithdraw {},
            }
        )
        .into(),
    )
}
