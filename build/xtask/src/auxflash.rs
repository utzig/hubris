// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha3::{Digest, Sha3_256};

/// List of binary blobs to include in the auxiliary flash binary shipped with
/// this image.  The auxiliary flash is used to offload storage of large
/// configuration files (e.g. FPGA bitstreams)
#[derive(Clone, Debug, Deserialize)]
pub struct AuxFlash {
    pub blobs: Vec<AuxFlashBlob>,
}

/// A single binary blob, encoded into the auxiliary flash file.
#[derive(Clone, Debug, Deserialize)]
pub struct AuxFlashBlob {
    pub file: String,
    pub compress: bool,
    pub tag: String,
}

/// Packs a single blob into a TLV-C structure
fn pack_blob(blob: &AuxFlashBlob) -> Result<tlvc_text::Piece> {
    if blob.tag.len() != 4 {
        bail!("Tag must be a 4-byte value, not '{}'", blob.tag);
    }
    let data = std::fs::read(&blob.file)
        .with_context(|| format!("Could not read blob {}", blob.file))?;
    let data = if blob.compress {
        gnarle::compress_to_vec(&data)
    } else {
        data
    };
    let tag: [u8; 4] = blob.tag.as_bytes().try_into().unwrap();
    Ok(tlvc_text::Piece::Chunk(
        tlvc_text::Tag::new(tag),
        vec![tlvc_text::Piece::Bytes(data)],
    ))
}

/// Constructs an auxiliary flash image, based on RFD 311
///
/// Returns the checksum and the raw data to be saved
pub fn build_auxflash(aux: &AuxFlash) -> Result<([u8; 32], Vec<u8>)> {
    let mut auxi = vec![];
    for f in &aux.blobs {
        auxi.push(pack_blob(f)?);
    }
    let sha = Sha3_256::digest(tlvc_text::pack(&auxi));

    let out = [
        tlvc_text::Piece::Chunk(
            tlvc_text::Tag::new(*b"CHCK"),
            vec![tlvc_text::Piece::Bytes(sha.to_vec())],
        ),
        tlvc_text::Piece::Chunk(tlvc_text::Tag::new(*b"AUXI"), auxi),
    ];

    Ok((sha.into(), tlvc_text::pack(&out)))
}
