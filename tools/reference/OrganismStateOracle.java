package biogenesis;

import java.awt.Color;
import java.awt.Rectangle;
import java.io.BufferedWriter;
import java.lang.reflect.Array;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;
import java.util.Random;

/** Observe real organism initialization and explicitly controlled effect reapplication */
public final class OrganismStateOracle {
    private static final String[] COLORS = ("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL = ("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static final String[] REACTIONS = ("red green blue plague scourge white gray silver default consumer plant magenta pink coral orange bark violet virus maroon olive mint cream spike fallow lightblue ochre sky lilac fire lightbrown greenbrown brown ice broken sick friend").split(" ");
    private static BufferedWriter output;
    private static int cases;

    private static final class Spec {
        final String name, operation;
        long seed = 827364;
        int symmetry = 1, mirror, spore = 1, activity, age = 1, reapplies;
        int[] branches;
        String[] colors = {"GREEN"};
        String oneHot, phenotype;
        boolean altruist, passive, pink, sky, plague, clockwise;
        Spec(String name, String operation, String... colors) {
            this.name = name; this.operation = operation;
            if (colors.length > 0) this.colors = colors;
        }
    }

    private static Field[] fields(Class<?> type) {
        return Arrays.stream(type.getDeclaredFields()).filter(field -> !Modifier.isStatic(field.getModifiers()))
                .peek(field -> field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);
    }
    private static String bits(double value) { return String.format(java.util.Locale.ROOT, "%016x", Double.doubleToRawLongBits(value)); }
    private static String scalar(Object value) {
        if (value == null) return "~";
        if (value instanceof Double number) return bits(number);
        if (value instanceof Color color) return String.format(java.util.Locale.ROOT, "%08x", color.getRGB());
        if (value instanceof Rectangle rectangle) return rectangle.x + "," + rectangle.y + "," + rectangle.width + "," + rectangle.height;
        if (value.getClass().isArray()) {
            StringBuilder result = new StringBuilder();
            for (int i = 0; i < Array.getLength(value); i++) { if (i != 0) result.append(','); result.append(scalar(Array.get(value, i))); }
            return result.toString();
        }
        if (value instanceof Number || value instanceof Boolean || value instanceof String) return value.toString();
        if (value instanceof World || value instanceof VisibleWorld || value instanceof GeneticCode) return "present";
        throw new IllegalArgumentException("Unsupported observation type: " + value.getClass().getName());
    }
    private static void row(Object... values) throws Exception {
        for (int i = 0; i < values.length; i++) { if (i != 0) output.write('\t'); output.write(values[i].toString()); }
        output.write('\n');
    }
    private static void require(boolean condition, String message) { if (!condition) throw new IllegalStateException(message); }

    private static Color color(String name) throws Exception {
        switch (name) {
            case "RED": return Color.RED;
            case "GREEN": return Color.GREEN;
            case "BLUE": return Color.BLUE;
            case "CYAN": return Color.CYAN;
            case "WHITE": return Color.WHITE;
            case "GRAY": return Color.GRAY;
            case "MAGENTA": return Color.MAGENTA;
            case "PINK": return Color.PINK;
            case "ORANGE": return Color.ORANGE;
            case "YELLOW": return Color.YELLOW;
            case "SILVER": return Color.LIGHT_GRAY;
            case "DARKGRAY": return Color.DARK_GRAY;
            default: return (Color) Utils.class.getDeclaredField("Color" + name).get(null);
        }
    }

    private static void configure(Spec spec) throws Exception {
        for (Field original : Utils.class.getDeclaredFields()) {
            if (!original.getName().startsWith("DEF_")) continue;
            Field effective = Utils.class.getDeclaredField(original.getName().substring(4));
            effective.set(null, original.get(null));
        }
        Utils.INITIAL_COMPLEXITY = 1;
        if (spec.oneHot != null) for (String name : COLORS) Utils.class.getDeclaredField(name + "_PROB").setInt(null, name.equals(spec.oneHot) ? 7 : 0);
    }

    private static World world() {
        World world = new World(null);
        world.addO2(Utils.INITIAL_O2);
        world.addCO2(Utils.INITIAL_CO2);
        world.addCH4(Utils.INITIAL_CH4);
        return world;
    }

    private static GeneticCode genome(Spec spec) throws Exception {
        List<Gene> genes = new ArrayList<>();
        for (int i = 0; i < spec.colors.length; i++) {
            Gene gene = new Gene();
            gene.setLength(6.25 + 3.5 * i);
            gene.setTheta(0.25 + 0.37 * i);
            gene.setColor(color(spec.colors[i]));
            gene.setBranch(spec.branches == null ? -1 : spec.branches[i]);
            for (int j = 0; j < REACTIONS.length; j++) Gene.class.getMethod("set" + REACTIONS[j] + "Reaction", int.class).invoke(gene, (i + j) % 6);
            genes.add(gene);
        }
        return new GeneticCode(genes, spec.symmetry, spec.mirror, 50, spec.activity, 2, 1, spec.spore, 30,
                spec.plague, false, false, false, spec.altruist, false, false, false, spec.passive, spec.clockwise,
                false, spec.pink, false, spec.sky, false);
    }

    private static void observeGenome(String stage, GeneticCode genome) throws Exception {
        if (genome == null) return;
        for (Field field : fields(GeneticCode.class)) if (!field.getName().equals("_genes")) row("G", stage, field.getName(), scalar(field.get(genome)));
        for (int i = 0; i < genome.getNGenes(); i++) {
            for (Field field : fields(Gene.class)) row("N", stage, i, field.getName(), scalar(field.get(genome.getGene(i))));
        }
    }

    private static void observe(String stage, Organism organism, World world) throws Exception {
        for (Field field : fields(Organism.class)) row("O", stage, field.getName(), field.getType().getTypeName(), scalar(field.get(organism)));
        row("O", stage, "bounds", "java.awt.Rectangle", scalar(organism.getBounds()));
        observeGenome(stage, organism.getGeneticCode());
        for (String name : new String[]{"_width", "_height", "NEXT_ID", "_population", "_O2", "_CO2", "_CH4", "_corridorexists"}) {
            Field field = World.class.getDeclaredField(name);
            row("W", stage, name, field.getType().getTypeName(), scalar(field.get(world)));
        }
        row("W", stage, "organism_count", "int", world._organisms.size());
    }

    private static void emit(Spec spec) throws Exception {
        configure(spec);
        Utils.random = new Random(spec.seed);
        row("C", spec.name, spec.operation, spec.seed);
        Field[] defaults = Arrays.stream(Utils.class.getDeclaredFields()).filter(field -> field.getName().startsWith("DEF_")).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);
        for (Field original : defaults) {
            Field effective = Utils.class.getDeclaredField(original.getName().substring(4));
            row("U", effective.getName(), effective.getType().getTypeName(), scalar(effective.get(null)));
        }
        row("I", "paste_x", "int", 3500);
        row("I", "paste_y", "int", 3500);
        row("I", "reapply_count", "int", spec.reapplies);
        row("I", "controlled_age", "int", spec.age);
        row("I", "controlled_phenotype", "java.awt.Color", spec.phenotype == null ? "~" : scalar(color(spec.phenotype)));
        World world = world();
        Organism organism = spec.operation.equals("randomCreate") || spec.operation.equals("constructor")
                ? new Organism(world) : new Organism(world, genome(spec));
        observe("constructor", organism, world);
        if (!spec.operation.equals("constructor")) {
            boolean success = spec.operation.equals("randomCreate") ? organism.randomCreate() : organism.pasteOrganism(3500, 3500);
            row("A", "initialization_succeeded", success);
            require(success, "Actual organism initialization failed: " + spec.name);
            observe("initial", organism, world);
        }
        if (spec.reapplies > 0) {
            Organism.class.getDeclaredField("_age").setInt(organism, spec.age);
            if (spec.phenotype != null) Arrays.fill(organism._segColor, color(spec.phenotype));
            observe("controlled_input", organism, world);
            for (int i = 0; i < spec.reapplies; i++) {
                organism.segmentsCreateEffects();
                observe(i == 0 ? "reapplied" : "reapplied_twice", organism, world);
            }
        }
        row("R", Utils.random.nextLong(), Utils.random.nextInt(10000), bits(Utils.random.nextDouble()));
        row("E");
        cases++;
    }

