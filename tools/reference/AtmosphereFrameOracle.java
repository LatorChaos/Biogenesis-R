// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.io.BufferedWriter;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;
import java.util.Random;

/** Observe real empty World.time gas updates, clocks, and partial UI-hook failure state */
public final class AtmosphereFrameOracle {
    private static BufferedWriter output;
    private static final long SEED = 827364;
    private record Spec(String name, double carbon, double methane, int carbonDivisor, int methaneDivisor, int frame, boolean diagnostic) {}
    private static void require(boolean value, String message) {
        if (!value) throw new IllegalStateException(message);
    }
    private static String bits(double value) {
        return String.format(Locale.ROOT, "%016x", Double.doubleToRawLongBits(value));
    }
    private static void row(Object... values) throws Exception {
        for (int i = 0; i < values.length; i++) {
            if (i != 0) output.write('\t');
            output.write(values[i].toString());
        }
        output.write('\n');
    }
    private static byte[] randomBytes() throws Exception {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (ObjectOutputStream stream = new ObjectOutputStream(bytes)) { stream.writeObject(Utils.random); }
        return bytes.toByteArray();
    }
    private static void random(String stage) throws Exception {
        Random copy;
        try (ObjectInputStream stream = new ObjectInputStream(new ByteArrayInputStream(randomBytes()))) {
            copy = (Random) stream.readObject();
        }
        row("R", stage, copy.nextLong(), copy.nextInt(10000), bits(copy.nextDouble()));
    }
    private static String scalar(Object value) {
        if (value == null) return "~";
        if (value instanceof Double number) return bits(number);
        if (value instanceof Integer || value instanceof Long || value instanceof Boolean) return value.toString();
        if (value instanceof List<?> values) {
            ArrayList<String> encoded = new ArrayList<>();
            for (Object element : values) {
                require(element instanceof Double, "Unexpected nonnumeric statistics history");
                encoded.add(bits((Double) element));
            }
            return String.join(",", encoded);
        }
        throw new IllegalStateException("Unsupported observation type " + value.getClass().getName());
    }
    private static List<Field> fields(Class<?> type) {
        return Arrays.stream(type.getDeclaredFields()).filter(f -> !Modifier.isStatic(f.getModifiers()))
            .sorted(Comparator.comparing(Field::getName)).toList();
    }
    private static void snapshot(String stage, World world) throws Exception {
        List<Field> worldFields = fields(World.class);
        require(worldFields.size() == 15, "World schema changed");
        for (Field field : worldFields) {
            field.setAccessible(true);
            Object value = field.get(world);
            String encoded;
            if (value instanceof WorldStatistics) encoded = "s0";
            else if (field.getName().equals("colDetTree")) encoded = value == null ? "~" : "present";
            else if (value instanceof List<?> values) {
                require(values.isEmpty(), "Empty-world membership changed");
                encoded = "";
            } else encoded = scalar(value);
            row("W", stage, field.getName(), field.getType().getSimpleName(), encoded);
        }
        List<Field> stats = fields(WorldStatistics.class);
        require(stats.size() == 52, "Statistics schema changed");
        for (Field field : stats) {
            field.setAccessible(true);
            row("S", stage, field.getName(), field.getType().getSimpleName(), scalar(field.get(world.worldStatistics)));
        }
        List<Method> getters = Arrays.stream(WorldStatistics.class.getDeclaredMethods())
            .filter(m -> Modifier.isPublic(m.getModifiers()) && m.getParameterCount() == 0 && m.getName().startsWith("get"))
            .sorted(Comparator.comparing(Method::getName)).toList();
        require(getters.size() == 49, "Statistics getter schema changed");
        for (Method method : getters) row("T", stage, method.getName(), method.getReturnType().getSimpleName(), scalar(method.invoke(world.worldStatistics)));
        row("G", stage, "World.getTime", "long", world.getTime());
        random(stage);
    }
    private static void observe(Spec spec) throws Exception {
        for (Field field : Utils.class.getDeclaredFields()) if (field.getName().startsWith("DEF_"))
            Utils.class.getDeclaredField(field.getName().substring(4)).set(null, field.get(null));
        Utils.INITIAL_ORGANISMS = 0;
        Utils.ACCEPT_CONNECTIONS = false;
        Utils.CONNECT_TO_SERVER = false;
        Utils.AUTO_BACKUP = false;
        Utils.CO2_TO_CH4_DIVISOR = spec.carbonDivisor;
        Utils.CH4_TO_CO2_DIVISOR = spec.methaneDivisor;
        Utils.random = new Random(SEED);
        World world = new World(null);
        world._CO2 = spec.carbon;
        world._CH4 = spec.methane;
        world._O2 = -0.0;
        Field frame = World.class.getDeclaredField("nFrames");
        frame.setAccessible(true);
        frame.setInt(world, spec.frame);
        require(world._organisms.isEmpty() && world.inCorridors.isEmpty() && world.outCorridors.isEmpty(), "World is not empty");
        require(!world._corridorexists && world._visibleWorld == null, "Unexpected collaborator");
        byte[] random = randomBytes();
        Object tree = world.colDetTree;
        row("C", spec.name, spec.diagnostic ? "ui-hook-diagnostic" : "raw-empty-frame", SEED);
        for (String name : List.of("INITIAL_ORGANISMS", "INITIAL_O2", "INITIAL_CO2", "INITIAL_CH4", "WORLD_WIDTH", "WORLD_HEIGHT", "CO2_TO_CH4_DIVISOR", "CH4_TO_CO2_DIVISOR")) {
            Field setting = Utils.class.getDeclaredField(name);
            row("U", name, setting.getType().getSimpleName(), scalar(setting.get(null)));
        }
        row("I", "_CO2", "double", bits(spec.carbon));
        row("I", "_CH4", "double", bits(spec.methane));
        row("I", "_O2", "double", bits(-0.0));
        row("I", "nFrames", "int", spec.frame);
        snapshot("before", world);
        row("K", "time");
        String failure = null;
        try { world.time(); }
        catch (NullPointerException error) {
            if (!spec.diagnostic) throw error;
            failure = error.getClass().getName();
        }
        require((failure != null) == spec.diagnostic, "Unexpected World.time result");
        row("B", "completed", failure == null);
        if (failure != null) row("X", "diagnostic_exception", failure);
        require(world.colDetTree != tree, "Frame tree was not replaced");
        row("B", "tree_replaced", true);
        require(Arrays.equals(random, randomBytes()), "Empty World.time consumed RNG");
        row("B", "random_unchanged", true);
        snapshot("after", world);
        row("E");
    }
    private static List<Spec> cases() {
        ArrayList<Spec> values = new ArrayList<>();
        values.add(new Spec("defaults-positive", 10, 3, 999, 1000, 1, false));
        values.add(new Spec("defaults-zero", 0, 0, 999, 1000, 1, false));
        values.add(new Spec("defaults-negative-zero", -0.0, -0.0, 999, 1000, 1, false));
        values.add(new Spec("defaults-mixed-zero", -0.0, 0, 999, 1000, 1, false));
        values.add(new Spec("defaults-negative", -10, -3, 999, 1000, 1, false));
        values.add(new Spec("defaults-negative-carbon", -10, 3, 999, 1000, 1, false));
        values.add(new Spec("defaults-negative-methane", 10, -3, 999, 1000, 1, false));
        int[] divisors = {0, 1, -1, Integer.MIN_VALUE, Integer.MAX_VALUE};
        String[] names = {"zero", "one", "negative-one", "minimum", "maximum"};
        for (int i = 0; i < divisors.length; i++) for (boolean negative : List.of(false, true)) {
            double sign = negative ? -1 : 1;
            String suffix = negative ? "negative" : "positive";
            values.add(new Spec("carbon-" + names[i] + "-" + suffix, sign * 10, sign * 3, divisors[i], 1000, 1, false));
            values.add(new Spec("methane-" + names[i] + "-" + suffix, sign * 10, sign * 3, 999, divisors[i], 1, false));
        }
        values.add(new Spec("both-zero-positive", 10, 3, 0, 0, 1, false));
        values.add(new Spec("both-zero-empty", 0, 0, 0, 0, 1, false));
        values.add(new Spec("both-zero-negative", -10, -3, 0, 0, 1, false));
        values.add(new Spec("both-one-updated-methane", 10, 0, 1, 1, 1, false));
        values.add(new Spec("both-negative-one", 10, 3, -1, -1, 1, false));
        values.add(new Spec("both-extreme-divisors", -10, 3, Integer.MIN_VALUE, Integer.MAX_VALUE, 1, false));
        values.add(new Spec("maximum-finite-positive", Double.MAX_VALUE, Double.MAX_VALUE, -1, -1, 1, false));
        values.add(new Spec("maximum-finite-negative", -Double.MAX_VALUE, -Double.MAX_VALUE, -1, -1, 1, false));
        for (int f : new int[]{254, 255, -1, Integer.MAX_VALUE})
            values.add(new Spec("clock-" + f, 10, 3, 999, 1000, f, false));
        return values;
    }
    public static void main(String[] args) throws Exception {
        require(args.length == 2, "Expected new output directory and mode");
        require(List.of("probe", "canonical", "diagnostic").contains(args[1]), "Unknown mode");
        require("true".equals(System.getProperty("java.awt.headless")), "Headless mode required");
        Path destination = Path.of(args[0]).toAbsolutePath().normalize();
        Path root = destination.getParent().toRealPath();
        require(!Files.exists(destination), "Output already exists");
        for (String key : List.of("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot")) {
            String value = System.getProperty(key);
            require(value != null && Path.of(value).toRealPath().startsWith(root), "Unowned profile " + key);
        }
        Files.createDirectory(destination);
        try (BufferedWriter stream = Files.newBufferedWriter(destination.resolve("atmosphere_frame.tsv"), StandardCharsets.US_ASCII)) {
            output = stream;
            if (args[1].equals("probe")) observe(cases().get(0));
            else if (args[1].equals("canonical")) for (Spec spec : cases()) observe(spec);
            else {
                observe(new Spec("ui-hook-positive-partial", 10, 3, 999, 1000, 0, true));
                observe(new Spec("ui-hook-zero-divisors-partial", 0, 0, 0, 0, 0, true));
            }
        }
    }
}
