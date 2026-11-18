"""Build and exercise the local Rust/Godot application without shared cache writes"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import tempfile
import tomllib

sys.dont_write_bytecode = True
from run_reference_oracles import run_command


ROOT = Path(__file__).resolve().parents[1]
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
UNSAFE_PREFIXES = ("CARGO_BUILD_", "CARGO_TARGET_", "CARGO_PROFILE_", "CARGO_ALIAS_",
                   "CARGO_SOURCE_", "CARGO_REGISTR", "GDRUST_", "LD_", "DYLD_")
UNSAFE_KEYS = {"RUSTFLAGS", "RUSTDOCFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_ENCODED_RUSTDOCFLAGS",
               "RUSTC", "RUSTDOC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTUP_TOOLCHAIN"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def local_path(root, path):
    """Keep generated paths inside the checkout, including existing symlink parents"""
    path = Path(path)
    require(path.resolve().is_relative_to(root.resolve()), "Output escapes project: " + str(path))
    for part in [path, *path.parents]:
        if part == root:
            break
        require(not part.is_symlink(), "Output uses a symlink: " + str(part))
    return path


def executable(value, name):
    located = shutil.which(value or name)
    require(located is not None, "Missing executable: " + (value or name))
    path = Path(located).absolute()
    require(path.is_file() and os.access(path, os.X_OK), "Not executable: " + str(path))
    return path


def environment(inherited, profile, cargo_home, target, tools):
    blocked = sorted(key for key, value in inherited.items() if value and
                     (key in UNSAFE_KEYS or key.startswith(UNSAFE_PREFIXES)))
    require(not blocked, "Remove invocation overrides: " + ", ".join(blocked))
    kept = ("LANG", "LC_ALL", "LC_CTYPE", "TZ", "TERM", "DISPLAY", "WAYLAND_DISPLAY",
            "XAUTHORITY", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS")
    env = {key: inherited[key] for key in kept if key in inherited}
    for variable, folder in [("HOME", "home"), ("XDG_CONFIG_HOME", "config"),
                             ("XDG_CACHE_HOME", "cache"), ("XDG_DATA_HOME", "data"),
                             ("TMPDIR", "tmp")]:
        path = local_path(profile, profile / folder)
        path.mkdir(parents=True, exist_ok=True)
        env[variable] = str(path)
    env.update(CARGO_HOME=str(cargo_home), CARGO_TARGET_DIR=str(target), CARGO_NET_OFFLINE="true",
               RUSTUP_HOME=inherited.get("RUSTUP_HOME", str(Path.home() / ".rustup")),
               RUSTUP_AUTO_INSTALL="0", RUSTC=str(tools["rustc"]), RUSTDOC=str(tools["rustdoc"]),
               PYTHONDONTWRITEBYTECODE="1", CARGO_TERM_COLOR="never")
    directories = [str(path.parent) for path in tools.values()]
    env["PATH"] = os.pathsep.join(dict.fromkeys([*directories, *os.defpath.split(os.pathsep)]))
    env.update(CARGO_BUILD_JOBS="1", RUST_TEST_THREADS="1")
    return env


def index_key(name):
    if len(name) == 1:
        return "1/" + name
    if len(name) == 2:
        return "2/" + name
    if len(name) == 3:
        return "3/" + name[0] + "/" + name
    return name[:2] + "/" + name[2:4] + "/" + name


def seed_cache(root, source, destination, lock):
    """Copy only lockfile archives and sparse index entries, without credentials or configuration"""
    destination = local_path(root, destination)
    for name in ["config", "config.toml", "credentials", "credentials.toml"]:
        require(not (destination / name).exists(), "Private Cargo home contains " + name)
    packages = tomllib.loads(lock.read_text())["package"]
    count = 0
    for package in packages:
        if "source" not in package:
            continue
        require(package["source"] == REGISTRY, "Unsupported dependency source: " + package["source"])
        name, version, checksum = package["name"], package["version"], package["checksum"]
        require(re.fullmatch(r"[A-Za-z0-9_-]+", name) is not None, "Invalid crate name")
        require(re.fullmatch(r"[A-Za-z0-9.+_-]+", version) is not None, "Invalid crate version")
        matches = sorted(source.glob(f"registry/cache/*/{name}-{version}.crate"))
        archive = next((path for path in matches if hashlib.sha256(path.read_bytes()).hexdigest() == checksum), None)
        require(archive is not None, f"Missing checksum-matched cached crate: {name} {version}; use --cache-source")
        registry = archive.parent.name
        for relative in [Path("cache") / registry / archive.name,
                         Path("index") / registry / "config.json",
                         Path("index") / registry / ".cache" / index_key(name)]:
            origin = source / "registry" / relative
            require(origin.is_file(), "Missing cached registry input: " + str(origin))
            output = local_path(root, destination / "registry" / relative)
            output.parent.mkdir(parents=True, exist_ok=True)
            if not output.exists() or output.read_bytes() != origin.read_bytes():
                shutil.copyfile(origin, output)
        count += 1
    return count


def cargo_arguments(cargo, command, root, target, release=False):
    arguments = [str(cargo), command, "--workspace", "--offline", "--locked", "--manifest-path",
                 str(root / "Cargo.toml"), "--target-dir", str(target)]
    if release:
        arguments.append("--release")
    return arguments


def neutral_directory(path):
    for parent in [path, *path.parents]:
        for name in ["config", "config.toml"]:
            require(not (parent / ".cargo" / name).exists(), "Cargo configuration affects neutral directory: " + str(parent))


def godot_errors(text):
    text = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text)
    return re.search(r"(^|\n)(?:SCRIPT ERROR|ERROR):", text) is not None



def extension_library_name(library):
    """Select the descriptor path from the actual ELF architecture"""
    with library.open("rb") as stream:
        header = stream.read(20)
    require(len(header) == 20 and header[:6] == b"\x7fELF\x02\x01",
            "Adapter library must be a 64-bit little-endian ELF")
    require(int.from_bytes(header[16:18], "little") == 3, "Adapter library must be a shared ELF")
    machine = int.from_bytes(header[18:20], "little")
    names = {62: "libbiogenesis_godot.so", 183: "libbiogenesis_godot.arm64.so"}
    require(machine in names, "Unsupported adapter ELF architecture: " + str(machine))
    return names[machine]


def ensure_extension_entry(root):
    """Preload the owned extension to avoid the observed cold-import crash"""
    require((root / "bgr.gdextension").is_file(), "Missing owned extension descriptor")
    path = local_path(root, root / ".godot/extension_list.cfg")
    before = path.read_bytes() if path.exists() else b""
    entry = b"res://bgr.gdextension"
    if entry in [line.strip() for line in before.splitlines()]:
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    separator = b"\n" if before and not before.endswith(b"\n") else b""
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix="extension-list-", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(before + separator + entry + b"\n")
        if path.exists():
            shutil.copymode(path, temporary)
        temporary.replace(path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    return True


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["build", "test", "boundary", "run"])
    for name in ["cargo", "rustc", "rustdoc", "godot"]:
        parser.add_argument("--" + name, help="Executable path or discoverable command name")
    parser.add_argument("--cache-source", type=Path, default=Path.home() / ".cargo",
                        help="Existing archive cache for seeding; no configuration or credentials are copied")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--timeout", type=float, default=180, help="Maximum seconds for each child command")
    args = parser.parse_args(argv)
    require(0 < args.timeout <= 3600, "Timeout must be positive and at most 3600 seconds")
    require(sys.platform.startswith("linux"), "This helper currently supports native Linux builds")
    names = ["cargo", "rustc", "rustdoc"] + (["godot"] if args.command in {"boundary", "run"} else [])
    tools = {name: executable(getattr(args, name), name) for name in names}
    require((ROOT / "Cargo.toml").is_file() and (ROOT / "Cargo.lock").is_file(), "Workspace manifest and lockfile are required")
    local = local_path(ROOT, ROOT / ".workflow-local")
    runs = local_path(ROOT, local / "build-runs")
    runs.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix=args.command + "-", dir=runs))
    receipts = []
    print(json.dumps({"run_root": str(run)}), flush=True)
    cargo_home, target = local / "cargo-home", local / "target"
    profile = local_path(ROOT, local / "runtime" if args.command == "run" else run / "profile")
    env = environment(os.environ, profile, local_path(ROOT, cargo_home), local_path(ROOT, target), tools)
    count = seed_cache(ROOT, args.cache_source.resolve(), cargo_home, ROOT / "Cargo.lock")

    def terminate(signum, _frame):
        raise SystemExit(128 + signum)

    previous = signal.signal(signal.SIGTERM, terminate)
    try:
        with tempfile.TemporaryDirectory(prefix="bgr-cargo-") as temporary:
            working = Path(temporary)
            neutral_directory(working)

            def command(arguments, godot=False):
                output = run_command(arguments, run, receipts, directory=working, env=env, timeout=args.timeout)
                if godot:
                    text = output + receipts[-1]["stderr"]
                    require(not godot_errors(text),
                            "Godot reported an error; inspect " + str(run / "commands.json"))
                return output.strip()

            versions = {}
            for name in names:
                versions[name] = command([str(tools[name]), "--version"])
                if name == "godot":
                    require(versions[name].startswith("4.7.2.stable."), "Godot 4.7.2 stable is required")
                else:
                    match = re.match(name + r" (\d+)\.(\d+)\.", versions[name])
                    require(match is not None and tuple(map(int, match.groups())) >= (1, 94), "Rust/Cargo 1.94 or newer is required")
            operation = "test" if args.command == "test" else "build"
            command(cargo_arguments(tools["cargo"], operation, ROOT, target, args.release))
            if args.command != "test":
                library = target / ("release" if args.release else "debug") / "libbiogenesis_godot.so"
                require(library.is_file(), "Cargo did not create the expected adapter library")
                output = local_path(ROOT, ROOT / "bin" / extension_library_name(library))
                output.parent.mkdir(exist_ok=True)
                temporary_library = local_path(ROOT, output.with_suffix(".so.tmp"))
                shutil.copyfile(library, temporary_library)
                temporary_library.replace(output)
            if args.command in {"boundary", "run"}:
                bootstrapped = ensure_extension_entry(ROOT)
                base = [str(tools["godot"]), "--path", str(ROOT), "--log-file", str(run / "godot.log")]
                command([*base, "--headless", "--editor", "--import", "--quit"], godot=True)
                extra = ["--headless", "--script", "res://ui/boundary_check.gd"] if args.command == "boundary" else []
                command([*base, *extra], godot=True)
            result = {"result": "PASS", "command": args.command, "versions": versions,
                      "registry_dependencies": count, "profile": str(profile), "target": str(target)}
            if args.command in {"boundary", "run"}:
                result["extension_list_bootstrapped"] = bootstrapped
            (run / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
            print(json.dumps(result), flush=True)
            return 0
    except ValueError as error:
        print(str(error), file=sys.stderr)
        if receipts and receipts[-1]["timed_out"]:
            return 124
        status = receipts[-1]["exit_code"] if receipts else 1
        return status if status and status > 0 else (128 - status if status and status < 0 else 1)
    finally:
        signal.signal(signal.SIGTERM, previous)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
