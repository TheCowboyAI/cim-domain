// Copyright 2025 Cowboy AI, LLC.

//! Utility to inspect CID details

use cim_ipld::Cid;

fn main() {
    // Example CID from the error message
    let cid_str = "bagaaihraaskygedmeurevmlp6cfsuxg2uv4jc3xihchwlojiiwksdz4vspiq";

    match Cid::try_from(cid_str) {
        Ok(cid) => {
            println!("CID: {cid}");
            println!("Version: {:?}", cid.version());
            println!("Codec: 0x{:x} ({})", cid.codec(), codec_name(cid.codec()));
            println!("Hash algorithm: {}", hash_name(cid.hash().code()));
            println!("Hash digest length: {} bytes", cid.hash().size());

            // Convert hash to hex string
            let hash_hex: String = cid
                .hash()
                .to_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect();
            println!("Multihash (hex): {hash_hex}");
        }
        Err(e) => {
            eprintln!("Failed to parse CID: {e}");
        }
    }
}

fn codec_name(codec: u64) -> &'static str {
    match codec {
        0x0200 => "json",
        0x0201 => "cbor",
        0x55 => "raw",
        0x70 => "dag-pb",
        0x71 => "dag-cbor",
        0x72 => "libp2p-key",
        0x0129 => "dag-json",
        _ => "unknown",
    }
}

fn hash_name(code: u64) -> &'static str {
    match code {
        0x00 => "identity",
        0x11 => "sha1",
        0x12 => "sha2-256",
        0x13 => "sha2-512",
        0x14 => "sha3-512",
        0x15 => "sha3-384",
        0x16 => "sha3-256",
        0x17 => "sha3-224",
        0x18 => "shake-128",
        0x19 => "shake-256",
        0x1a => "keccak-224",
        0x1b => "keccak-256",
        0x1c => "keccak-384",
        0x1d => "keccak-512",
        0x1e => "blake3",
        0x20 => "murmur3-128",
        0x21 => "murmur3-32",
        _ => "unknown",
    }
}
