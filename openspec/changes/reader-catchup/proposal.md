## Why
Information volume makes a chronological list alone insufficient for catching up. Readers need a finite selection with visible reasons, related coverage and changes to previously read articles.

## What Changes
- Implement Astra's initial plan in `docs/reader-catchup-plan.md`: article revisions, conservative related-article grouping, shared explicit interests and a finite catch-up queue.
- Offer 10 or 20 candidates for 24 hours or 7 days, with mute filtering, collection-coverage limits, source diversity and explanations.
- Keep confirmation/skipping independent from read/star/visit/read-later states and scoped to the exact observed revision.
- Integrate these controls into the React Reader while preserving the ordinary list.

## Impact
New Rust feature modules, additive v10 storage, React panels and synthetic acceptance. Collection reliability has a separate change. Existing Web article, visit, mute, source and settings contracts remain the baseline. Newsletter ingestion, external source adapters, external notifications and AI translation/summary remain subsequent release stages.
