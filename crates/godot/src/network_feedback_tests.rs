// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

#[test]
fn network_notice_codes_retain_raw_addresses_and_listener_limits() {
    let address: SocketAddr = "[::1]:65535".parse().unwrap();
    let cases = [
        (NetworkFeedback::Listening(0), "listening", "Listening on port 0; incoming ownership is limited to 8 genomes per peer"),
        (NetworkFeedback::Listening(65535), "listening", "Listening on port 65535; incoming ownership is limited to 8 genomes per peer"),
        (NetworkFeedback::Connecting(address), "connecting", "Connecting to [::1]:65535"),
        (NetworkFeedback::Connected(address), "connected", "Connected with [::1]:65535"),
        (NetworkFeedback::Closed(address), "closed", "Connection closed with [::1]:65535"),
        (NetworkFeedback::Closing, "closing", "Closing network connections"),
        (NetworkFeedback::AllClosed, "all_closed", "Network connections closed"),
        (NetworkFeedback::Cancelled, "cancelled", "Remaining network notifications were cancelled"),
    ];
    for (feedback, code, raw) in cases {
        assert_eq!(feedback.code(), code);
        assert_eq!(feedback.raw(), raw);
        assert_eq!(feedback.detail(), "");
    }
}

#[test]
fn network_failure_codes_retain_guidance_arguments_and_opaque_diagnostics() {
    let cases = [
        (NetworkFeedback::InvalidLocalPort, "invalid_local_port", "Local port must be between 0 and 65535 to start a listener"),
        (NetworkFeedback::InvalidRemotePort, "invalid_remote_port", "Remote port must be a positive Java integer between 1 and 65535"),
        (NetworkFeedback::InvalidAddress, "invalid_address", "A resolved IP address is required"),
        (NetworkFeedback::InvalidIdentity, "invalid_identity", "A valid connection identity is required"),
        (NetworkFeedback::AlreadyClosed, "already_closed", "This connection has already closed"),
        (NetworkFeedback::WorldRequired, "world_required", "Initialize a world before connecting"),
        (NetworkFeedback::ClosingPrevious, "closing_previous", "Previous network connections are still closing; retry after cleanup"),
        (NetworkFeedback::Duplicate, "duplicate", "This address and port already has a connection"),
        (NetworkFeedback::Capacity, "capacity", "Network connection or socket worker capacity is full"),
        (NetworkFeedback::WorkerCapacity, "worker_capacity", "Network socket worker capacity is full"),
        (NetworkFeedback::Refused(16), "refused", "Connection refused with protocol reply 16"),
    ];
    for (feedback, code, raw) in cases {
        assert_eq!(feedback.code(), code);
        assert_eq!(feedback.raw(), raw);
        assert_eq!(feedback.detail(), "");
    }
    let detail = "opaque: Deadline { shutdown_error: None }\nport=4321";
    let cases = [
        (NetworkFeedback::Listener(detail.into()), "listener_failed", detail.to_owned()),
        (NetworkFeedback::Transfer(detail.into()), "transfer_failed", detail.to_owned()),
        (NetworkFeedback::Incoming(detail.into()), "incoming_failed", format!("Incoming transfer was not acknowledged: {detail}")),
        (NetworkFeedback::PreferencesSave(detail.into()), "preferences_failed", format!("Network settings changed; preference save failed: {detail}. Listener port was retained")),
        (NetworkFeedback::Cleanup(detail.into()), "cleanup_failed", detail.to_owned()),
        (NetworkFeedback::from(detail.to_owned()), "operation_failed", detail.to_owned()),
    ];
    for (feedback, code, raw) in cases {
        assert_eq!(feedback.code(), code);
        assert_eq!(feedback.raw(), raw);
        assert_eq!(feedback.detail(), detail);
    }
}

#[test]
fn network_failure_replacement_and_clear_keep_raw_and_typed_state_together() {
    let mut network = Network::default();
    assert_eq!(network.listen(-1).unwrap_err(), NetworkFeedback::InvalidLocalPort);
    assert_eq!(network.disconnect(1).unwrap_err(), NetworkFeedback::AlreadyClosed);
    network.fail(NetworkFeedback::Refused(4));
    assert_eq!(network.error, "Connection refused with protocol reply 4");
    assert_eq!(network.error_feedback, Some(NetworkFeedback::Refused(4)));
    network.fail("opaque worker diagnostic");
    assert_eq!(network.error, "opaque worker diagnostic");
    assert_eq!(network.error_feedback, Some(NetworkFeedback::Operation("opaque worker diagnostic".into())));
    let revision = network.revision;
    network.clear_error();
    assert!(network.error.is_empty());
    assert!(network.error_feedback.is_none());
    assert_eq!(network.revision, revision);
}

#[test]
fn idle_stop_starts_clean_status_after_historical_command_errors() {
    for feedback in [
        NetworkFeedback::Duplicate,
        NetworkFeedback::InvalidRemotePort,
        NetworkFeedback::Refused(16),
    ] {
        let mut network = Network::default();
        network.fail(feedback);
        assert!(!network.error.is_empty());
        assert!(network.error_feedback.is_some());
        let generation = network.generation;
        network.stop();
        assert!(network.error.is_empty(), "{}", network.error);
        assert!(network.error_feedback.is_none());
        assert!(network.cleanup_error.is_none());
        assert_eq!(network.generation, generation);
        assert!(network.poll_close());
        network.stop();
        assert!(network.error.is_empty());
        assert!(network.error_feedback.is_none());
    }
}

#[test]
fn stop_and_poll_retain_prior_cleanup_failure_after_command_feedback() {
    for active in [false, true] {
        let world = World::new(WorldConfig {
            seed: 14721,
            width: 101,
            height: 101,
            initial_attempts: 0,
        }).unwrap();
        let mut network = Network::default();
        if active {
            network.root = Some(world.handle());
        }
        let diagnostic = "owned cleanup failure";
        network.cleanup_failed(diagnostic.to_owned());
        network.fail(NetworkFeedback::Duplicate);
        network.stop();
        assert_eq!(network.cleanup_error.as_deref(), Some(diagnostic));
        assert_eq!(network.error, diagnostic);
        assert_eq!(network.error_feedback, Some(NetworkFeedback::Cleanup(diagnostic.to_owned())));
        assert!(network.poll_close());
        network.fail(NetworkFeedback::InvalidIdentity);
        network.stop();
        assert!(network.poll_close());
        assert_eq!(network.cleanup_error.as_deref(), Some(diagnostic));
        assert_eq!(network.error, diagnostic);
        assert_eq!(network.error_feedback, Some(NetworkFeedback::Cleanup(diagnostic.to_owned())));
        assert!(network.root.is_none());
    }
}