    private static Spec seekSymmetry(int symmetry) throws Exception {
        Spec spec = new Spec("random-symmetry-" + symmetry, "randomCreate");
        spec.oneHot = "GREEN";
        for (long seed = 0; seed < 256; seed++) {
            configure(spec); Utils.random = new Random(seed);
            Organism organism = new Organism(world());
            if (organism.randomCreate() && organism.getGeneticCode().getSymmetry() == symmetry) { spec.seed = seed; return spec; }
        }
        throw new IllegalStateException("Symmetry search exhausted: " + symmetry);
    }

    private static void scenarios() throws Exception {
        emit(new Spec("constructor-only", "constructor"));
        for (String name : COLORS) {
            Spec spec = new Spec("random-one-hot-" + name, "randomCreate");
            spec.oneHot = name; emit(spec);
        }
        for (int symmetry : new int[]{1, 3, 8}) emit(seekSymmetry(symmetry));
        for (int symmetry : new int[]{1, 3, 8}) for (int mirror : new int[]{0, 1}) {
            Spec spec = new Spec("paste-symmetry-" + symmetry + "-mirror-" + mirror, "pasteOrganism", "GREEN", "YELLOW", "MINT", "INDIGO", "MAGENTA");
            spec.symmetry = symmetry; spec.mirror = mirror; emit(spec);
        }
        for (int modifier = 1; modifier <= 12; modifier++) {
            Spec spec = new Spec("spore-" + modifier, "pasteOrganism", "SPORE", "LAVENDER");
            spec.spore = modifier; spec.symmetry = 3; spec.reapplies = 2; emit(spec);
            Spec c4 = new Spec("c4-spore-" + modifier, "pasteOrganism", "C4", "SPORE");
            c4.spore = modifier; c4.symmetry = 8; emit(c4);
        }
        String[][] combinations = {
            {"SILVER", "CORAL"}, {"SILVER", "PINK"}, {"SILVER", "GRAY"}, {"SILVER", "PLAGUE"}, {"SPIKE", "DARKGRAY"},
            {"C4"}, {"C4", "INDIGO"}, {"C4", "GREEN"}, {"C4", "BLUE"}, {"C4", "OLIVE"}, {"C4", "DARK"}, {"C4", "PURPLE"}, {"C4", "WHITE"},
            {"ROSE"}, {"ROSE", "ROSE"}, {"ROSE", "BLOND", "WHITE"}, {"LAVENDER", "WHITE", "BLOND"}, {"BLOND", "GREEN"}, {"BLOND", "RED"},
            {"FLOWER", "WHITE"}, {"FLOWER", "GREEN"}, {"GOLD", "GOLD"}, {"PURPLE"}, {"PURPLE", "GREEN"}, {"PURPLE", "DARKGRAY"},
            {"TEAL"}, {"TEAL", "BLUE"}, {"TEAL", "OLIVE"}, {"TEAL", "WHITE"}, {"EYE", "GREEN"}, {"EYE", "EYE", "CYAN"}
        };
        for (int i = 0; i < combinations.length; i++) for (boolean modifier : new boolean[]{false, true}) {
            Spec spec = new Spec("mixed-" + i + "-flags-" + modifier, "pasteOrganism", combinations[i]);
            spec.altruist = spec.passive = spec.pink = spec.sky = spec.plague = spec.clockwise = modifier;
            spec.symmetry = modifier ? 3 : 1; spec.mirror = modifier ? 1 : 0; emit(spec);
        }
        for (int activity = 0; activity <= 2; activity++) {
            Spec spec = new Spec("summer-activity-" + activity, "pasteOrganism", "SUMMER");
            spec.activity = activity; spec.reapplies = 1; emit(spec);
        }
        Spec eye = new Spec("eye-branch-reference", "pasteOrganism", "EYE", "GREEN", "CYAN");
        eye.symmetry = 8; eye.branches = new int[]{-1, 0, 1}; emit(eye);
        for (String name : INTERNAL) {
            Spec spec = new Spec("controlled-phenotype-" + name, "controlledCreateEffects", "GREEN");
            spec.phenotype = name; spec.reapplies = 2; emit(spec);
        }
        Spec spring = new Spec("controlled-darkgreen-spring", "controlledCreateEffects", "SPRING");
        spring.phenotype = "DARKGREEN"; spring.reapplies = 1; spring.clockwise = true; emit(spring);
        Spec jade = new Spec("controlled-darkgreen-jade", "controlledCreateEffects", "JADE");
        jade.phenotype = "DARKGREEN"; jade.reapplies = 1; emit(jade);
        Spec repeat = new Spec("controlled-spore-age-zero-repeat", "controlledCreateEffects", "SPORE", "LAVENDER");
        repeat.age = 0; repeat.reapplies = 2; repeat.spore = 8; emit(repeat);
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("Usage: biogenesis.OrganismStateOracle NEW_OUTPUT_DIRECTORY");
        Path directory = Path.of(args[0]).toAbsolutePath().normalize();
        Path parent = directory.getParent().toRealPath();
        for (String property : List.of("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot")) {
            String value = System.getProperty(property);
            require(value != null && Path.of(value).toRealPath().startsWith(parent), "Explicit isolated profile required: " + property);
        }
        require(!Files.exists(directory), "Output directory must not exist");
        Files.createDirectory(directory);
        try (BufferedWriter writer = Files.newBufferedWriter(directory.resolve("organism_state.tsv"), StandardCharsets.UTF_8)) {
            output = writer; scenarios();
        }
        System.out.println("ORGANISM_STATE_ORACLE PASS cases=" + cases);
    }
}
