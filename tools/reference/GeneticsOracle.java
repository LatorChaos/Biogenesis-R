package biogenesis;

import java.awt.Color;
import java.io.BufferedWriter;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.function.Predicate;

/** Observe the unchanged genetics implementation with real seeded Java random draws */
public final class GeneticsOracle {
    private static final String[] COLORS = ("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] REACTIONS = ("red green blue plague scourge white gray silver default consumer plant magenta pink coral orange bark violet virus maroon olive mint cream spike fallow lightblue ochre sky lilac fire lightbrown greenbrown brown ice broken sick friend").split(" ");
    private static final int SEARCH_LIMIT = 2048;
    private static BufferedWriter output;
    private static BufferedWriter traces;
    private static int cases;
    private static final Set<String> coverage = new LinkedHashSet<>();

    private static final class Spec {
        String name;
        String operation;
        long seed;
        int complexity = 2, minimum = 0, maximum = 100, meta = 0;
        int count = 5, rate = 0, symmetry = 4, branchMode = 0, generation = 41;
        String clade = "root+3-5>8<9";
        int[] weights;
        boolean edit, rawMetadata, constructorDefaults;
        Spec(String name, String operation, long seed) { this.name = name; this.operation = operation; this.seed = seed; }
    }

    private static final class Draw {
        final String kind, value, site;
        final int bound, line;
        Draw(String kind, int bound, String value) {
            this.kind = kind; this.bound = bound; this.value = value;
            String found = "external";
            int foundLine = -1;
            for (StackTraceElement frame : Thread.currentThread().getStackTrace()) {
                if (frame.getClassName().equals("biogenesis.GeneticCode") || frame.getClassName().equals("biogenesis.Gene") || frame.getClassName().equals("biogenesis.Utils")) {
                    found = frame.getClassName() + ":" + frame.getMethodName();
                    foundLine = frame.getLineNumber();
                    break;
                }
            }
            site = found; line = foundLine;
        }
    }

    private static final class RecordingRandom extends Random {
        final List<Draw> draws = new ArrayList<>();
        RecordingRandom(long seed) { super(seed); }
        @Override public int nextInt(int bound) { int value = super.nextInt(bound); draws.add(new Draw("int", bound, Integer.toString(value))); return value; }
        @Override public boolean nextBoolean() { boolean value = super.nextBoolean(); draws.add(new Draw("boolean", 0, Boolean.toString(value))); return value; }
        @Override public double nextDouble() { double value = super.nextDouble(); draws.add(new Draw("double", 0, bits(value))); return value; }
        @Override public long nextLong() { long value = super.nextLong(); draws.add(new Draw("long", 0, Long.toString(value))); return value; }
    }

    private static final class Result {
        final StringBuilder rows = new StringBuilder();
        final Set<String> tags = new LinkedHashSet<>();
        GeneticCode input, child;
        String tail;
        List<Draw> draws = List.of();
        void add(Object... values) {
            for (int i = 0; i < values.length; i++) { if (i != 0) rows.append('\t'); rows.append(values[i]); }
            rows.append('\n');
        }
        void observe(String label, GeneticCode code, GeneticCode parent) throws Exception {
            for (Field field : fields(GeneticCode.class)) {
                if (field.getName().equals("_genes")) continue;
                add("G", label, field.getName(), scalar(field.get(code)));
            }
            for (int i = 0; i < code.getNGenes(); i++) {
                Gene gene = code.getGene(i);
                for (Field field : fields(Gene.class)) add("N", label, i, field.getName(), scalar(field.get(gene)));
                int shared = -1, alias = -1;
                if (parent != null) for (int j = 0; j < parent.getNGenes(); j++) if (gene == parent.getGene(j)) { shared = j; break; }
                for (int j = 0; j < i; j++) if (gene == code.getGene(j)) { alias = j; break; }
                add("N", label, i, "_shared_parent_index", shared);
                add("N", label, i, "_alias_index", alias);
            }
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
        return value.toString();
    }
    private static void require(boolean condition, String message) { if (!condition) throw new IllegalStateException(message); }

    private static void configure(Spec spec) throws Exception {
        for (String name : COLORS) {
            Field field = Utils.class.getDeclaredField(name + "_PROB");
            Field original = Utils.class.getDeclaredField("DEF_" + name + "_PROB");
            field.setInt(null, original.getInt(null));
        }
        if (spec.weights != null) for (int i = 0; i < COLORS.length; i++) Utils.class.getDeclaredField(COLORS[i] + "_PROB").setInt(null, spec.weights[i]);
        Utils.INITIAL_COMPLEXITY = spec.complexity;
        Utils.MIN_MUTATION_RATE = spec.minimum;
        Utils.MAX_MUTATION_RATE = spec.maximum;
        Utils.META_MUTATION_RATE = spec.meta;
        Utils.CLADE_COMPLEXITY = 25;
    }

    private static Gene explicitGene(int index) {
        int base = 1000 + index * 100;
        return new Gene(20.25 + index, -0.375 - index / 8.0, new Color(0x12000000 | ((index + 1) * 0x12345 & 0xffffff), true), index == 0 ? -1 : index,
                base, base+1, base+2, base+3, base+4, base+5, base+6, base+7, base+8, base+9, base+10, base+11,
                base+12, base+13, base+14, base+15, base+16, base+17, base+18, base+19, base+20, base+21, base+22, base+23,
                base+24, base+25, base+26, base+27, base+28, base+29, base+30, base+31, base+32, base+33, base+34, base+35);
    }

    private static GeneticCode explicit(List<Gene> genes, Spec spec) {
        GeneticCode code = new GeneticCode(genes, spec.symmetry, spec.rawMetadata ? -7 : 1, spec.rate,
                spec.rawMetadata ? -4 : 2, spec.rawMetadata ? Integer.MIN_VALUE : 7,
                spec.rawMetadata ? -9 : 4, spec.rawMetadata ? 99 : 12, spec.rawMetadata ? -30 : 70,
                true, false, true, false, true, false, true, false, true, false, true, false, true, false, true);
        if (!spec.constructorDefaults) {
            code._generation = spec.generation;
            code._cladeID = spec.clade;
            code._updateClade = true;
        }
        return code;
    }

    private static GeneticCode parent(Spec spec) {
        List<Gene> genes = new ArrayList<>();
        for (int i = 0; i < spec.count; i++) {
            Gene gene = explicitGene(i);
            if (spec.branchMode == 1) gene.setBranch(i + 4);
            if (spec.branchMode == 2) gene.setBranch(i % 2 == 0 ? -1 : i);
            genes.add(gene);
        }
        return explicit(genes, spec);
    }

    private static void editGene(Gene gene, boolean all) throws Exception {
        gene.setLength(all ? Double.longBitsToDouble(0x7ff8000000000042L) : -1234.5);
        if (all) { gene.setTheta(-0.0); gene.setColor(null); gene.setBranch(Integer.MIN_VALUE); }
        for (int i = 0; i < (all ? REACTIONS.length : 1); i++) {
            Gene.class.getMethod("set" + REACTIONS[i] + "Reaction", int.class).invoke(gene, all ? Integer.MIN_VALUE + i : -777);
        }
    }

    private static Result run(Spec spec, boolean recording) throws Exception {
        configure(spec);
        Utils.random = recording ? new RecordingRandom(spec.seed) : new Random(spec.seed);
        Result result = new Result();
        try {
            switch (spec.operation) {
                case "gene-blank": result.child = explicit(List.of(new Gene()), spec); break;
                case "gene-explicit": {
                    result.child = explicit(List.of(explicitGene(0)), spec);
                    result.observe("input", result.child, null);
                    break;
                }
                case "gene-setters": {
                    result.input = explicit(List.of(new Gene()), spec);
                    result.observe("input", result.input, null);
                    editGene(result.input.getGene(0), true);
                    result.child = result.input;
                    result.add("A", "edit_all_fields", true);
                    break;
                }
                case "gene-random": {
                    Gene gene = new Gene(); gene.randomize(); result.child = explicit(List.of(gene), spec); break;
                }
                case "gene-color": {
                    Gene gene = explicitGene(0);
                    result.input = explicit(List.of(gene), spec); result.observe("input", result.input, null);
                    gene.randomizeColor(); result.child = result.input; break;
                }
                case "gene-clone": {
                    Gene gene = explicitGene(0);
                    result.input = explicit(List.of(gene), spec); result.observe("input", result.input, null);
                    result.child = explicit(List.of((Gene) gene.clone()), spec);
                    result.add("A", "clone_gene_distinct", result.child.getGene(0) != gene);
                    result.add("A", "clone_color_shared", result.child.getGene(0).getColor() == gene.getColor());
                    break;
                }
                case "genome-empty": result.child = explicit(List.of(), spec); break;
                case "genome-explicit": {
                    Gene shared = explicitGene(0);
                    List<Gene> supplied = new ArrayList<>(List.of(shared, shared, explicitGene(2)));
                    result.input = explicit(supplied, spec);
                    result.child = explicit(supplied, spec);
                    result.observe("input", result.input, null);
                    result.observe("output", result.child, result.input);
                    result.add("A", "explicit_array_distinct", result.input._genes != result.child._genes);
                    supplied.set(0, explicitGene(9));
                    result.add("A", "list_replacement_detached", result.child.getGene(0) == shared);
                    result.add("A", "edit_supplied_gene_length", bits(-1234.5));
                    result.add("A", "edit_supplied_gene_redreaction", -777);
                    editGene(shared, false);
                    result.observe("input_after", result.input, null);
                    result.observe("output_after", result.child, result.input);
                    result.add("A", "supplied_gene_edit_shared", result.child.getGene(1).getLength() == shared.getLength());
                    break;
                }
                case "genome-random": result.child = new GeneticCode(); break;
                case "genome-clone": {
                    Gene shared = explicitGene(0);
                    result.input = explicit(List.of(shared, shared, explicitGene(2)), spec);
                    result.observe("input", result.input, null);
                    result.child = (GeneticCode) result.input.clone();
                    result.add("A", "clone_array_distinct", result.child._genes != result.input._genes);
                    result.add("A", "clone_repeated_slots_distinct", result.child.getGene(0) != result.child.getGene(1));
                    break;
                }
                case "inherit": {
                    result.input = parent(spec); result.observe("input", result.input, null);
                    result.child = new GeneticCode(result.input); break;
                }
                default: throw new IllegalArgumentException("Unknown operation: " + spec.operation);
            }
            if (!spec.operation.equals("genome-explicit")) result.observe("output", result.child, result.input);
            if (spec.operation.equals("inherit") && !spec.edit) result.observe("input_after", result.input, null);
            if (spec.edit || spec.operation.endsWith("clone")) {
                result.add("A", "edit_child_gene0_length", bits(-1234.5));
                result.add("A", "edit_child_gene0_redreaction", -777);
                editGene(result.child.getGene(0), false);
                result.observe("input_after", result.input, null);
                result.observe("output_after", result.child, result.input);
            }
        } catch (IllegalArgumentException | NegativeArraySizeException error) {
            result.add("X", error.getClass().getSimpleName(), error.getMessage());
        }
        if (recording) result.draws = new ArrayList<>(((RecordingRandom) Utils.random).draws);
        long nextLong = Utils.random.nextLong();
        int nextInt = Utils.random.nextInt(10000);
        double nextDouble = Utils.random.nextDouble();
        result.tail = nextLong + "\t" + nextInt + "\t" + bits(nextDouble);
        classify(spec, result);
        return result;
    }

    private static Draw at(Result result, int line) {
        for (Draw draw : result.draws) if (draw.site.equals("biogenesis.GeneticCode:<init>") && draw.line == line) return draw;
        return null;
    }

    private static void classify(Spec spec, Result result) {
        if (!spec.operation.equals("inherit") || result.child == null) return;
        int difference = result.child.getNGenes() - result.input.getNGenes();
        result.tags.add("count:" + (difference > 0 ? "insert" : difference < 0 ? "delete" : "unchanged"));
        for (Draw draw : result.draws) {
            if (!draw.site.startsWith("biogenesis.GeneticCode")) continue;
            if (draw.kind.equals("boolean")) result.tags.add("boolean:" + draw.line + ":" + draw.value);
            if (draw.kind.equals("int") && draw.bound == 10000) result.tags.add("gate:" + draw.line + ":" + (Integer.parseInt(draw.value) < result.child.getMutationrate()));
        }
        Draw symmetry = at(result, 760);
        if (symmetry != null && Integer.parseInt(symmetry.value) < result.child.getMutationrate()) result.tags.add("symmetry:" + (result.child.getSymmetry() == result.input.getSymmetry() ? "selected-same" : "selected-different"));
        Draw added = at(result, 776), copied = at(result, 803);
        if (added != null && copied != null) {
            int index = Integer.parseInt(added.value);
            if (!copied.value.equals("0")) result.tags.add("insert:new");
            else if (index == 0) result.tags.add("insert:clone-first");
            else if (index == result.child.getNGenes() - 1) result.tags.add("insert:clone-last");
            else result.tags.add("insert:clone-" + (at(result, 809).value.equals("true") ? "before" : "after"));
        }
        Draw meta = at(result, 728);
        if (meta != null) result.tags.add("meta:" + (meta.value.equals("0") ? "large-" + (at(result, 730).value.equals("true") ? "increase" : "decrease") : "small-" + (at(result, 739).value.equals("true") ? "increase" : "decrease")));
    }

    private static void emit(Spec spec) throws Exception {
        Result recorded = run(spec, true);
        Result plain = run(spec, false);
        require(recorded.rows.toString().equals(plain.rows.toString()) && recorded.tail.equals(plain.tail), "Recording changes actual Random behavior: " + spec.name);
        output.write("C\t" + spec.name + "\t" + spec.operation + "\t" + spec.seed + "\t" + spec.complexity + "\t" + spec.minimum + "\t" + spec.maximum + "\t" + spec.meta + "\t25\n");
        configure(spec);
        for (int i = 0; i < COLORS.length; i++) output.write("W\t" + i + "\t" + COLORS[i] + "\t" + Utils.class.getDeclaredField(COLORS[i] + "_PROB").getInt(null) + "\n");
        output.write(recorded.rows.toString());
        output.write("R\t" + recorded.tail + "\nE\n");
        for (Draw draw : recorded.draws) traces.write(spec.name + "\t" + draw.site + "\t" + draw.line + "\t" + draw.kind + "\t" + draw.bound + "\t" + draw.value + "\n");
        coverage.addAll(recorded.tags);
        cases++;
    }

    private static Spec find(Spec spec, Predicate<Result> selected) throws Exception {
        for (long seed = 0; seed < SEARCH_LIMIT; seed++) {
            spec.seed = seed;
            Result result = run(spec, true);
            if (selected.test(result)) return spec;
        }
        throw new IllegalStateException("Bounded search found no case: " + spec.name);
    }

    private static void scenarios() throws Exception {
        for (String operation : List.of("gene-blank", "gene-explicit", "gene-setters", "gene-clone", "genome-empty", "genome-explicit", "genome-clone")) emit(new Spec(operation, operation, 42));
        Spec defaults = new Spec("genome-explicit-constructor-defaults", "genome-explicit", 42);
        defaults.constructorDefaults = true; emit(defaults);
        Spec raw = new Spec("genome-explicit-raw-metadata", "genome-explicit", 42);
        raw.rawMetadata = true; raw.symmetry = -3; raw.rate = -17; emit(raw);
        for (long seed : new long[]{0, 1, -1, Long.MIN_VALUE, Long.MAX_VALUE}) emit(new Spec("gene-random-" + seed, "gene-random", seed));
        for (int color = 0; color < COLORS.length; color++) {
            Spec spec = new Spec("one-hot-" + COLORS[color], "gene-color", 7);
            spec.weights = new int[COLORS.length]; spec.weights[color] = 7; emit(spec);
        }
        Spec weighted = new Spec("custom-weights", "gene-random", 2026);
        weighted.weights = new int[COLORS.length];
        for (int i = 0; i < COLORS.length; i++) weighted.weights[i] = i % 5 + 1;
        emit(weighted);
        for (int complexity : new int[]{1, 2, 7, 101}) {
            Spec spec = new Spec("random-complexity-" + complexity, "genome-random", 827364);
            spec.complexity = complexity; spec.minimum = 25; spec.maximum = 75;
            emit(spec);
        }
        Spec fixedRate = new Spec("random-fixed-mutation-rate", "genome-random", 2026);
        fixedRate.minimum = fixedRate.maximum = 73; emit(fixedRate);
        Spec invalid = new Spec("invalid-complexity-zero", "genome-random", 42); invalid.complexity = 0; emit(invalid);
        invalid = new Spec("invalid-weights-zero", "gene-random", 42); invalid.weights = new int[COLORS.length]; emit(invalid);
        for (int count : new int[]{1, 5, 101}) {
            Spec spec = new Spec("inherit-no-mutation-" + count, "inherit", 827364); spec.count = count; spec.edit = count == 5; emit(spec);
        }
        Spec overflow = new Spec("generation-wrap-null-clade", "inherit", 9); overflow.generation = Integer.MAX_VALUE; overflow.clade = null; emit(overflow);
        for (long seed : new long[]{0, 1}) {
            Spec spec = new Spec("inherit-full-" + seed, "inherit", seed); spec.maximum = spec.rate = 10000; emit(spec);
        }
        for (int rate : new int[]{-50, 20000}) {
            Spec spec = new Spec("clamp-" + rate, "inherit", 17); spec.minimum = 10; spec.maximum = 100; spec.rate = rate; emit(spec);
        }
        Set<String> found = new LinkedHashSet<>();
        for (long seed = 0; seed < SEARCH_LIMIT; seed++) {
            Spec spec = new Spec("partial-" + seed, "inherit", seed); spec.maximum = 10000; spec.rate = 2500; spec.branchMode = 2;
            Result result = run(spec, true);
            if (!found.containsAll(result.tags)) { emit(spec); found.addAll(result.tags); }
        }
        Spec retainBranch = new Spec("clone-retains-nonnegative-branch", "inherit", 0);
        retainBranch.maximum = 10000; retainBranch.rate = 2500;
        emit(find(retainBranch, result -> at(result, 952) != null && at(result, 952).value.equals("false")));
        for (String goal : List.of("meta:large-increase", "meta:large-decrease", "meta:small-increase", "meta:small-decrease")) {
            Spec spec = new Spec(goal.replace(':', '-'), "inherit", 0); spec.meta = 10000; spec.rate = 50; spec.count = 1;
            emit(find(spec, result -> result.tags.contains(goal)));
        }
        for (int bound : new int[]{0, 100}) {
            for (String direction : List.of("large-increase", "large-decrease", "small-increase", "small-decrease")) {
                Spec spec = new Spec("meta-bound-" + bound + "-" + direction, "inherit", 0); spec.meta = 10000; spec.rate = bound; spec.count = 1;
                emit(find(spec, result -> result.tags.contains("meta:" + direction)));
            }
        }
        for (int extreme : new int[]{Integer.MIN_VALUE, Integer.MAX_VALUE}) {
            String direction = extreme < 0 ? "small-decrease" : "small-increase";
            Spec spec = new Spec("meta-wrapping-" + extreme, "inherit", 0);
            spec.meta = 10000; spec.rate = extreme; spec.count = 1;
            emit(find(spec, result -> result.tags.contains("meta:" + direction)));
        }
        for (int count : new int[]{1, 100, 101}) {
            for (String direction : List.of("true", "false")) {
                Spec spec = new Spec("count-limit-" + count + "-increase-" + direction, "inherit", 0); spec.count = count; spec.maximum = 10000; spec.rate = 2500; spec.branchMode = 1;
                emit(find(spec, result -> at(result, 770) != null && at(result, 770).value.equals(direction)));
            }
        }
        Spec aboveAdditionCap = new Spec("random-complexity-101-full-count", "genome-random", 0);
        aboveAdditionCap.complexity = 101;
        emit(find(aboveAdditionCap, result -> result.child != null && result.child.getNGenes() == 101));
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("Usage: biogenesis.GeneticsOracle NEW_OUTPUT_DIRECTORY");
        Path directory = Path.of(args[0]).toAbsolutePath().normalize();
        Path parent = directory.getParent().toRealPath();
        for (String property : List.of("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot")) {
            String value = System.getProperty(property);
            require(value != null && Path.of(value).toRealPath().startsWith(parent), "Explicit isolated profile required: " + property);
        }
        require(!Files.exists(directory), "Output directory must not exist");
        Files.createDirectory(directory);
        try (BufferedWriter states = Files.newBufferedWriter(directory.resolve("genetics.tsv"), StandardCharsets.UTF_8);
             BufferedWriter draws = Files.newBufferedWriter(directory.resolve("traces.tsv"), StandardCharsets.UTF_8)) {
            output = states; traces = draws;
            scenarios();
        }
        Files.write(directory.resolve("coverage.txt"), coverage, StandardCharsets.UTF_8);
        System.out.println("GENETICS_ORACLE PASS cases=" + cases + " coverage_tags=" + coverage.size() + " search_limit=" + SEARCH_LIMIT);
    }
}
