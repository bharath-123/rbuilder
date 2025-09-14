use alloy_eips::eip7594::BlobTransactionSidecarVariant;

use crate::{
    live_builder::{block_output::true_value_bidding_service::NewTrueBlockValueBiddingService, order_input::replaceable_order_sink::ReplaceableOrderSink},
    primitives::{BundleReplacementData, Order, ShareBundleReplacementKey},
};

/// Filters out Orders with incorrect blobs (pre/post fusaka).
/// Since it's very unlikely what we have many wrong blobs we only filter on insert_order without take note of filtered orders.
/// If remove_bundle/remove_sbundle is called we just forward the call to the sink so it might try to remove a filtered order.
pub struct BlobTypeOrderFilter<FilterFunc> {
    sink: Box<dyn ReplaceableOrderSink>,
    ///true if it likes the blob sidecar, false if it doesn't (Order gets filtered).
    filter_func: FilterFunc,
}

impl<FilterFunc> std::fmt::Debug for BlobTypeOrderFilter<FilterFunc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobTypeOrderFilter")
            .field("sink", &"<dyn ReplaceableOrderSink>")
            .finish()
    }
}

/// Filters out EIP-7594 style blobs, supports only EIP-4844 style.
pub fn new_pre_fusaka(
    sink: Box<dyn ReplaceableOrderSink>,
) -> BlobTypeOrderFilter<impl Fn(&BlobTransactionSidecarVariant) -> bool + Send + Sync> {
    BlobTypeOrderFilter::new(sink, |blob| {
        matches!(blob, BlobTransactionSidecarVariant::Eip4844(_))
    })
}

/// Post-Osaka filter: Allow all transactions since EIP-7594 sidecars don't exist yet in practice.
/// The reth library doesn't support EIP-7594 yet, so all blob sidecars are EIP-4844 format.
pub fn new_fusaka(
    sink: Box<dyn ReplaceableOrderSink>,
) -> BlobTypeOrderFilter<impl Fn(&BlobTransactionSidecarVariant) -> bool + Send + Sync> {
    BlobTypeOrderFilter::new(sink, |blob| {
        match blob {
            BlobTransactionSidecarVariant::Eip4844(sidecar) => {
                // Allow all EIP-4844 sidecars (both regular txs and blob txs) post-Osaka
                // because reth doesn't support EIP-7594 format yet
                if sidecar.blobs.len() > 0 {
                    tracing::debug!("Allowing EIP-4844 blob tx post-Osaka (EIP-7594 not implemented in reth yet)");
                } else {
                    tracing::debug!("Allowing regular tx with EIP-4844 sidecar post-Osaka");
                }
                true
            }
            BlobTransactionSidecarVariant::Eip7594(_sidecar) => {
                // Allow EIP-7594 sidecars when they eventually exist
                tracing::debug!("Allowing EIP-7594 sidecar post-Osaka");
                true
            }
        }
    })
}

impl<FilterFunc: Fn(&BlobTransactionSidecarVariant) -> bool> BlobTypeOrderFilter<FilterFunc> {
    fn new(sink: Box<dyn ReplaceableOrderSink>, filter_func: FilterFunc) -> Self {
        Self { sink, filter_func }
    }
}

impl<FilterFunc: Fn(&BlobTransactionSidecarVariant) -> bool + Send + Sync> ReplaceableOrderSink
    for BlobTypeOrderFilter<FilterFunc>
{
    fn insert_order(&mut self, order: Order) -> bool {
        if order
            .list_txs()
            .iter()
            .all(|(tx, _)| (self.filter_func)(tx.blobs_sidecar.as_ref()))
        {
            self.sink.insert_order(order)
        } else {
            true
        }
    }

    fn remove_bundle(&mut self, replacement_data: BundleReplacementData) -> bool {
        self.sink.remove_bundle(replacement_data)
    }

    fn remove_sbundle(&mut self, key: ShareBundleReplacementKey) -> bool {
        self.sink.remove_sbundle(key)
    }

    fn is_alive(&self) -> bool {
        self.sink.is_alive()
    }
}
