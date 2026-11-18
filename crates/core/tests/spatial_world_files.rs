// SPDX-License-Identifier: GPL-2.0-or-later

use biogenesis_core::object_stream::Limits;
use biogenesis_core::{World, WorldConfig};
use std::path::Path;

#[test]
fn authored_spatial_alias_worlds_load_without_changing_the_caller() {
    let caller = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
    let owner = caller.handle();
    let session = caller.session();
    let random = caller.random_snapshot();
    let limits = Limits::default();
    let mut rejected = Vec::new();
    for case in ["tree_history", "pending_history_build", "child_history_cache"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/shared_world_owners")
            .join(format!("{case}-authored.ser"));
        match caller.load_legacy_file(&path, limits, limits.allocated_bytes) {
            Ok(world) => {
                assert!(!world.handle().same_owner(&owner));
                assert!(world.session().same_owner(&session));
            }
            Err(error) => rejected.push((case, format!("{error:?}"))),
        }
        assert!(caller.handle().same_owner(&owner));
        assert!(caller.session().same_owner(&session));
        assert_eq!(caller.random_snapshot(), random);
    }
    assert!(rejected.is_empty(), "Authored spatial worlds rejected: {rejected:?}");
}
