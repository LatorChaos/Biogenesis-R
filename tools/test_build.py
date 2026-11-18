"""Check build invocation, cache provenance, and isolated output boundaries"""

from contextlib import redirect_stderr, redirect_stdout
import hashlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

import build

sys.dont_write_bytecode = True
from build import cargo_arguments, ensure_extension_entry, environment, godot_errors, index_key, local_path, neutral_directory, seed_cache
from run_reference_oracles import run_command


class BuildTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="bgr-build-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def test_argv_keeps_shell_metacharacters_literal(self):
        root = self.root / "project # with $(touch marker)"
        arguments = cargo_arguments(Path("/tools/cargo"), "test", root, root / "output", True)
        with redirect_stdout(io.StringIO()):
            observed = json.loads(run_command([sys.executable, "-c", "import json,sys;print(json.dumps(sys.argv[1:]))",
                                              *arguments], self.root, [], timeout=3))
        self.assertEqual(observed[0], "/tools/cargo")
        self.assertEqual(observed[observed.index("--manifest-path") + 1], str(root / "Cargo.toml"))
        self.assertEqual(observed[observed.index("--target-dir") + 1], str(root / "output"))
        self.assertIn("--offline", observed)
        self.assertIn("--locked", observed)
        self.assertFalse((self.root / "marker").exists())

    def test_environment_rejects_compiler_wrappers_and_flags(self):
        for key in ["RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER",
                    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER", "CARGO_PROFILE_RELEASE_PANIC", "LD_PRELOAD"]:
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "invocation overrides"):
                environment({key: "unwanted"}, self.root, self.root, self.root, {})

    def test_environment_uses_private_profiles_and_explicit_tools(self):
        tools = {"cargo": Path("/chosen/cargo"), "rustc": Path("/chosen/rustc"), "rustdoc": Path("/chosen/rustdoc")}
        env = environment({"HOME": "/real/home", "PATH": "/untrusted", "DISPLAY": ":5", "SECRET": "hidden",
                           "RUST_TEST_THREADS": "8"},
                          self.root / "profile", self.root / "cargo", self.root / "target", tools)
        self.assertEqual(env["HOME"], str(self.root / "profile/home"))
        self.assertEqual(env["CARGO_HOME"], str(self.root / "cargo"))
        self.assertEqual(env["RUSTC"], "/chosen/rustc")
        self.assertEqual(env["DISPLAY"], ":5")
        self.assertEqual(env["CARGO_BUILD_JOBS"], "1")
        self.assertEqual(env["RUST_TEST_THREADS"], "1")
        self.assertNotIn("SECRET", env)
        self.assertNotIn("/untrusted", env["PATH"])

    def native_publication(self, machine, operation="build", malformed=False):
        project = self.root / (str(machine) + "-" + operation + "-" + str(malformed))
        project.mkdir()
        (project / "Cargo.toml").write_text("[workspace]\n")
        (project / "Cargo.lock").write_text("version=4\n")
        (project / "bgr.gdextension").write_text('[configuration]\nentry_symbol="bgr_init"\n')
        (project / "bin").mkdir()
        x86 = project / "bin/libbiogenesis_godot.so"
        arm = project / "bin/libbiogenesis_godot.arm64.so"
        x86.write_bytes(b"preserved x86")
        arm.write_bytes(b"preserved ARM")
        header = bytearray(20)
        header[:7] = b"\x7fELF\x02\x01\x01"
        header[16:18] = (3).to_bytes(2, "little")
        header[18:20] = machine.to_bytes(2, "little")
        payload = b"malformed" if malformed else bytes(header) + b"synthetic shared-library fixture"
        calls = []
        published = arm if machine == 183 else x86

        def command(arguments, run, receipts, directory=None, env=None, timeout=40):
            name = Path(arguments[0]).name
            calls.append((list(arguments), dict(env)))
            receipts.append({"stderr": "", "exit_code": 0, "timed_out": False})
            if "--version" in arguments:
                return "4.7.2.stable.synthetic" if name == "godot" else name + " 1.96.0"
            if name == "cargo":
                self.assertEqual(arguments[1], "build")
                target = Path(arguments[arguments.index("--target-dir") + 1])
                library = target / ("release" if "--release" in arguments else "debug") / "libbiogenesis_godot.so"
                library.parent.mkdir(parents=True)
                library.write_bytes(payload)
                return ""
            self.assertEqual(name, "godot")
            self.assertEqual(published.read_bytes(), payload)
            self.assertEqual((project / ".godot/extension_list.cfg").read_bytes(), b"res://bgr.gdextension\n")
            return ""

        arguments = [operation] + (["--release"] if operation == "build" else [])
        with mock.patch.object(build, "ROOT", project), \
                mock.patch.object(build, "executable", side_effect=lambda value, name: Path("/tools") / name), \
                mock.patch.object(build, "seed_cache", return_value=0), \
                mock.patch.object(build, "run_command", side_effect=command), \
                mock.patch.dict(build.os.environ, {}, clear=True), \
                redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            result = build.main(arguments)
        self.assertTrue(calls)
        for _, env in calls:
            self.assertEqual(env["CARGO_BUILD_JOBS"], "1")
            self.assertEqual(env["RUST_TEST_THREADS"], "1")
        return result, project, payload

    def test_native_arm_build_boundary_and_run_publish_descriptor_library(self):
        for operation in ["build", "boundary", "run"]:
            with self.subTest(operation=operation):
                result, project, payload = self.native_publication(183, operation)
                self.assertEqual(result, 0)
                self.assertEqual((project / "bin/libbiogenesis_godot.arm64.so").read_bytes(), payload)
                self.assertEqual((project / "bin/libbiogenesis_godot.so").read_bytes(), b"preserved x86")
                self.assertEqual(list((project / "bin").glob("*.tmp")), [])

    def test_native_x86_publication_keeps_existing_descriptor_path(self):
        result, project, payload = self.native_publication(62)
        self.assertEqual(result, 0)
        self.assertEqual((project / "bin/libbiogenesis_godot.so").read_bytes(), payload)
        self.assertEqual((project / "bin/libbiogenesis_godot.arm64.so").read_bytes(), b"preserved ARM")
        self.assertEqual(list((project / "bin").glob("*.tmp")), [])

    def test_invalid_adapter_architecture_preserves_both_existing_libraries(self):
        for machine, malformed in [(40, False), (183, True)]:
            with self.subTest(machine=machine, malformed=malformed):
                result, project, _ = self.native_publication(machine, malformed=malformed)
                self.assertEqual(result, 1)
                self.assertEqual((project / "bin/libbiogenesis_godot.so").read_bytes(), b"preserved x86")
                self.assertEqual((project / "bin/libbiogenesis_godot.arm64.so").read_bytes(), b"preserved ARM")
                self.assertEqual(list((project / "bin").glob("*.tmp")), [])

    def test_symlink_outputs_and_neutral_cargo_configuration_are_rejected(self):
        outside = self.root / "outside"
        outside.mkdir()
        project = self.root / "project"
        project.mkdir()
        (project / "cache").symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "escapes project"):
            local_path(project, project / "cache/generated")
        config = self.root / ".cargo"
        config.mkdir()
        (config / "config.toml").write_text('[build]\nrustc-wrapper="unwanted"\n')
        with self.assertRaisesRegex(ValueError, "Cargo configuration"):
            neutral_directory(project)

    def test_profile_children_cannot_redirect_user_data(self):
        profile, real = self.root / "profile", self.root / "real-profile"
        profile.mkdir()
        real.mkdir()
        (profile / "home").symlink_to(real, target_is_directory=True)
        tools = {"rustc": Path("/chosen/rustc"), "rustdoc": Path("/chosen/rustdoc")}
        with self.assertRaisesRegex(ValueError, "escapes project"):
            environment({}, profile, self.root / "cargo", self.root / "target", tools)
        self.assertEqual(list(real.iterdir()), [])

    def test_colored_engine_errors_are_failures_even_with_success_exit(self):
        self.assertTrue(godot_errors("Godot Engine\n\x1b[91mERROR:\x1b[0m extension failed\n"))
        self.assertTrue(godot_errors("\x1b[1;31mSCRIPT ERROR:\x1b[0m Parse Error\n"))
        self.assertFalse(godot_errors("BGR_BOUNDARY_CHECK {\"result\":\"PASS\"}\n"))

    def test_cache_requires_lock_checksum_and_never_copies_configuration(self):
        source, target = self.root / "source", self.root / "private"
        archive = source / "registry/cache/index-test/demo-1.0.0.crate"
        archive.parent.mkdir(parents=True)
        archive.write_bytes(b"crate fixture")
        checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
        index = source / "registry/index/index-test"
        (index / ".cache/de/mo").mkdir(parents=True)
        (index / ".cache/de/mo/demo").write_bytes(b"index fixture")
        (index / "config.json").write_text('{}\n')
        (source / "config.toml").write_text('[build]\nrustc-wrapper="unwanted"\n')
        (source / "credentials.toml").write_text('token="fixture"\n')
        lock = self.root / "Cargo.lock"
        lock.write_text('version=4\n[[package]]\nname="demo"\nversion="1.0.0"\n'
                        'source="registry+https://github.com/rust-lang/crates.io-index"\n'
                        f'checksum="{checksum}"\n')
        before = {path: (path.read_bytes(), path.stat().st_mtime_ns) for path in source.rglob("*") if path.is_file()}
        self.assertEqual(seed_cache(self.root, source, target, lock), 1)
        self.assertEqual(seed_cache(self.root, source, target, lock), 1)
        self.assertFalse((target / "config.toml").exists())
        self.assertFalse((target / "credentials.toml").exists())
        self.assertEqual({path: (path.read_bytes(), path.stat().st_mtime_ns) for path in before}, before)
        archive.write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "checksum-matched"):
            seed_cache(self.root, source, target, lock)

    def test_sparse_index_paths_cover_short_names(self):
        self.assertEqual([index_key(name) for name in ["a", "ab", "abc", "serde"]],
                         ["1/a", "2/ab", "3/a/abc", "se/rd/serde"])

    def test_extension_bootstrap_preserves_entries_and_is_idempotent(self):
        (self.root / "bgr.gdextension").write_text('[configuration]\nentry_symbol="bgr_init"\n')
        cache = self.root / ".godot"
        cache.mkdir()
        listing = cache / "extension_list.cfg"
        original = b"res://other.gdextension\r\n\nres://another.gdextension"
        listing.write_bytes(original)
        listing.chmod(0o640)
        self.assertTrue(ensure_extension_entry(self.root))
        self.assertEqual(listing.read_bytes(), original + b"\nres://bgr.gdextension\n")
        before = listing.stat()
        self.assertEqual(before.st_mode & 0o777, 0o640)
        self.assertFalse(ensure_extension_entry(self.root))
        self.assertEqual(listing.stat().st_mtime_ns, before.st_mtime_ns)
        self.assertEqual(listing.read_bytes().count(b"res://bgr.gdextension"), 1)
        self.assertEqual(list(cache.glob("extension-list-*")), [])

    def test_extension_bootstrap_creates_only_the_owned_entry(self):
        (self.root / "bgr.gdextension").write_text('[configuration]\nentry_symbol="bgr_init"\n')
        self.assertTrue(ensure_extension_entry(self.root))
        listing = self.root / ".godot/extension_list.cfg"
        self.assertEqual(listing.read_bytes(), b"res://bgr.gdextension\n")
        listing.write_bytes(b"  res://bgr.gdextension  \r\nres://other.gdextension\n")
        original = listing.read_bytes()
        self.assertFalse(ensure_extension_entry(self.root))
        self.assertEqual(listing.read_bytes(), original)

    def test_extension_bootstrap_rejects_symlinked_cache_or_list(self):
        project, outside = self.root / "project", self.root / "outside"
        project.mkdir()
        outside.mkdir()
        (project / "bgr.gdextension").write_text('[configuration]\nentry_symbol="bgr_init"\n')
        cache = project / ".godot"
        cache.symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "escapes project"):
            ensure_extension_entry(project)
        self.assertEqual(list(outside.iterdir()), [])
        cache.unlink()
        cache.mkdir()
        protected = outside / "list"
        protected.write_bytes(b"preserved")
        (cache / "extension_list.cfg").symlink_to(protected)
        with self.assertRaisesRegex(ValueError, "escapes project"):
            ensure_extension_entry(project)
        self.assertEqual(protected.read_bytes(), b"preserved")


if __name__ == "__main__":
    unittest.main()
