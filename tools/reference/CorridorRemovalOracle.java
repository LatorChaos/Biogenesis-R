// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Frame;
import java.awt.Graphics;
import java.awt.Rectangle;
import java.awt.Window;
import java.awt.image.BufferedImage;
import java.io.OutputStream;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Queue;
import java.util.Random;
import java.util.concurrent.FutureTask;
import java.util.prefs.Preferences;
import javax.swing.SwingUtilities;

/** Capture actual finite World.removeCorridors and downstream effects */
public final class CorridorRemovalOracle {
    private static final String HEADER = "kind\tcase\tstage\tobject\tfield\tvalue\textra1\textra2";
    private static final List<String> rows = new ArrayList<>();
    private static final IdentityHashMap<Object, String> labels = new IdentityHashMap<>();
    private static MainWindow window;
    private static String caseName = "bootstrap";
    private static String stage = "bootstrap";
    private static int caseCount;
    private static int admissionCount;
    private static int repaintOrder;
    private static boolean recordAdmissions;

    private static final class TraceVisibleWorld extends VisibleWorld {
        private boolean captureRepaints;

        TraceVisibleWorld(MainWindow main) {
            super(main);
        }

        @Override
        public void repaint(Rectangle bounds) {
            if (!captureRepaints) return;
            row("REPAINT", stage, label(bounds), "order", Integer.toString(repaintOrder++), rect(bounds), "");
        }
    }

    private static final class TraceWorld extends World {
        TraceWorld(VisibleWorld visible) {
            super(visible);
        }

        @Override
        public void addOrganism(Organism child, Organism parent) {
            int before = _organisms.size();
            super.addOrganism(child, parent);
            if (recordAdmissions) {
                if (!labels.containsKey(child)) labels.put(child, "admitted-" + admissionCount++);
                row("ADMIT", stage, label(child), "genome", label(child._geneticCode),
                    Integer.toString(before), Integer.toString(_organisms.size()));
            }
        }
    }

    private static final class TraceInCorridor extends InCorridor {
        private int draws;

        TraceInCorridor(World world) {
            super(world);
        }

        @Override
        public void draw(Graphics graphics) {
            draws++;
            super.draw(graphics);
        }
    }

    private static final class TraceOutCorridor extends OutCorridor {
        private int draws;

        TraceOutCorridor(World world) {
            super(world, null);
        }

        @Override
        public void draw(Graphics graphics) {
            draws++;
            super.draw(graphics);
        }
    }

    private static final class TraceList<T extends Corridor> extends ArrayList<T> {
        private static final long serialVersionUID = 1L;
        private final String kind;

        TraceList(String kind) {
            this.kind = kind;
        }

        @Override
        public boolean remove(Object target) {
            row("LIST", stage, kind, "remove-start", label(target), ids(this), rectOf(target));
            boolean removed = super.remove(target);
            row("LIST", stage, kind, "remove-end", Boolean.toString(removed), ids(this), rectOf(target));
            return removed;
        }
    }

    private static void row(String kind, String at, String object, String field,
                            String value, String extra1, String extra2) {
        String[] cells = {kind, caseName, at, object, field, value, extra1, extra2};
        StringBuilder line = new StringBuilder();
        for (int index = 0; index < cells.length; index++) {
            String cell = cells[index] == null ? "~" : cells[index];
            if (cell.indexOf('\t') >= 0 || cell.indexOf('\n') >= 0 || cell.indexOf('\r') >= 0
                || cell.chars().anyMatch(character -> character > 0x7f)) {
                throw new IllegalArgumentException("TSV cells cannot contain delimiters");
            }
            if (index != 0) line.append('\t');
            line.append(cell);
        }
        rows.add(line.toString());
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }

    private static Object field(Object owner, String name) throws ReflectiveOperationException {
        for (Class<?> type = owner.getClass(); type != null; type = type.getSuperclass()) {
            try {
                Field selected = type.getDeclaredField(name);
                selected.setAccessible(true);
                return selected.get(owner);
            } catch (NoSuchFieldException ignored) {
                continue;
            }
        }
        throw new NoSuchFieldException(owner.getClass().getName() + "." + name);
    }

