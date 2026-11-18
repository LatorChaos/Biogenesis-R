package biogenesis;

import java.awt.Color;
import java.awt.GraphicsEnvironment;
import java.awt.geom.Line2D;
import java.awt.geom.Point2D;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.ObjectInputFilter;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.io.ObjectStreamClass;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.HashSet;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Random;
import org.xml.sax.SAXException;

/** Observe real reference objects without a window, preference access, networking, or simulated world frames */
public final class ReferenceOracle {
    private static int checks;
    private static Path output;

    private static void require(boolean condition, String message) {
        checks++;
        if (!condition) throw new AssertionError(message);
    }

    private static Map<String, Object> values(Object... entries) {
        Map<String, Object> result = new LinkedHashMap<>();
        for (int i = 0; i < entries.length; i += 2) result.put((String)entries[i], entries[i + 1]);
        return result;
    }

    private static String quote(String value) {
        StringBuilder result = new StringBuilder("\"");
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            switch (c) {
                case '"' -> result.append("\\\"");
                case '\\' -> result.append("\\\\");
                case '\n' -> result.append("\\n");
                case '\r' -> result.append("\\r");
                case '\t' -> result.append("\\t");
                default -> {
                    if (c < 32 || c > 126) result.append(String.format(Locale.ROOT, "\\u%04x", (int)c));
                    else result.append(c);
                }
            }
        }
        return result.append('"').toString();
    }

    private static String json(Object value) {
        if (value == null) return "null";
        if (value instanceof String text) return quote(text);
        if (value instanceof Double number) return quote(Double.toHexString(number));
        if (value instanceof Float number) return quote(Float.toHexString(number));
        if (value instanceof Number || value instanceof Boolean) return value.toString();
        if (value instanceof Map<?, ?> map) {
            List<String> entries = new ArrayList<>();
            for (var entry : map.entrySet()) entries.add(quote((String)entry.getKey()) + ":" + json(entry.getValue()));
            return "{" + String.join(",", entries) + "}";
        }
        if (value instanceof Iterable<?> list) {
            List<String> entries = new ArrayList<>();
            for (Object entry : list) entries.add(json(entry));
            return "[" + String.join(",", entries) + "]";
        }
        throw new IllegalArgumentException("Unsupported JSON value: " + value.getClass().getName());
    }

    private static void emit(String scenario, Map<String, Object> observations) {
        System.out.println(json(values("scenario", scenario, "observations", observations)));
    }

    private static String sha256(byte[] bytes) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    }

    private static String fingerprint(Object value) throws Exception {
        return sha256(json(value).getBytes(StandardCharsets.UTF_8));
    }

    private static Field[] fields(Class<?> type) {
        Field[] result = type.getDeclaredFields();
        Arrays.sort(result, Comparator.comparing(Field::getName));
        return result;
    }

    private static Object state(Object value) throws Exception {
        if (value == null || value instanceof String || value instanceof Number || value instanceof Boolean) return value;
        if (value instanceof Color color) return List.of(color.getRed(), color.getGreen(), color.getBlue(), color.getAlpha());
        if (value instanceof World world) return worldState(world);
        if (value instanceof Iterable<?> list) {
            List<Object> result = new ArrayList<>();
            for (Object item : list) result.add(state(item));
            return result;
        }
        if (value.getClass().isArray()) {
            List<Object> result = new ArrayList<>();
            for (int i = 0; i < java.lang.reflect.Array.getLength(value); i++) {
                result.add(state(java.lang.reflect.Array.get(value, i)));
            }
            return result;
        }
        if (!(value instanceof Gene || value instanceof GeneticCode || value instanceof WorldStatistics)) {
            throw new IllegalArgumentException("Unsupported snapshot: " + value.getClass().getName());
        }
        Map<String, Object> result = new LinkedHashMap<>();
        for (Field field : fields(value.getClass())) {
            if (Modifier.isStatic(field.getModifiers())) continue;
            field.setAccessible(true);
            result.put(field.getName(), state(field.get(value)));
        }
        return result;
    }

    private static Map<String, Object> worldState(World world) throws Exception {
        return values("width", world.getWidth(), "height", world.getHeight(), "gases", gases(world),
                "population", world.getPopulation(), "corpses", world.getNCorpses(), "frame", world.getFrame(),
                "next_id", world.NEXT_ID, "corridor_flag", world._corridorexists,
                "incoming_count", world.inCorridors.size(), "outgoing_count", world.outCorridors.size(),
                "statistics", state(world.worldStatistics));
    }

    private static List<Double> gases(World world) {
        return List.of(world.getO2(), world.getCO2(), world.getCH4());
    }

    private static double totalGas(World world) {
        return world.getO2() + world.getCO2() + world.getCH4();
    }

    private static void close(double actual, double expected, String message) {
        require(Math.abs(actual - expected) <= 1e-10 * Math.max(1, Math.abs(expected)), message);
    }

    private static Map<String, Object> configuration() throws Exception {
        Map<String, Object> result = new LinkedHashMap<>();
        for (Field field : fields(Utils.class)) {
            if (!Modifier.isStatic(field.getModifiers()) || !field.getName().startsWith("DEF_")) continue;
            field.setAccessible(true);
            Field effective = Utils.class.getDeclaredField(field.getName().substring(4));
            effective.setAccessible(true);
            result.put(field.getName(), values("default", state(field.get(null)), "effective", state(effective.get(null))));
        }
        return result;
    }

    private static void randomObservations() throws Exception {
        for (long seed : new long[] {0, 1, 42, -1, Long.MIN_VALUE, Long.MAX_VALUE}) {
            Random first = new Random(seed);
            Random second = new Random(seed);
            List<Object> draws = new ArrayList<>();
            for (int i = 0; i < 8; i++) {
                int integer = first.nextInt();
                int bounded = first.nextInt(10000);
                double unit = first.nextDouble();
                boolean bit = first.nextBoolean();
                long wide = first.nextLong();
                require(integer == second.nextInt() && bounded == second.nextInt(10000)
                        && unit == second.nextDouble() && bit == second.nextBoolean()
                        && wide == second.nextLong(), "Java Random same-seed determinism");
                require(bounded >= 0 && bounded < 10000 && unit >= 0 && unit < 1, "Java Random output domains");
                draws.add(values("int", integer, "bounded_10000", bounded, "double", unit, "boolean", bit, "long", wide));
            }
            emit("java_random", values("seed", seed, "draw_order", List.of("nextInt", "nextInt(10000)",
                    "nextDouble", "nextBoolean", "nextLong"), "draws", draws, "fingerprint", fingerprint(draws)));
        }
    }

    private static Gene geneFixture() throws Exception {
        Gene gene = new Gene();
        gene.setLength(7.25);
        gene.setTheta(-0.375);
        gene.setBranch(-1);
        gene.setColor(Color.GREEN);
        int index = 0;
        for (Field field : fields(Gene.class)) {
            if (field.getName().endsWith("reaction")) {
                field.setAccessible(true);
                field.setInt(gene, index++ % 6);
            }
        }
        require(index == 36, "All 36 reaction channels are represented");
        return gene;
    }

    private static void geneObservations() throws Exception {
        Gene original = geneFixture();
        Object before = state(original);
        Gene clone = (Gene)original.clone();
        require(clone != original && state(clone).equals(before), "Gene clone preserves every instance field");
        clone.setLength(13.5);
        for (Field field : fields(Gene.class)) {
            if (field.getName().endsWith("reaction")) {
                field.setAccessible(true);
                field.setInt(clone, (field.getInt(clone) + 1) % 6);
            }
        }
        require(state(original).equals(before), "Gene clone mutations leave original unchanged");
        emit("gene_clone", values("input", before, "changed_clone", state(clone), "original_after", state(original)));
        for (long seed : new long[] {0, 42}) {
            Utils.random = new Random(seed);
            Gene randomized = new Gene();
            randomized.randomize();
            require(randomized.getLength() >= 2 && randomized.getLength() < 18, "Random gene length domain");
            require(randomized.getTheta() >= 0 && randomized.getTheta() < 2 * Math.PI, "Random gene theta domain");
            emit("gene_random", values("seed", seed, "state", state(randomized), "next_rng_long", Utils.random.nextLong()));
        }
    }

    private static List<Object> inheritance(long seed, int minimum, int maximum, int meta) throws Exception {
        Utils.MIN_MUTATION_RATE = minimum;
        Utils.MAX_MUTATION_RATE = maximum;
        Utils.META_MUTATION_RATE = meta;
        Utils.random = new Random(seed);
        GeneticCode code = new GeneticCode();
        code._generation = 1;
        code._cladeID = "oracle-root";
        List<Object> result = new ArrayList<>();
        result.add(state(code));
        for (int i = 0; i < 4; i++) {
            int previousGeneration = code.getGeneration();
            code = new GeneticCode(code);
            require(code.getGeneration() == previousGeneration + 1, "Inheritance increments generation once");
            require(code.getNGenes() >= 1 && code.getNGenes() <= 100, "Inherited gene count domain");
            require(code.getSymmetry() >= 1 && code.getSymmetry() <= 8, "Inherited symmetry domain");
            require(code.getMutationrate() >= minimum && code.getMutationrate() <= maximum, "Inherited mutation domain");
            result.add(state(code));
        }
        result.add(values("next_rng_long", Utils.random.nextLong()));
        return result;
    }

    private static GeneticCode genomeObservations() throws Exception {
        int minimum = Utils.MIN_MUTATION_RATE;
        int maximum = Utils.MAX_MUTATION_RATE;
        int meta = Utils.META_MUTATION_RATE;
        try {
            for (int[] rates : new int[][] {{minimum, maximum, meta}, {0, 0, 0}, {10000, 10000, 10000}}) {
                for (long seed : new long[] {0, 42}) {
                    List<Object> first = inheritance(seed, rates[0], rates[1], rates[2]);
                    List<Object> second = inheritance(seed, rates[0], rates[1], rates[2]);
                    require(first.equals(second), "Full inheritance snapshot and random stream are deterministic");
                    emit("genome_inheritance", values("seed", seed, "min_rate", rates[0], "max_rate", rates[1],
                            "meta_rate", rates[2], "states", first, "fingerprint", fingerprint(first)));
                }
            }
        } finally {
            Utils.MIN_MUTATION_RATE = minimum;
            Utils.MAX_MUTATION_RATE = maximum;
            Utils.META_MUTATION_RATE = meta;
        }
        Utils.random = new Random(2026);
        GeneticCode original = new GeneticCode();
        original._generation = 7;
        original._cladeID = "oracle-root+7";
        Object before = state(original);
        GeneticCode clone = (GeneticCode)original.clone();
        require(clone != original && state(clone).equals(before), "Genome clone preserves full schema");
        for (int i = 0; i < original.getNGenes(); i++) {
            require(clone.getGene(i) != original.getGene(i), "Genome clone deep-copies every gene");
        }
        clone.getGene(0).setLength(3.125);
        require(state(original).equals(before), "Deep clone editing leaves source genome unchanged");
        emit("genome_clone", values("seed", 2026, "original", before, "changed_clone", state(clone)));
        return original;
    }

    private static void colorObservations() throws Exception {
        ColorComboBox selector = new ColorComboBox(Color.GREEN);
        require(selector.getItemCount() == 46, "Selector exposes 46 base colors");
        Method classify = Organism.class.getDeclaredMethod("getTypeColor", Color.class);
        classify.setAccessible(true);
        Field[] constants = Arrays.stream(fields(Organism.class))
                .filter(field -> field.getType() == int.class && Modifier.isStatic(field.getModifiers()))
                .toArray(Field[]::new);
        Map<Integer, String> names = new java.util.TreeMap<>();
        for (Field field : constants) {
            field.setAccessible(true);
            names.put(field.getInt(null), field.getName());
        }
        require(names.size() == 65 && names.containsKey(-1), "64 classified colors plus unknown sentinel");
        List<Object> colors = new ArrayList<>();
        HashSet<Integer> distinct = new HashSet<>();
        for (int index = 0; index < 64; index++) {
            Color color;
            if (index < selector.getItemCount()) {
                selector.setSelectedIndex(index);
                color = selector.getSelectedColor();
                selector.setSelectedColor(color);
                require(selector.getSelectedIndex() == index, "Color selector reverse mapping");
            } else {
                Field field = Utils.class.getDeclaredField("Color" + names.get(index));
                field.setAccessible(true);
                color = (Color)field.get(null);
            }
            require(((Integer)classify.invoke(null, color)) == index, "Selector/state RGB agrees with organism classification");
            require(distinct.add(color.getRGB()), "Classified RGB identities remain distinct");
            colors.add(values("index", index, "name", names.get(index), "rgba", state(color), "selectable", index < 46));
        }
        Color unknown = new Color(1, 2, 3);
        int fallback = (Integer)classify.invoke(null, unknown);
        require(fallback == -1, "Unknown RGB retains unknown classifier sentinel");
        emit("color_identities", values("colors", colors, "unknown_rgba", state(unknown), "unknown_type", fallback));
    }

    private static void geometryObservations() {
        Vector2D vector = new Vector2D();
        require(vector.getX() == 0 && vector.getY() == 0, "Blank vector is zero");
        List<Object> vectors = new ArrayList<>();
        for (double theta : new double[] {0, Math.PI / 2, -0.75, 7 * Math.PI}) {
            vector.setPolar(3.5, theta);
            double x = vector.getX(), y = vector.getY();
            close(x * x + y * y, 12.25, "Polar vector preserves squared length");
            vector.invertX();
            close(vector.getX(), -x, "X inversion reflects horizontal component");
            close(vector.getY(), y, "X inversion preserves vertical component");
            vector.invertY();
            close(vector.getX(), -x, "Y inversion preserves current horizontal component");
            close(vector.getY(), -y, "Y inversion reflects current vertical component");
            vectors.add(values("input_modulus", 3.5, "input_theta", theta, "initial_xy", List.of(x, y),
                    "after_both_inversions", List.of(vector.getX(), vector.getY()), "stored_theta", vector.getTheta()));
        }
        emit("vector_geometry", values("observations", vectors));
        double[][] cases = {{0, 0, 4, 0, 3, -2, 3, 2}, {0, 0, 4, 0, 3, 2, 3, -2},
                {0, 0, 4, 0, 0, 2, 4, 2}, {2, 2, 2, 2, 0, 0, 4, 4}};
        List<Object> intersections = new ArrayList<>();
        for (double[] points : cases) {
            ExLine2DDouble first = new ExLine2DDouble();
            first.setLine(points[0], points[1], points[2], points[3]);
            Line2D.Double second = new Line2D.Double(points[4], points[5], points[6], points[7]);
            Point2D.Double intersection = first.getIntersection(second);
            require(Double.isFinite(intersection.x) && Double.isFinite(intersection.y), "Bounded geometry fixture remains finite");
            intersections.add(values("input", Arrays.stream(points).boxed().toList(), "output", List.of(intersection.x, intersection.y)));
        }
        emit("line_intersection_quirks", values("cases", intersections,
                "note", "Observe signed determinant fallback and endpoint-order sensitivity; no corrected intersection is substituted"));
    }

    private static World gasObservations() throws Exception {
        World world = null;
        List<Object> transitions = new ArrayList<>();
        for (String operation : List.of("respiration", "decomposition", "photosynthesis", "methanotrophy")) {
            world = new World(null);
            world.addO2(12);
            world.addCO2(20);
            world.addCH4(8);
            for (double request : new double[] {0, 0.5, 100}) {
                List<Double> before = gases(world);
                double total = totalGas(world);
                double amount = switch (operation) {
                    case "respiration" -> world.respiration(request);
                    case "decomposition" -> world.decomposition(request);
                    case "photosynthesis" -> world.photosynthesis(request);
                    case "methanotrophy" -> world.methanotrophy(request);
                    default -> throw new AssertionError(operation);
                };
                require(amount >= 0 && amount <= request, "Gas conversion remains bounded by nonnegative request");
                require(gases(world).stream().allMatch(value -> value >= 0), "Nonnegative gas fixture remains nonnegative");
                close(totalGas(world), total, "Gas conversion conserves total atmosphere");
                transitions.add(values("operation", operation, "request", request, "before", before, "returned", amount, "after", gases(world)));
            }
        }
        for (String operation : List.of("decreaseO2", "decreaseCO2", "decreaseCH4")) {
            for (double request : new double[] {0, 0.5, 100}) {
                List<Double> before = gases(world);
                double total = totalGas(world);
                switch (operation) {
                    case "decreaseO2" -> world.decreaseO2(request);
                    case "decreaseCO2" -> world.decreaseCO2(request);
                    case "decreaseCH4" -> world.decreaseCH4(request);
                    default -> throw new AssertionError(operation);
                }
                require(gases(world).stream().allMatch(value -> value >= 0), "Gas removal preserves nonnegative quantities");
                require(totalGas(world) <= total && totalGas(world) >= total - request, "Gas removal is bounded by request");
                transitions.add(values("operation", operation, "request", request, "before", before, "after", gases(world)));
            }
        }
        emit("world_gas_methods", values("initial_o2_co2_ch4", List.of(12, 20, 8), "transitions", transitions,
                "collaborator", "Real World(null); no frame, visible-world, organism, or corridor simulation"));
        World invalid = new World(null);
        invalid.addO2(1);
        invalid.addCO2(1);
        invalid.addCH4(1);
        List<Double> before = gases(invalid);
        double amount = invalid.respiration(-2);
        emit("negative_gas_request_quirk", values("operation", "respiration", "request", -2,
                "before", before, "returned", amount, "after", gases(invalid), "note", "Reference input validation is observed, not repaired"));
        return world;
    }

    private static WorldStatistics statisticsObservations(GeneticCode code) throws Exception {
        Utils.random = new Random(7000);
        World world = new World(null);
        Organism parent = new Organism(world, code);
        Organism child = new Organism(world, (GeneticCode)code.clone());
        WorldStatistics stats = new WorldStatistics();
        require(stats.getAveragePopulation() == 0 && stats.getAverageBirths() == 0
                && stats.getAverageDeaths() == 0 && stats.getAverageInfections() == 0, "Zero-time averages are defined");
        List<Object> trace = new ArrayList<>();
        stats.eventOrganismCreated();
        stats.eventPopulationIncrease(4);
        stats.eventPopulationDecrease(2);
        parent._nTotalChildren = 3;
        stats.eventOrganismBorn(child, parent);
        parent._nTotalKills = 2;
        stats.eventOrganismDie(child, parent);
        parent._nTotalInfected = 4;
        stats.eventOrganismInfects(child, parent);
        require(stats.getBeingMostChildrenTime() == 0 && stats.getBeingMostKillsTime() == 0
                && stats.getBeingMostInfectionsTime() == 0, "Events preceding first tick retain zero timestamps");
        trace.add(values("operation", "events_before_first_tick", "state", state(stats)));
        stats.eventTime(7, 10, 20, 30);
        require(stats.getTime() == 1 && stats.getAverageBirths() == 1 && stats.getAverageDeaths() == 1
                && stats.getAverageInfections() == 1, "First tick exposes event totals without clearing cumulative counts");
        trace.add(values("operation", "first_tick", "state", state(stats)));
        parent._nTotalChildren = 4;
        stats.eventOrganismBorn(child, parent);
        require(stats.getBeingMostChildrenTime() == 1, "Later record uses current time before next tick");
        for (int i = 2; i <= 101; i++) stats.eventTime(i, 10, 20, 30);
        require(stats.getPopulationList().size() == 100 && stats.getPopulationList().get(0) == 2
                && stats.getPopulationList().get(99) == 101, "History retains exactly the newest 100 samples");
        child._nTotalChildren = parent._nTotalChildren;
        stats.findBestAliveBeings(List.of(parent, child));
        require(stats.getAliveBeingMostChildren() == parent.getGeneticCode(), "Alive leader ties retain first organism");
        parent.alive = false;
        stats.findBestAliveBeings(List.of(parent, child));
        require(stats.getAliveBeingMostChildren() == child.getGeneticCode(), "Dead organisms are excluded from alive leaders");
        stats.findBestAliveBeings(List.of());
        require(stats.getAliveBeingMostChildren() == null && stats.getAliveBeingMostKills() == null
                && stats.getAliveBeingMostInfections() == null, "Empty alive leader scan clears all leaders");
        trace.add(values("operation", "after_101_ticks_and_leader_scans", "state", state(stats)));
        emit("statistics_events", values("organism_constructor_seed", 7000,
                "fixture_scope", "Real statistics methods and real organism field inputs; no world-frame behavior is claimed",
                "inputs", List.of("created", "population 4 then 2", "parent children=3, kills=2, infections=4",
                        "birth, death, infection", "time(population7,gases10/20/30)", "children=4, birth",
                        "100 time events with populations2..101", "alive leader tie, death-filter, empty scans"), "trace", trace));
        return stats;
    }

    private static Object serializationFixture(String name, Object original) throws Exception {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (ObjectOutputStream stream = new ObjectOutputStream(bytes)) {
            stream.writeObject(original);
        }
        byte[] data = bytes.toByteArray();
        Files.write(output.resolve(name + ".ser"), data, StandardOpenOption.CREATE_NEW);
        Object restored;
        try (ObjectInputStream stream = new ObjectInputStream(new ByteArrayInputStream(data))) {
            stream.setObjectInputFilter(info -> info.depth() > 64 || info.references() > 10000
                    || info.arrayLength() > 10000 || info.streamBytes() > 1000000
                    ? ObjectInputFilter.Status.REJECTED : ObjectInputFilter.Status.UNDECIDED);
            restored = stream.readObject();
            require(stream.read() == -1, "Generated serialization has no trailing primitive block data");
        }
        require(state(original).equals(state(restored)), "Fresh Java serialization preserves observed complete state");
        emit("java_serialization", values("fixture", name + ".ser", "class", original.getClass().getName(),
                "serial_uid", ObjectStreamClass.lookup(original.getClass()).getSerialVersionUID(), "bytes", data.length,
                "sha256", sha256(data), "before", state(original), "after", state(restored),
                "input_scope", "Only bytes created in this process are deserialized"));
        return restored;
    }

    private static void xmlObservations(GeneticCode code) throws Exception {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (PrintStream stream = new PrintStream(bytes, false, StandardCharsets.UTF_8)) {
            BioXMLParser.writeGeneticCode(stream, code);
            require(!stream.checkError(), "XML writer reports no stream error");
        }
        Path path = output.resolve("genome.bgg");
        Files.write(path, bytes.toByteArray(), StandardOpenOption.CREATE_NEW);
        BioXMLParser parser = new BioXMLParser();
        require(parser.builder != null, "Real reference XML parser initialized");
        parser.builder.setEntityResolver((publicId, systemId) -> {
            throw new SAXException("External XML resources are prohibited in the oracle");
        });
        GeneticCode restored = parser.parseGeneticCode(path.toFile());
        ByteArrayOutputStream restoredBytes = new ByteArrayOutputStream();
        try (PrintStream stream = new PrintStream(restoredBytes, false, StandardCharsets.UTF_8)) {
            BioXMLParser.writeGeneticCode(stream, restored);
            require(!stream.checkError(), "Restored XML writer reports no stream error");
        }
        require(Arrays.equals(bytes.toByteArray(), restoredBytes.toByteArray()), "XML round trip preserves exported schema exactly");
        emit("xml_round_trip", values("fixture", "genome.bgg", "bytes", bytes.size(), "sha256", sha256(bytes.toByteArray()),
                "before", state(code), "after", state(restored), "export_text", bytes.toString(StandardCharsets.UTF_8),
                "isolation", "Generated internal DTD only; external access denied; default parser security is not established"));
        String invalid = bytes.toString(StandardCharsets.UTF_8).replaceFirst("symmetry=\"[1-8]\"", "symmetry=\"9\"");
        Path invalidPath = output.resolve("invalid-symmetry.bgg");
        Files.writeString(invalidPath, invalid, StandardCharsets.UTF_8, StandardOpenOption.CREATE_NEW);
        boolean rejected = false;
        String diagnostic = "";
        try {
            parser.parseGeneticCode(invalidPath.toFile());
        } catch (SAXException expected) {
            rejected = true;
            diagnostic = expected.getMessage();
        }
        require(rejected, "Real XML parser rejects out-of-domain symmetry");
        emit("xml_invalid_symmetry", values("fixture", "invalid-symmetry.bgg", "input_symmetry", 9,
                "rejected", rejected, "diagnostic", diagnostic));
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("Usage: biogenesis.ReferenceOracle EMPTY_OUTPUT_DIRECTORY");
        if (!GraphicsEnvironment.isHeadless()) throw new IllegalStateException("Run with -Djava.awt.headless=true");
        Locale.setDefault(Locale.US);
        System.setProperty("javax.xml.accessExternalDTD", "");
        System.setProperty("javax.xml.accessExternalSchema", "");
        output = Path.of(args[0]).toAbsolutePath().normalize();
        Files.createDirectories(output);
        try (var files = Files.list(output)) {
            require(files.findAny().isEmpty(), "Fixture output directory must be empty");
        }
        emit("runtime_inputs", values("schema_version", 1, "java_version", System.getProperty("java.version"),
                "headless", true, "locale", Locale.getDefault().toLanguageTag(), "double_encoding", "Java hexadecimal strings",
                "preferences_read_or_written", false, "world_frames_executed", 0,
                "effective_reference_defaults", configuration()));
        randomObservations();
        geneObservations();
        GeneticCode code = genomeObservations();
        colorObservations();
        geometryObservations();
        World world = gasObservations();
        WorldStatistics stats = statisticsObservations(code);
        serializationFixture("gene", geneFixture());
        serializationFixture("genome", code);
        serializationFixture("statistics", stats);
        serializationFixture("world", world);
        xmlObservations(code);
        emit("checks_complete", values("checks", checks, "result", "PASS", "unexercised", List.of("world frames",
                "organism color contacts", "reproduction and placement", "real UI journeys", "network peers",
                "untrusted Java serialization", "default XML parser security", "Rust parity")));
    }
}
