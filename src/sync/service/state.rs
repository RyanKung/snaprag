use crate::sync::client::proto;

/// Statistics for chunk processing
#[derive(Debug, Default, Clone)]
pub struct ChunkProcessStats {
    pub blocks_processed: u64,
    pub messages_processed: u64,
    pub last_block_number: Option<u64>,
}

impl ChunkProcessStats {
    #[must_use]
    pub const fn blocks_processed(&self) -> u64 {
        self.blocks_processed
    }

    #[must_use]
    pub const fn messages_processed(&self) -> u64 {
        self.messages_processed
    }

    #[must_use]
    pub const fn last_block_number(&self) -> Option<u64> {
        self.last_block_number
    }

    pub(super) fn record_chunk(&mut self, block_number: Option<u64>, message_count: u64) {
        self.blocks_processed += 1;
        self.messages_processed += message_count;
        if let Some(block_number) = block_number {
            let updated = match self.last_block_number {
                Some(current) => current.max(block_number),
                None => block_number,
            };
            self.last_block_number = Some(updated);
        }
    }
}

/// Extract block number from shard chunk
pub(super) fn extract_block_number(chunk: &proto::ShardChunk) -> Option<u64> {
    chunk
        .header
        .as_ref()
        .and_then(|header| header.height.as_ref())
        .map(|height| height.block_number)
}

/// Count messages in a shard chunk
pub(super) fn count_chunk_messages(chunk: &proto::ShardChunk) -> u64 {
    chunk
        .transactions
        .iter()
        .map(|tx| tx.user_messages.len() as u64)
        .sum()
}