    @SuppressWarnings("unchecked")
    private static Queue<GeneticCode> pending(InCorridor corridor) throws ReflectiveOperationException {
        return (Queue<GeneticCode>) field(corridor, "pendingOrganisms");
    }

    private static Organism candidate(InCorridor corridor) throws ReflectiveOperationException {
        return (Organism) field(corridor, "nextOrganism");
    }

    private static String label(Object value) {
        if (value == null) return "~";
        String found = labels.get(value);
        return found == null ? "unlabeled" : found;
    }

    private static String rect(Rectangle bounds) {
        return bounds.x + "," + bounds.y + "," + bounds.width + "," + bounds.height;
    }

    private static String rectOf(Object value) {
        return value instanceof Rectangle bounds ? rect(bounds) : "~";
    }

    private static String ids(List<? extends Corridor> corridors) {
        StringBuilder result = new StringBuilder();
        for (Corridor corridor : corridors) {
            if (result.length() != 0) result.append(',');
            result.append(label(corridor));
        }
        return result.length() == 0 ? "~" : result.toString();
    }

    private static String genomes(Queue<GeneticCode> queue) {
        StringBuilder result = new StringBuilder();
        for (GeneticCode code : queue) {
            if (result.length() != 0) result.append(',');
            result.append(label(code));
        }
        return result.length() == 0 ? "~" : result.toString();
    }

    private static String hex(double value) {
        return String.format(java.util.Locale.ROOT, "%016x", Double.doubleToRawLongBits(value));
    }

    private static TraceWorld freshWorld(String name) {
        caseName = name;
        stage = "setup";
        labels.clear();
        TraceVisibleWorld visible = new TraceVisibleWorld(window);
        TraceWorld world = new TraceWorld(visible);
        labels.put(world, "world");
        window._world = world;
        world.genesis();
        world.inCorridors = Collections.synchronizedList(new TraceList<>("in"));
        world.outCorridors = Collections.synchronizedList(new TraceList<>("out"));
        world._CO2 = Utils.INITIAL_CO2;
        world._O2 = Utils.INITIAL_O2;
        world._CH4 = Utils.INITIAL_CH4;
        world._population = 0;
        require(world._organisms.isEmpty(), "The fresh Source World must contain no organisms");
        return world;
    }

