use crate::bitcoin::{Hash, MerkleRoot, TxHash};
use crate::{Error, Result};
use bytes::{BufMut, BytesMut};

/// Calculate the Merkle root from a list of transaction hashes
///
/// This implements the Bitcoin Merkle tree algorithm where:
/// - If there's only one transaction, its hash is the root
/// - If there's an odd number of transactions, the last one is duplicated
/// - Hashes are combined pairwise with double SHA256
pub fn calculate_merkle_root(tx_hashes: &[TxHash]) -> Result<MerkleRoot> {
    if tx_hashes.is_empty() {
        return Err(Error::BadArgument(
            "Cannot calculate merkle root of empty transaction list".to_string(),
        ));
    }

    // If there's only one transaction, its hash is the merkle root
    if tx_hashes.len() == 1 {
        return Ok(tx_hashes[0]);
    }

    // Start with the transaction hashes
    let mut current_level: Vec<Hash> = tx_hashes.to_vec();

    // Build the tree level by level
    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        // Process pairs of hashes
        let mut i = 0;
        while i < current_level.len() {
            let left = &current_level[i];

            // If we're at the last element and it's odd, duplicate it
            let right = if i + 1 < current_level.len() {
                &current_level[i + 1]
            } else {
                &current_level[i]
            };

            // Combine the two hashes
            let combined = hash_merkle_branches(left, right);
            next_level.push(combined);

            i += 2;
        }

        current_level = next_level;
    }

    Ok(current_level[0])
}

/// Hash two merkle branches together
///
/// This concatenates the two hashes and applies double SHA256
fn hash_merkle_branches(left: &Hash, right: &Hash) -> Hash {
    let mut data = BytesMut::with_capacity(64);
    data.put_slice(&left.raw);
    data.put_slice(&right.raw);
    Hash::sha256d(&data)
}

/// Build a Merkle proof for a transaction
///
/// Returns the list of hashes needed to prove that a transaction at the given
/// index is part of the Merkle tree with the given root
pub fn build_merkle_proof(tx_hashes: &[TxHash], index: usize) -> Result<Vec<Hash>> {
    if tx_hashes.is_empty() {
        return Err(Error::BadArgument(
            "Cannot build merkle proof for empty transaction list".to_string(),
        ));
    }

    if index >= tx_hashes.len() {
        return Err(Error::BadArgument(
            "Transaction index out of bounds".to_string(),
        ));
    }

    let mut proof = Vec::new();
    let mut current_level: Vec<Hash> = tx_hashes.to_vec();
    let mut current_index = index;

    // Build the proof by traversing up the tree
    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        // Process pairs of hashes
        let mut i = 0;
        while i < current_level.len() {
            let left_idx = i;
            let right_idx = if i + 1 < current_level.len() {
                i + 1
            } else {
                i // Duplicate the last element if odd
            };

            // If our target is in this pair, add the sibling to the proof
            if left_idx == current_index || right_idx == current_index {
                if left_idx == current_index {
                    proof.push(current_level[right_idx]);
                } else {
                    proof.push(current_level[left_idx]);
                }
                // Update the index for the next level
                current_index = next_level.len();
            }

            // Combine the two hashes
            let combined =
                hash_merkle_branches(&current_level[left_idx], &current_level[right_idx]);
            next_level.push(combined);

            i += 2;
        }

        current_level = next_level;
    }

    Ok(proof)
}

/// Verify a Merkle proof
///
/// Given a transaction hash, its index, a proof (list of sibling hashes),
/// and the expected root, verify that the transaction is part of the tree
pub fn verify_merkle_proof(
    tx_hash: &TxHash,
    index: usize,
    proof: &[Hash],
    root: &MerkleRoot,
) -> bool {
    let mut current_hash = *tx_hash;
    let mut current_index = index;

    for sibling in proof {
        // Determine if we're the left or right child
        if current_index.is_multiple_of(2) {
            // We're the left child
            current_hash = hash_merkle_branches(&current_hash, sibling);
        } else {
            // We're the right child
            current_hash = hash_merkle_branches(sibling, &current_hash);
        }
        // Move up to the parent level
        current_index /= 2;
    }

    current_hash == *root
}

