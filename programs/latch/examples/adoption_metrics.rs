//! Adoption metrics indexer — independently verifiable, not self-reported.
//!
//! Counts, from on-chain CPI events emitted by the Latch program:
//!   - total deals created                (`DealCreated` events)
//!   - distinct creator accounts          (creator field of `DealCreated`)
//!   - unique signer pubkeys              (`TermsSigned` events)
//!
//! Anyone can run this against public RPC and reproduce the numbers:
//!   cargo run --example adoption_metrics [RPC_URL]
//!
//! Method: walk every transaction that mentions the program, then decode the
//! program's self-CPI event instructions from `meta.innerInstructions`
//! (Anchor `emit_cpi!` events ride in inner-instruction data, NOT in logs —
//! logs can be truncated by RPC providers and are not trustworthy for
//! metrics).

use {
    anchor_lang::{AnchorDeserialize, Discriminator},
    latch::events::{DealCreated, TermsSigned},
    solana_rpc_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient},
    solana_rpc_client_api::request::RpcRequest,
    solana_signature::Signature,
    std::{collections::HashSet, str::FromStr},
};

const DEFAULT_RPC: &str = "https://api.devnet.solana.com";

fn main() {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_RPC.to_string());
    let client = RpcClient::new(url.clone());
    let program_id = latch::id();
    println!("program: {program_id}");
    println!("rpc:     {url}\n");

    // 1. Collect every transaction signature mentioning the program (paginated).
    let mut signatures: Vec<String> = Vec::new();
    let mut before: Option<Signature> = None;
    loop {
        let batch = client
            .get_signatures_for_address_with_config(
                &program_id,
                GetConfirmedSignaturesForAddress2Config {
                    before: before.take(),
                    ..Default::default()
                },
            )
            .expect("get_signatures_for_address");
        if batch.is_empty() {
            break;
        }
        let last = batch.last().unwrap().signature.clone();
        signatures.extend(
            batch
                .into_iter()
                .filter(|s| s.err.is_none())
                .map(|s| s.signature),
        );
        if signatures.len() % 1000 == 0 {
            eprintln!("  … {} signatures", signatures.len());
        }
        before = Some(Signature::from_str(&last).expect("signature parse"));
    }
    println!("transactions to scan: {}\n", signatures.len());

    // 2. Decode CPI events out of each transaction's inner instructions.
    let mut total_deals = 0u64;
    let mut creators: HashSet<String> = HashSet::new();
    let mut signers: HashSet<String> = HashSet::new();

    for (i, sig_str) in signatures.iter().enumerate() {
        if i > 0 && i % 50 == 0 {
            eprintln!("  … scanned {i}/{}", signatures.len());
        }
        // Fetch the raw RPC JSON so the decode path matches the wire format
        // exactly (independent of client-library type churn).
        let v: serde_json::Value = match client.send(
            RpcRequest::GetTransaction,
            serde_json::json!([sig_str, {"encoding": "json", "maxSupportedTransactionVersion": 0}]),
        ) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("  skip {sig_str}: {e}");
                continue;
            }
        };
        let keys: Vec<String> = v["transaction"]["message"]["accountKeys"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|k| k.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let empty = Vec::new();
        let inner = v["meta"]["innerInstructions"].as_array().unwrap_or(&empty);
        for group in inner {
            for ix in group["instructions"].as_array().unwrap_or(&empty) {
                let pidx = ix["programIdIndex"].as_u64().unwrap_or(u64::MAX) as usize;
                if keys.get(pidx).map(String::as_str) != Some(&program_id.to_string() as &str) {
                    continue;
                }
                let Some(data_b58) = ix["data"].as_str() else {
                    continue;
                };
                let Ok(data) = bs58::decode(data_b58).into_vec() else {
                    continue;
                };
                // Event CPI layout: 8-byte event-ix tag, 8-byte event
                // discriminator, then the borsh-encoded event.
                if data.len() < 16 || &data[..8] != anchor_lang::event::EVENT_IX_TAG_LE {
                    continue;
                }
                let (disc, body) = (&data[8..16], &data[16..]);
                if disc == DealCreated::DISCRIMINATOR {
                    if let Ok(ev) = DealCreated::try_from_slice(body) {
                        total_deals += 1;
                        creators.insert(ev.creator.to_string());
                    }
                } else if disc == TermsSigned::DISCRIMINATOR {
                    if let Ok(ev) = TermsSigned::try_from_slice(body) {
                        signers.insert(ev.party.to_string());
                    }
                }
            }
        }
    }

    println!("== Latch adoption metrics (from on-chain events) ==");
    println!("total deals created:      {total_deals}");
    println!("distinct creator accounts: {}", creators.len());
    println!("unique signer pubkeys:     {}", signers.len());
}