    private static GeneticCode genome(String name) {
        Gene gene = new Gene(12.0, 0.0, java.awt.Color.GREEN, -1,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        GeneticCode code = new GeneticCode(List.of(gene), 1, 0, 0, 0,
            0, 0, 0, 0, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false, false);
        row("GENOME", stage, name, "recipe", "green-length12-theta0-branch-(-1)",
            "symmetry=1;mirror=0;mutation=0;activity=0", "all flags=false; reactions=0");
        code._cladeID = name;
        labels.put(code, name);
        return code;
    }

    private static Organism traveller(World world, String name, GeneticCode code) {
        Organism organism = new Organism(world, code);
        labels.put(organism, name);
        return organism;
    }

    private static InCorridor input(World world, String name, int x, int y, int width, int height) {
        InCorridor corridor = new InCorridor(world);
        corridor.setBounds(x, y, width, height);
        labels.put(corridor, name);
        return corridor;
    }

    private static OutCorridor output(World world, String name, int x, int y, int width, int height,
                                      Organism traveller) {
        OutCorridor corridor = new OutCorridor(world, null);
        corridor.setBounds(x, y, width, height);
        corridor.travellingOrganism = traveller;
        labels.put(corridor, name);
        return corridor;
    }

    private static void observe(TraceWorld world, String when, Corridor... corridors) throws Exception {
        stage = when;
        row("STATE", when, "world", "in-list", ids(world.inCorridors),
            ids(world.outCorridors), Boolean.toString(world._corridorexists));
        row("STATE", when, "world", "population", Integer.toString(world._population),
            hex(world._CO2), hex(world._O2));
        row("STATE", when, "world", "methane", hex(world._CH4),
            Integer.toString(world._organisms.size()), "");
        for (Corridor corridor : corridors) {
            String queue = "~";
            String next = "~";
            String travelling = "~";
            if (corridor instanceof InCorridor incoming) {
                Queue<GeneticCode> values = pending(incoming);
                queue = genomes(values) + "#" + values.size();
                next = label(candidate(incoming));
            }
            if (corridor instanceof OutCorridor outgoing) {
                travelling = label(outgoing.getTravellingOrganism());
            }
            row("OBJECT", when, label(corridor), "rect", rect(corridor), queue, next + "," + travelling);
        }
    }

    private static void remove(TraceWorld world, TraceVisibleWorld visible, String label,
                               InCorridor incoming, OutCorridor outgoing) throws Exception {
        stage = label;
        repaintOrder = 0;
        visible.captureRepaints = true;
        world.removeCorridors(incoming, outgoing);
        visible.captureRepaints = false;
        observe(world, label + "-after", incoming, outgoing);
    }

    private static void firstMatchAndRepeatedRemoval() throws Exception {
        TraceWorld world = freshWorld("equal-bounds-first-match-and-repeat");
        TraceVisibleWorld visible = (TraceVisibleWorld) world._visibleWorld;
        GeneticCode codeA = genome("code-a");
        GeneticCode codeB = genome("code-b");
        GeneticCode codeB2 = genome("code-b2");
        InCorridor inA = input(world, "in-a", 40, 50, 20, 30);
        InCorridor inB = input(world, "in-b", 40, 50, 20, 30);
        inA.receiveOrganism(codeA);
        inB.receiveOrganism(codeB);
        inB.receiveOrganism(codeB2);
        OutCorridor outA = output(world, "out-a", 100, 110, 20, 30,
            traveller(world, "traveller-a", codeA));
        OutCorridor outB = output(world, "out-b", 100, 110, 20, 30,
            traveller(world, "traveller-b", codeB));
        world.addCorridors(inA, outA);
        world.addCorridors(inB, outB);
        observe(world, "before", inA, inB, outA, outB);
        Utils.random = new Random(7712301L);
        long expectedProbe = new Random(7712301L).nextLong();
        remove(world, visible, "remove-b", inB, outB);
        long actualProbe = Utils.random.nextLong();
        row("RNG", "remove-b", "probe", "random", "next-long", Long.toHexString(expectedProbe),
            Long.toHexString(actualProbe));
        remove(world, visible, "repeat-b", inB, outB);
        remove(world, visible, "repeat-absent-b", inB, outB);
        caseCount++;
    }

    private static void duplicateAliases() throws Exception {
        TraceWorld world = freshWorld("duplicate-aliases-and-repeated-removal");
        TraceVisibleWorld visible = (TraceVisibleWorld) world._visibleWorld;
        GeneticCode code = genome("alias-code");
        InCorridor incoming = input(world, "in-alias", 70, 80, 11, 12);
        incoming.receiveOrganism(code);
        OutCorridor outgoing = output(world, "out-alias", 170, 180, 13, 14,
            traveller(world, "alias-traveller", code));
        world.addCorridors(incoming, outgoing);
        world.addCorridors(incoming, outgoing);
        observe(world, "before", incoming, outgoing);
        remove(world, visible, "alias-remove-first", incoming, outgoing);
        remove(world, visible, "alias-remove-second", incoming, outgoing);
        remove(world, visible, "alias-remove-absent", incoming, outgoing);
        caseCount++;
    }

    private static void absentMembers() throws Exception {
        TraceWorld world = freshWorld("absent-members-do-not-set-sticky-flag");
        TraceVisibleWorld visible = (TraceVisibleWorld) world._visibleWorld;
        InCorridor incoming = input(world, "in-absent", 5, 6, 7, 8);
        OutCorridor outgoing = output(world, "out-absent", 15, 16, 17, 18, null);
        observe(world, "before", incoming, outgoing);
        remove(world, visible, "remove-absent", incoming, outgoing);
        caseCount++;
    }

    private static void integerWrap() throws Exception {
        TraceWorld world = freshWorld("i32-width-height-increment-wrap");
        TraceVisibleWorld visible = (TraceVisibleWorld) world._visibleWorld;
        InCorridor incoming = input(world, "in-wrap", 1, 2, Integer.MAX_VALUE, Integer.MAX_VALUE);
        OutCorridor outgoing = output(world, "out-wrap", 3, 4, Integer.MAX_VALUE, Integer.MAX_VALUE, null);
        observe(world, "before", incoming, outgoing);
        remove(world, visible, "remove-wrap", incoming, outgoing);
        caseCount++;
    }

    private static void frameAdmissionAndRetainedDrawing() throws Exception {
        TraceWorld world = freshWorld("frame-first-equal-removed-retained-alias-active");
        TraceVisibleWorld visible = (TraceVisibleWorld) world._visibleWorld;
        GeneticCode removedCode = genome("removed-code");
        GeneticCode retainedCode = genome("retained-code");
        InCorridor removed = input(world, "in-removed", 10, 10, 40, 40);
        TraceInCorridor retained = new TraceInCorridor(world);
        retained.setBounds(10, 10, 40, 40);
        labels.put(retained, "in-retained");
        removed.receiveOrganism(removedCode);
        retained.receiveOrganism(retainedCode);
        OutCorridor removedOut = output(world, "out-removed", 100, 100, 40, 40, null);
        TraceOutCorridor retainedOut = new TraceOutCorridor(world);
        retainedOut.setBounds(100, 100, 40, 40);
        labels.put(retainedOut, "out-retained");
        world.addCorridors(removed, removedOut);
        world.addCorridors(retained, retainedOut);
        world.addCorridors(retained, retainedOut);
        observe(world, "before-remove", removed, retained, removedOut, retainedOut);
        remove(world, visible, "remove-equal-first", retained, retainedOut);
        observe(world, "before-frame", removed, retained, removedOut, retainedOut);
        Utils.random = new Random(827364L);
        stage = "world-time";
        admissionCount = 0;
        recordAdmissions = true;
        world.time();
        recordAdmissions = false;
        observe(world, "after-frame", removed, retained, removedOut, retainedOut);
        Organism probe = new Organism(world, retainedCode);
        probe._centerX = 101;
        probe._centerY = 101;
        OutCorridor hit = world.checkHitCorridor(probe);
        row("HIT", "after-frame", "checkHitCorridor", label(hit), "center", "101,101",
            Integer.toString(world.outCorridors.size()));
        BufferedImage image = new BufferedImage(256, 256, BufferedImage.TYPE_INT_ARGB);
        Graphics graphics = image.getGraphics();
        world.draw(graphics);
        graphics.dispose();
        row("DRAW", "after-frame", "world", "corridor-calls",
            retained.draws + "," + retainedOut.draws, "edge-rgb", String.format(java.util.Locale.ROOT, "%08x", image.getRGB(100, 100)));
        row("DRAW", "after-frame", "world", "fill-rgb",
            String.format(java.util.Locale.ROOT, "%08x", image.getRGB(102, 102)),
            "in-alias-count", Integer.toString(world.inCorridors.size()));
        caseCount++;
    }

    private static String outputText() {
        StringBuilder result = new StringBuilder(HEADER).append('\n');
        for (String line : rows) result.append(line).append('\n');
        return result.toString();
    }

    private static void configurePreferences() throws Exception {
        Preferences preferences = Preferences.userNodeForPackage(Utils.class);
        preferences.putInt("VERSION", 700);
        preferences.putInt("INITIAL_ORGANISMS", 0);
        preferences.putInt("WORLD_WIDTH", 7000);
        preferences.putInt("WORLD_HEIGHT", 7000);
        preferences.putDouble("INITIAL_CO2", 500000d);
        preferences.putBoolean("ACCEPT_CONNECTIONS", false);
        preferences.putBoolean("CONNECT_TO_SERVER", false);
        preferences.putBoolean("AUTO_BACKUP", false);
        preferences.putInt("HARDWARE_ACCELERATION", 0);
        preferences.putInt("DELAY", 20);
        preferences.put("LOCALE", "en");
        preferences.flush();
        Utils.INITIAL_ORGANISMS = 0;
        Utils.ACCEPT_CONNECTIONS = false;
        Utils.CONNECT_TO_SERVER = false;
        Utils.AUTO_BACKUP = false;
    }

    private static void capture() throws Exception {
        Utils.INITIAL_ORGANISMS = 0;
        Utils.WORLD_WIDTH = 7000;
        Utils.WORLD_HEIGHT = 7000;
        Utils.AUTO_BACKUP = false;
        Utils.ACCEPT_CONNECTIONS = false;
        Utils.CONNECT_TO_SERVER = false;
        firstMatchAndRepeatedRemoval();
        duplicateAliases();
        absentMembers();
        integerWrap();
        frameAdmissionAndRetainedDrawing();
        caseName = "all";
        stage = "terminal";
        row("SUMMARY", "terminal", "oracle", "cases", Integer.toString(caseCount),
            "records", Integer.toString(rows.size()));
    }

    public static void main(String[] args) throws Exception {
        require(args.length == 1, "Usage: biogenesis.CorridorRemovalOracle NEW_OUTPUT_DIRECTORY");
        Path directory = Path.of(args[0]).toAbsolutePath().normalize();
        Path parent = directory.getParent().toRealPath();
        for (String property : List.of("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot")) {
            String value = System.getProperty(property);
            require(value != null && Path.of(value).toRealPath().startsWith(parent)
                && !Path.of(value).toRealPath().equals(parent), "Explicit isolated profile required: " + property);
        }
        require(!Files.exists(directory), "Output directory must not exist");
        require("1".equals(System.getenv("BGR_ISOLATED_XVFB")) && System.getenv("DISPLAY") != null
            && System.getenv("DISPLAY").matches(":[1-9][0-9]{2,}"), "Owned isolated Xvfb required");
        Files.createDirectory(directory);
        Throwable failure = null;
        int status = 0;
        PrintStream originalOut = System.out;
        String output = null;
        try {
            System.setOut(new PrintStream(OutputStream.nullOutputStream(), true, StandardCharsets.US_ASCII));
            configurePreferences();
            SwingUtilities.invokeAndWait(() -> MainWindow.main(new String[] {"827364"}));
            for (Frame frame : Frame.getFrames()) if (frame instanceof MainWindow main) window = main;
            require(window != null && !window.isProcessActive() && window.serverThread == null
                && window.getWorld() != null && window.getWorld()._organisms.isEmpty(),
                "Actual paused MainWindow without a network server required");
            if (window._timer != null) window._timer.cancel();
            FutureTask<String> capture = new FutureTask<>(() -> {
                capture();
                return outputText();
            });
            SwingUtilities.invokeAndWait(capture);
            output = capture.get();
            byte[] bytes = output.getBytes(StandardCharsets.US_ASCII);
            Files.write(directory.resolve("corridor-removal.tsv"), bytes, StandardOpenOption.CREATE_NEW);
        } catch (Throwable error) {
            failure = error;
            status = 1;
        } finally {
            try {
                SwingUtilities.invokeAndWait(() -> {
                    for (Frame frame : Frame.getFrames()) if (frame instanceof MainWindow main && main._timer != null) main._timer.cancel();
                    for (Window target : Window.getWindows()) target.dispose();
                });
            } catch (Throwable cleanup) {
                status = 1;
                if (failure == null) failure = cleanup;
                else failure.addSuppressed(cleanup);
            } finally {
                if (failure != null) failure.printStackTrace();
                System.setOut(originalOut);
                if (status == 0 && output != null) {
                    try {
                        originalOut.write(output.getBytes(StandardCharsets.US_ASCII));
                        originalOut.flush();
                    } catch (Exception emitFailure) {
                        emitFailure.printStackTrace();
                        status = 1;
                    }
                }
                System.exit(status);
            }
        }
    }
}