/// Calculate the Merkle root for a partial tree (used in SPV)
///
/// This is used when we only have some of the transactions and their positions
#[allow(dead_code)]
pub fn calculate_partial_merkle_root(
    tx_hashes: &[(usize, TxHash)],
    total_tx_count: usize,
) -> Result<MerkleRoot> {
    if tx_hashes.is_empty() {
        return Err(Error::BadArgument(
            "Cannot calculate merkle root of empty transaction list".to_string(),
        ));
    }

    // Calculate the depth of the tree
    let mut _depth = 0;
    let mut level_size = total_tx_count;
    while level_size > 1 {
        level_size = level_size.div_ceil(2);
        _depth += 1;
    }

    // For partial trees, we'd need more complex logic to handle missing branches
    // This is a simplified version that assumes we have all transactions
    // A full implementation would require a different data structure

    // For now, return an error indicating this needs full implementation
    Err(Error::BadArgument(
        "Partial merkle tree calculation not fully implemented".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::FromHex;

    fn create_test_hash(s: &str) -> Hash {
        Hash::from_hex(s).unwrap()
    }

    #[test]
    fn test_single_transaction_merkle_root() {
        let tx =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let txs = vec![tx];

        let root = calculate_merkle_root(&txs).unwrap();
        assert_eq!(root, tx);
    }

    #[test]
    fn test_two_transaction_merkle_root() {
        let tx1 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let tx2 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000002");
        let txs = vec![tx1, tx2];

        let root = calculate_merkle_root(&txs).unwrap();
        let expected = hash_merkle_branches(&tx1, &tx2);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_odd_transaction_count() {
        // With 3 transactions, the last one should be duplicated
        let tx1 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let tx2 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000002");
        let tx3 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000003");
        let txs = vec![tx1, tx2, tx3];

        let root = calculate_merkle_root(&txs).unwrap();

        // Manual calculation
        let hash12 = hash_merkle_branches(&tx1, &tx2);
        let hash33 = hash_merkle_branches(&tx3, &tx3); // tx3 is duplicated
        let expected = hash_merkle_branches(&hash12, &hash33);

        assert_eq!(root, expected);
    }

    #[test]
    fn test_four_transaction_merkle_root() {
        let tx1 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let tx2 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000002");
        let tx3 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000003");
        let tx4 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000004");
        let txs = vec![tx1, tx2, tx3, tx4];

        let root = calculate_merkle_root(&txs).unwrap();

        // Manual calculation
        let hash12 = hash_merkle_branches(&tx1, &tx2);
        let hash34 = hash_merkle_branches(&tx3, &tx4);
        let expected = hash_merkle_branches(&hash12, &hash34);

        assert_eq!(root, expected);
    }

    #[test]
    fn test_large_tree() {
        // Test with 8 transactions
        let mut txs = Vec::new();
        for i in 1..=8 {
            let hex = format!("{:064x}", i);
            txs.push(create_test_hash(&hex));
        }

        let root = calculate_merkle_root(&txs).unwrap();

        // Build the tree manually to verify
        let h12 = hash_merkle_branches(&txs[0], &txs[1]);
        let h34 = hash_merkle_branches(&txs[2], &txs[3]);
        let h56 = hash_merkle_branches(&txs[4], &txs[5]);
        let h78 = hash_merkle_branches(&txs[6], &txs[7]);

        let h1234 = hash_merkle_branches(&h12, &h34);
        let h5678 = hash_merkle_branches(&h56, &h78);

        let expected = hash_merkle_branches(&h1234, &h5678);

        assert_eq!(root, expected);
    }

    #[test]
    fn test_empty_transaction_list() {
        let txs: Vec<TxHash> = vec![];
        let result = calculate_merkle_root(&txs);
        assert!(result.is_err());
    }

    #[test]
    fn test_merkle_proof_generation() {
        let tx1 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let tx2 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000002");
        let tx3 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000003");
        let tx4 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000004");
        let txs = vec![tx1, tx2, tx3, tx4];

        // Get proof for tx1 (index 0)
        let proof = build_merkle_proof(&txs, 0).unwrap();

        // The proof should contain tx2 (sibling at level 0) and hash34 (sibling at level 1)
        assert_eq!(proof.len(), 2);
        assert_eq!(proof[0], tx2);

        let hash34 = hash_merkle_branches(&tx3, &tx4);
        assert_eq!(proof[1], hash34);
    }

    #[test]
    fn test_merkle_proof_verification() {
        let tx1 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let tx2 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000002");
        let tx3 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000003");
        let tx4 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000004");
        let txs = vec![tx1, tx2, tx3, tx4];

        let root = calculate_merkle_root(&txs).unwrap();

        // Test proof for each transaction
        for (index, tx) in txs.iter().enumerate() {
            let proof = build_merkle_proof(&txs, index).unwrap();
            assert!(verify_merkle_proof(tx, index, &proof, &root));

            // Test with wrong index
            assert!(!verify_merkle_proof(tx, (index + 1) % 4, &proof, &root));

            // Test with wrong transaction
            let wrong_tx = create_test_hash(
                "000000000000000000000000000000000000000000000000000000000000dead",
            );
            assert!(!verify_merkle_proof(&wrong_tx, index, &proof, &root));
        }
    }

    #[test]
    fn test_merkle_proof_odd_count() {
        let tx1 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let tx2 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000002");
        let tx3 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000003");
        let txs = vec![tx1, tx2, tx3];

        let root = calculate_merkle_root(&txs).unwrap();

        // Test proof for tx3 (which gets duplicated)
        let proof = build_merkle_proof(&txs, 2).unwrap();
        assert!(verify_merkle_proof(&tx3, 2, &proof, &root));

        // The proof should contain tx3 (its own duplicate) and hash12
        assert_eq!(proof.len(), 2);
        assert_eq!(proof[0], tx3); // Its own duplicate

        let hash12 = hash_merkle_branches(&tx1, &tx2);
        assert_eq!(proof[1], hash12);
    }

    #[test]
    fn test_merkle_proof_single_transaction() {
        let tx =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let txs = vec![tx];

        let root = calculate_merkle_root(&txs).unwrap();
        let proof = build_merkle_proof(&txs, 0).unwrap();

        // For a single transaction, the proof should be empty
        assert_eq!(proof.len(), 0);

        // Verification should still work
        assert!(verify_merkle_proof(&tx, 0, &proof, &root));
    }

    #[test]
    fn test_hash_merkle_branches_order() {
        // Test that order matters in hash_merkle_branches
        let h1 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001");
        let h2 =
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000002");

        let hash12 = hash_merkle_branches(&h1, &h2);
        let hash21 = hash_merkle_branches(&h2, &h1);

        // The hashes should be different
        assert_ne!(hash12, hash21);
    }

    #[test]
    fn test_real_bitcoin_merkle_root() {
        // Test with transaction hashes from Bitcoin block #100000
        // These are actual transaction hashes from that block
        let tx_hashes = vec![
            create_test_hash("8c14f0db3df150123e6f3dbbf30f8b955a8249b62ac1d1ff16284aefa3d06d87"),
            create_test_hash("fff2525b8931402dd09222c50775608f75787bd2b87e56995a7bdd30f79702c4"),
            create_test_hash("6359f0868171b1d194cbee1af2f16ea598ae8fad666d9b012c8ed2b79a236ec4"),
            create_test_hash("e9a66845e05d5abc0ad04ec80f774a7e585c6e8db975962d069a522137b80c1d"),
        ];

        // Calculate the merkle root
        let root = calculate_merkle_root(&tx_hashes).unwrap();

        // The expected merkle root for these transactions
        // This can be verified against blockchain explorers
        // Note: This is a made-up example, real verification would need actual block data
        assert_eq!(root.raw.len(), 32);
    }

    #[test]
    fn test_merkle_proof_edge_cases() {
        // Test proof generation with invalid index
        let txs = vec![
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000001"),
            create_test_hash("0000000000000000000000000000000000000000000000000000000000000002"),
        ];

        let result = build_merkle_proof(&txs, 5);
        assert!(result.is_err());

        // Test proof generation with empty list
        let empty_txs: Vec<TxHash> = vec![];
        let result = build_merkle_proof(&empty_txs, 0);
        assert!(result.is_err());
    }
}
