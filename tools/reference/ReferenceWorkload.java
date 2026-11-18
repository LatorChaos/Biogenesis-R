package biogenesis;

import java.awt.Frame;
import java.awt.Window;
import java.io.ByteArrayOutputStream;
import java.io.ObjectOutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.concurrent.FutureTask;
import java.util.prefs.Preferences;
import javax.swing.SwingUtilities;

/** Measure actual world frames separately from real-control journeys */
public final class ReferenceWorkload {
    private static MainWindow window;

    public static void main(String[] args) throws Exception {
        if (args.length != 2 || !"1".equals(System.getenv("BGR_ISOLATED_XVFB"))
                || System.getenv("DISPLAY") == null || ":0".equals(System.getenv("DISPLAY"))) {
            throw new IllegalArgumentException("Expected population and empty fixture directory on isolated Xvfb");
        }
        int attempts = Integer.parseInt(args[0]);
        if (attempts != 0 && attempts != 32 && attempts != 512 && attempts != 12500) {
            throw new IllegalArgumentException("Unspecified reference workload");
        }
        Path output = Path.of(args[1]).toAbsolutePath();
        Files.createDirectories(output);
        try (var files = Files.list(output)) {
            if (files.findAny().isPresent()) throw new IllegalArgumentException("Fixture directory must be empty");
        }
        Path root = output.getParent().toRealPath();
        for (String key : new String[]{"user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot"}) {
            String value = System.getProperty(key);
            if (value == null || !Path.of(value).toRealPath().startsWith(root) || Path.of(value).toRealPath().equals(root)) {
                throw new IllegalArgumentException("Missing isolated profile: " + key);
            }
        }
        try {
            Preferences preferences = Preferences.userNodeForPackage(Utils.class);
            preferences.putInt("VERSION", 700);
            preferences.putInt("INITIAL_ORGANISMS", attempts);
            preferences.putInt("WORLD_WIDTH", 7000);
            preferences.putInt("WORLD_HEIGHT", 7000);
            preferences.putBoolean("ACCEPT_CONNECTIONS", false);
            preferences.putBoolean("CONNECT_TO_SERVER", false);
            preferences.putBoolean("AUTO_BACKUP", false);
            preferences.putInt("HARDWARE_ACCELERATION", 0);
            preferences.putInt("DELAY", 20);
            preferences.put("LOCALE", "en");
            preferences.flush();
            long startup = System.nanoTime();
            SwingUtilities.invokeAndWait(() -> MainWindow.main(new String[]{"827364"}));
            for (Frame frame : Frame.getFrames()) if (frame instanceof MainWindow main) window = main;
            if (window == null || window.isProcessActive()) throw new IllegalStateException("Invalid paused reference entry point");
            long startup_ns = System.nanoTime() - startup;
            FutureTask<String> measurement = new FutureTask<>(() -> {
                World world = window.getWorld();
                for (int index = 0; index < 16; index++) world.time();
                long[] times = new long[64];
                for (int index = 0; index < times.length; index++) {
                    long begin = System.nanoTime();
                    world.time();
                    times[index] = System.nanoTime() - begin;
                }
                if (world.getTime() != 0 || world.getFrame() != 80) throw new IllegalStateException("Frame ordering differs from workload");
                ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                try (ObjectOutputStream stream = new ObjectOutputStream(bytes)) { stream.writeObject(world); }
                byte[] snapshot = bytes.toByteArray();
                Files.write(output.resolve("world-after-80.bgw"), snapshot, StandardOpenOption.CREATE_NEW);
                String digest = HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(snapshot));
                long sum = Arrays.stream(times).sum();
                Arrays.sort(times);
                Runtime runtime = Runtime.getRuntime();
                return "{\"attempts\":" + attempts + ",\"population\":" + world.getPopulation()
                        + ",\"world_width\":7000,\"world_height\":7000,\"seed\":827364,\"warmup_frames\":16,\"measured_frames\":64"
                        + ",\"startup_ns\":" + startup_ns + ",\"mean_frame_ns\":" + sum / times.length
                        + ",\"p95_frame_ns\":" + times[60] + ",\"max_frame_ns\":" + times[63]
                        + ",\"heap_used_bytes\":" + (runtime.totalMemory() - runtime.freeMemory())
                        + ",\"heap_limit_bytes\":" + runtime.maxMemory() + ",\"available_processors\":" + runtime.availableProcessors()
                        + ",\"snapshot_bytes\":" + snapshot.length + ",\"snapshot_sha256\":\"" + digest + "\""
                        + ",\"co2\":\"" + Double.toHexString(world.getCO2()) + "\",\"o2\":\"" + Double.toHexString(world.getO2())
                        + "\",\"ch4\":\"" + Double.toHexString(world.getCH4()) + "\",\"frame_timing_includes_rendering\":false}";
            });
            SwingUtilities.invokeAndWait(measurement);
            System.out.println(measurement.get());
        } catch (Throwable error) {
            error.printStackTrace();
            System.exit(1);
        } finally {
            if (window != null && window._timer != null) window._timer.cancel();
            SwingUtilities.invokeAndWait(() -> {
                for (Window target : Window.getWindows()) target.dispose();
            });
        }
    }
}
