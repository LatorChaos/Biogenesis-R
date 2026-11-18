package biogenesis;

import java.awt.Color;
import java.awt.Rectangle;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.lang.reflect.Array;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.IdentityHashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.function.Predicate;
import org.locationtech.jts.geom.Envelope;
import org.locationtech.jts.index.strtree.STRtree;

/** Observe actual birth selection, placement, retries, and successful clade updates */
public final class BirthOracle {
    private static final String[] COLORS = ("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] ORGANISM_FIELDS = ("_ID _parentID _energy _reproduceEnergy _earlyReproduceEnergy _infectedReproduceEnergy _growthRatio _segments _age _max_age _nChildren _nVirusChildren _nTotalChildren _nTotalInfected _nPosition _timeToReproduce _timeToReproduceMax _reproducelate _sporetime _sporeversion _indigo _jadefactor _isaconsumer _isafungus _isaplant _isreproductive _isinfectious _isblond _isauburn _haseyes _hasgoodvision _canmove _canreact _candodge _usepretoucheffects _usefriendeffects _isenhanced _photosynthesis _methanotrophy _mphoto _segColor _color _framesColor active alive hasMoved dx dy dtheta _centerX _centerY _dCenterX _dCenterY _theta _mass _I _sizeRect").split(" ");
    private static final int SEARCH_LIMIT = 1024;
    private static int cases;

    private static final class Spec {
        String name, operation = "single", clade = "root";
        String[] colors = {"GREEN"};
        long seed = 827364;
        int symmetry = 1, mirror, rate, spore = 1, adapt = 1, cladeLimit = 25;
        int infectedSymmetry = 1, infectedMirror;
        boolean selfish, disperse, infected, saved, fullSnapshot, first = true;
        Double oxygenAfterPaste, geneLength;
        final Map<String, Object> controls = new LinkedHashMap<>();
        final Map<String, Object> settings = new LinkedHashMap<>();
        Spec(String name) {
            this.name = name;
            controls.put("_energy", 1000d);
            controls.put("_reproduceEnergy", 101);
            controls.put("_earlyReproduceEnergy", 0);
            controls.put("_nChildren", 3);
            controls.put("_nVirusChildren", 0);
            controls.put("dx", 1.25d);
            controls.put("dy", -0.5d);
        }
        Spec control(String field, Object value) { controls.put(field, value); return this; }
        Spec setting(String field, Object value) { settings.put(field, value); return this; }
    }

    private static final class Context {
        final Spec spec;
        final StringBuilder rows = new StringBuilder();
        final IdentityHashMap<GeneticCode, String> codes = new IdentityHashMap<>();
        final IdentityHashMap<Organism, String> names = new IdentityHashMap<>();
        final List<Organism> children = new ArrayList<>();
        World world;
        Organism parent, blocker, failed, child;
        GeneticCode originalInfected, initialSaved;
        boolean success;
        int successful;
        Context(Spec spec) { this.spec = spec; }
        void row(Object... values) {
            for (int i = 0; i < values.length; i++) {
                if (i != 0) rows.append('\t');
                rows.append(values[i]);
            }
            rows.append('\n');
        }
        String reference(GeneticCode code) {
            if (code == null) return "~";
            return codes.computeIfAbsent(code, ignored -> "g" + codes.size());
        }
        void genotype(String stage, String owner, GeneticCode code) throws Exception {
            row("G", stage, owner, "_reference", reference(code));
            if (code == null) return;
            row("G", stage, owner, "_nGenes", code.getNGenes());
            for (Field field : fields(GeneticCode.class)) {
                if (!field.getName().equals("_genes")) row("G", stage, owner, field.getName(), scalar(field.get(code)));
            }
            for (int i = 0; i < code.getNGenes(); i++) {
                for (Field field : fields(Gene.class)) row("N", stage, owner, i, field.getName(), scalar(field.get(code.getGene(i))));
            }
        }
        void organism(String stage, String owner, Organism organism) throws Exception {
            if (organism == null) return;
            Field[] selected;
            if (spec.fullSnapshot) selected = fields(Organism.class);
            else {
                selected = new Field[ORGANISM_FIELDS.length];
                for (int i = 0; i < selected.length; i++) {
                    selected[i] = Organism.class.getDeclaredField(ORGANISM_FIELDS[i]);
                    selected[i].setAccessible(true);
                }
            }
            for (Field field : selected) {
                row("O", stage, owner, field.getName(), field.getType().getSimpleName(), scalar(field.get(organism)));
            }
            row("O", stage, owner, "bounds", "Rectangle", scalar(organism.getBounds()));
        }
        void snapshot(String stage) throws Exception {
            organism(stage, "parent", parent);
            organism(stage, "child", child);
            genotype(stage, "parent.genotype", parent._geneticCode);
            genotype(stage, "parent.infected", parent._infectedGeneticCode);
            genotype(stage, "parent.saved", parent._savedGeneticCode);
            genotype(stage, "child.genotype", child == null ? null : child._geneticCode);
            if (failed != null) genotype(stage, "failed.genotype", failed._geneticCode);
            row("A", stage, "child_genotype_is_parent", child != null && child._geneticCode != null && child._geneticCode == parent._geneticCode);
            row("A", stage, "child_genotype_is_infected", child != null && child._geneticCode != null && child._geneticCode == parent._infectedGeneticCode);
            row("A", stage, "child_genotype_is_saved", child != null && child._geneticCode != null && child._geneticCode == parent._savedGeneticCode);
            row("A", stage, "child_genotype_is_failed", child != null && failed != null && child._geneticCode != null && child._geneticCode == failed._geneticCode);
            row("A", stage, "saved_is_failed", failed != null && parent._savedGeneticCode == failed._geneticCode);
            row("A", stage, "infected_is_original", originalInfected != null && parent._infectedGeneticCode == originalInfected);
            row("A", stage, "child_genotype_is_initial_saved", child != null && initialSaved != null && child._geneticCode == initialSaved);
            row("W", stage, "CO2", bits(world.getCO2()));
            row("W", stage, "O2", bits(world.getO2()));
            row("W", stage, "CH4", bits(world.getCH4()));
            row("W", stage, "NEXT_ID", world.NEXT_ID);
            row("W", stage, "stored_organisms", world._organisms.size());
            row("W", stage, "population", world.getPopulation());
            Random peek = copyRandom(Utils.random);
            row("R", stage, peek.nextLong(), peek.nextInt(10000), bits(peek.nextDouble()));
        }
        void tree(String stage, List<Organism> members) {
            world.colDetTree = new STRtree();
            row("T", stage, "rebuild", members.size());
            for (int i = 0; i < members.size(); i++) {
                Organism organism = members.get(i);
                Envelope envelope = new Envelope(organism.getX(), organism.getMaxX(), organism.getY(), organism.getMaxY());
                world.colDetTree.insert(envelope, organism);
                row("T", stage, i, names.get(organism), scalar(organism.getBounds()),
                        bits(envelope.getMinX()), bits(envelope.getMaxX()), bits(envelope.getMinY()), bits(envelope.getMaxY()));
            }
            world.colDetTree.build();
        }
        boolean attempt(int number, boolean first) throws Exception {
            String stage = "attempt-" + number;
            child = new Organism(world);
            names.put(child, "child-" + number);
            row("I", stage, "first", "boolean", first);
            snapshot(stage + "-before");
            success = child.inherit(parent, first);
            row("A", stage + "-after", "inherit_success", success);
            row("A", stage + "-after", "direction_x", Integer.signum(child._centerX - parent._centerX));
            row("A", stage + "-after", "direction_y", Integer.signum(child._centerY - parent._centerY));
            snapshot(stage + "-after");
            if (success) {
                successful++;
                children.add(child);
                world._organisms.add(child);
                row("L", stage + "-after", "append_without_tree_rebuild", names.get(child));
            }
            return success;
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
        if (value instanceof Rectangle rect) return rect.x + "," + rect.y + "," + rect.width + "," + rect.height;
        if (value instanceof World || value instanceof VisibleWorld || value instanceof GeneticCode) return "present";
        if (value.getClass().isArray()) {
            StringBuilder result = new StringBuilder();
            for (int i = 0; i < Array.getLength(value); i++) {
                if (i != 0) result.append(',');
                result.append(scalar(Array.get(value, i)));
            }
            return result.toString();
        }
        return value.toString();
    }
    private static void require(boolean condition, String message) { if (!condition) throw new IllegalStateException(message); }
    private static Random copyRandom(Random random) throws Exception {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (ObjectOutputStream stream = new ObjectOutputStream(bytes)) { stream.writeObject(random); }
        try (ObjectInputStream stream = new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))) {
            return (Random) stream.readObject();
        }
    }
    private static Color color(String name) throws Exception {
        switch (name) {
            case "GREEN": return Color.GREEN;
            case "WHITE": return Color.WHITE;
            case "BLUE": return Color.BLUE;
            case "RED": return Color.RED;
            case "CYAN": return Color.CYAN;
            case "GRAY": return Color.GRAY;
            case "PINK": return Color.PINK;
            case "DARKGRAY": return Color.DARK_GRAY;
            default: return (Color) Utils.class.getDeclaredField("Color" + name).get(null);
        }
    }
    private static GeneticCode genome(String[] colors, int symmetry, int mirror, int rate, int spore, int adapt,
            boolean selfish, boolean disperse, int generation, String clade) throws Exception {
        List<Gene> genes = new ArrayList<>();
        for (int i = 0; i < colors.length; i++) {
            Gene gene = new Gene();
            gene.setLength(12 + i * 2);
            gene.setTheta(i * 0.375);
            gene.setColor(color(colors[i]));
            gene.setBranch(-1);
            genes.add(gene);
        }
        GeneticCode code = new GeneticCode(genes, symmetry, mirror, rate, 0, 1, 1, spore, adapt,
                false, disperse, false, false, false, false, false, false,
                false, false, false, false, false, false, selfish);
        code._generation = generation;
        code._cladeID = clade;
        return code;
    }
    private static Context prepare(Spec spec) throws Exception {
        Context context = new Context(spec);
        for (Field original : Utils.class.getDeclaredFields()) {
            if (!original.getName().startsWith("DEF_")) continue;
            Field effective = Utils.class.getDeclaredField(original.getName().substring(4));
            effective.set(null, original.get(null));
        }
        Utils.MIN_MUTATION_RATE = 0;
        Utils.MAX_MUTATION_RATE = 10000;
        Utils.META_MUTATION_RATE = 0;
        Utils.CLADE_COMPLEXITY = spec.cladeLimit;
        for (String name : COLORS) Utils.class.getDeclaredField(name + "_PROB").setInt(null, name.equals("GREEN") ? 7 : 0);
        for (Map.Entry<String, Object> setting : spec.settings.entrySet()) Utils.class.getDeclaredField(setting.getKey()).set(null, setting.getValue());
        Utils.random = new Random(827364);
        context.world = new World(null);
        context.world.addCO2(Utils.INITIAL_CO2);
        context.world.addO2(Utils.INITIAL_O2);
        context.world.addCH4(Utils.INITIAL_CH4);
        GeneticCode code = genome(spec.colors, spec.symmetry, spec.mirror, spec.rate, spec.spore, spec.adapt,
                spec.selfish, spec.disperse, 41, spec.clade);
        if (spec.geneLength != null) for (Gene gene : code._genes) gene.setLength(spec.geneLength);
        context.parent = new Organism(context.world, code);
        require(context.parent.pasteOrganism(3500, 3500), "Real parent paste failed");
        context.world._organisms.add(context.parent);
        context.names.put(context.parent, "parent");
        if (spec.infected) {
            context.originalInfected = genome(new String[]{"WHITE"}, spec.infectedSymmetry, spec.infectedMirror,
                    0, 1, 1, false, false, 80, "virus");
            context.parent._infectedGeneticCode = context.originalInfected;
        }
        if (spec.saved) {
            context.initialSaved = genome(new String[]{"BLUE"}, 1, 0, 0, 1, 1, false, false, 91, "saved");
            context.parent._savedGeneticCode = context.initialSaved;
        }
        context.row("C", spec.name, spec.operation, spec.seed);
        for (Field effective : Utils.class.getDeclaredFields()) {
            if (Modifier.isFinal(effective.getModifiers()) || !Modifier.isStatic(effective.getModifiers())) continue;
            if (effective.getType().isPrimitive()) context.row("U", effective.getName(), effective.getType().getSimpleName(), scalar(effective.get(null)));
        }
        context.row("I", "setup", "parent_setup_seed", "long", 827364);
        if (spec.infected) context.row("I", "setup", "parent._infectedGeneticCode", "GeneticCode", "explicit-virus-genome");
        if (spec.saved) context.row("I", "setup", "parent._savedGeneticCode", "GeneticCode", "explicit-saved-genome");
        context.snapshot("pasted");
        for (Map.Entry<String, Object> entry : spec.controls.entrySet()) {
            Field field = Organism.class.getDeclaredField(entry.getKey());
            field.setAccessible(true);
            field.set(context.parent, entry.getValue());
            context.row("I", "parent-controls", entry.getKey(), field.getType().getSimpleName(), scalar(entry.getValue()));
        }
        if (spec.oxygenAfterPaste != null) {
            context.world.decreaseO2(context.world.getO2());
            context.world.addO2(spec.oxygenAfterPaste);
            context.row("I", "parent-controls", "world.O2_after_paste", "double", bits(spec.oxygenAfterPaste));
        }
        context.snapshot("prepared");
        return context;
    }
    private static void blocker(Context context, Rectangle bounds) throws Exception {
        GeneticCode code = genome(new String[]{"GREEN"}, 1, 0, 0, 1, 1, false, false, 1, "blocker");
        Organism organism = new Organism(context.world, code);
        require(organism.pasteOrganism(1000, 1000), "Real blocker paste failed");
        context.names.put(organism, "blocker");
        context.world._organisms.add(organism);
        organism.setBounds(bounds);
        context.row("I", "blocking", "blocker.bounds", "Rectangle", scalar(organism.getBounds()));
        context.blocker = organism;
    }
    private static Context execute(Spec spec) throws Exception {
        Rectangle previewBounds = null;
        if (spec.operation.equals("blocked-direction")) {
            Spec preview = new Spec("unblocked-preview").control("_nChildren", 1);
            preview.seed = spec.seed;
            Context result = execute(preview);
            require(result.success, "Unblocked source preview failed");
            previewBounds = result.child.getBounds();
        }
        Context context = prepare(spec);
        if (spec.operation.equals("retry")) blocker(context, new Rectangle(3400, 3400, 200, 200));
        if (previewBounds != null) {
            context.row("I", "blocking", "unblocked_source_child.bounds", "Rectangle", scalar(previewBounds));
            blocker(context, previewBounds);
        }
        context.tree("initial", context.blocker != null ? List.of(context.parent, context.blocker) : List.of(context.parent));
        Utils.random = new Random(spec.seed);
        context.row("I", "birth", "birth_seed_reset", "long", spec.seed);
        if (spec.operation.equals("litter")) {
            for (int i = 0; i < 9; i++) context.attempt(i, i == 0);
        } else if (spec.operation.equals("retry")) {
            context.attempt(0, spec.first);
            context.failed = context.child;
            context.world._organisms.remove(context.blocker);
            context.row("L", "retry", "remove_blocker", "blocker");
            context.tree("retry", List.of(context.parent));
            context.attempt(1, spec.first);
        } else if (spec.operation.equals("spore-wake")) {
            context.attempt(0, true);
            context.attempt(1, true);
        } else {
            context.attempt(0, spec.first);
        }
        context.row("E");
        return context;
    }
    private static Spec structural(String name) {
        Spec spec = new Spec(name);
        spec.colors = new String[]{"GREEN", "GREEN", "GREEN", "GREEN"};
        spec.symmetry = 4;
        spec.rate = 5000;
        spec.operation = "retry";
        spec.control("_nChildren", 1);
        return spec;
    }
    private static long search(Spec spec, Predicate<Context> predicate) throws Exception {
        for (int seed = 0; seed < SEARCH_LIMIT; seed++) {
            spec.seed = seed;
            Context context = execute(spec);
            if (predicate.test(context)) return seed;
        }
        throw new IllegalStateException("Bounded search exhausted: " + spec.name);
    }
    private static void adopt(StringBuilder output, Spec spec) throws Exception {
        output.append(execute(spec).rows);
        cases++;
    }
    private static Spec infection(String name) {
        Spec spec = new Spec(name);
        spec.infected = true;
        spec.control("_infectedReproduceEnergy", 121);
        return spec;
    }
    private static String capture() throws Exception {
        StringBuilder output = new StringBuilder();
        adopt(output, new Spec("ordinary"));
        Spec selfish = new Spec("selfish-integer-half"); selfish.selfish = true; adopt(output, selfish);
        adopt(output, new Spec("parent-energy-cap").control("_energy", 2.5d));
        Spec early = new Spec("early-threshold"); early.colors = new String[]{"GREEN", "BLOND"};
        early.control("_earlyReproduceEnergy", 61); adopt(output, early);
        adopt(output, infection("infected"));
        Spec jade = infection("jade-infected-cap"); jade.colors = new String[]{"GREEN", "JADE"};
        jade.control("_infectedReproduceEnergy", 201); adopt(output, jade);
        Spec consumer = infection("jade-consumer-exception"); consumer.colors = new String[]{"RED", "JADE"};
        consumer.control("_infectedReproduceEnergy", 201); adopt(output, consumer);
        Spec indigo = infection("indigo-infected"); indigo.colors = new String[]{"GREEN", "INDIGO"};
        indigo.control("_indigo", 20); adopt(output, indigo);
        Spec combined = infection("jade-indigo-infected"); combined.colors = new String[]{"GREEN", "JADE", "INDIGO"};
        combined.control("_indigo", 20).control("_infectedReproduceEnergy", 201); adopt(output, combined);
        for (int mirror = 0; mirror < 2; mirror++) {
            Spec low = infection("indigo-low-energy-mirror-" + mirror); low.infectedSymmetry = 3; low.infectedMirror = mirror;
            low.saved = true; low.control("_indigo", 20).control("_infectedReproduceEnergy", 10).control("_timeToReproduce", 35);
            adopt(output, low);
            Spec boundary = infection("indigo-boundary-mirror-" + mirror);
            boundary.infectedSymmetry = 3; boundary.infectedMirror = mirror;
            boundary.control("_indigo", 20).control("_infectedReproduceEnergy", 19).control("_timeToReproduce", 35);
            adopt(output, boundary);
        }
        Spec saved = infection("saved-overrides-infection-genotype"); saved.saved = true; adopt(output, saved);
        adopt(output, infection("infected-dead-birth").control("_infectedReproduceEnergy", 4));
        Spec single = new Spec("single-inherited-velocity").control("_nChildren", 1); adopt(output, single);
        Spec dispersed = new Spec("single-dispersal").control("_nChildren", 1); dispersed.disperse = true; adopt(output, dispersed);

        for (int sx = -1; sx <= 1; sx++) for (int sy = -1; sy <= 1; sy++) {
            if (sx == 0 && sy == 0) continue;
            final int x = sx, y = sy;
            Spec direction = new Spec("direction-" + sx + "-" + sy).control("_nChildren", 1);
            search(direction, result -> result.success && Integer.signum(result.child._centerX - result.parent._centerX) == x
                    && Integer.signum(result.child._centerY - result.parent._centerY) == y);
            adopt(output, direction);
            Spec blocked = new Spec("blocked-direction-" + sx + "-" + sy).control("_nChildren", 1);
            blocked.operation = "blocked-direction";
            search(blocked, result -> result.success && Integer.signum(result.child._centerX - result.parent._centerX) == x
                    && Integer.signum(result.child._centerY - result.parent._centerY) == y);
            adopt(output, blocked);
        }
        Spec litter = new Spec("litter-eight-reservations").control("_nChildren", 8).control("_energy", 10000d).control("_reproduceEnergy", 1000);
        litter.operation = "litter";
        search(litter, result -> result.successful == 8 && !result.success);
        adopt(output, litter);

        for (String kind : new String[]{"increase-genes", "decrease-genes", "increase-symmetry", "decrease-symmetry", "equal-symmetry"}) {
            Spec clade = structural("clade-retry-" + kind);
            search(clade, result -> {
                GeneticCode before = result.failed._geneticCode, parent = result.parent._geneticCode;
                if (before == null || result.successful != 1) return false;
                switch (kind) {
                    case "increase-genes": return before.getNGenes() > parent.getNGenes();
                    case "decrease-genes": return before.getNGenes() < parent.getNGenes();
                    case "increase-symmetry": return before.getSymmetry() > parent.getSymmetry();
                    case "decrease-symmetry": return before.getSymmetry() < parent.getSymmetry();
                    default: return before.getNGenes() == parent.getNGenes() && before.getSymmetry() == parent.getSymmetry()
                            && result.rows.indexOf("failed.genotype\t_updateClade\ttrue") >= 0;
                }
            });
            adopt(output, clade);
            if (kind.equals("increase-genes")) {
                for (int limit = 0; limit <= 2; limit++) {
                    Spec limited = structural("clade-marker-limit-" + limit);
                    limited.seed = clade.seed; limited.clade = "root+3"; limited.cladeLimit = limit;
                    adopt(output, limited);
                }
            }
        }
        for (int version = 1; version <= 12; version++) {
            Spec spore = new Spec("spore-parent-" + version);
            spore.colors = new String[]{"GREEN", "SPORE"}; spore.spore = version;
            spore.control("_sporetime", 30).control("_nChildren", 1);
            adopt(output, spore);
        }
        Spec wake = new Spec("spore-negative-time-wake");
        wake.colors = new String[]{"GREEN", "SPORE"}; wake.spore = 3; wake.operation = "spore-wake";
        wake.control("_sporetime", -30).control("_nChildren", 3); adopt(output, wake);

        for (int version : new int[]{1, 3}) for (boolean virus : new boolean[]{false, true}) {
            Spec spore = infection("spore-infected-" + version + (virus ? "-virus" : "-own"));
            spore.colors = new String[]{"GREEN", "SPORE"}; spore.spore = version;
            spore.control("_sporetime", 30).control("_nChildren", 1).control("_isreproductive", true);
            search(spore, result -> result.success && result.child._geneticCode != null
                    && result.child._geneticCode.getcladeID().startsWith(virus ? "virus" : "root"));
            adopt(output, spore);
        }
        Spec later = infection("spore-infected-later-litter");
        later.colors = new String[]{"GREEN", "SPORE"}; later.spore = 3; later.first = false;
        later.control("_sporetime", 30).control("_nChildren", 1).control("_nVirusChildren", 1).control("_isreproductive", true);
        adopt(output, later);
        for (boolean pass : new boolean[]{false, true}) {
            Spec negative = infection("spore-negative-infected-energy-" + pass);
            negative.colors = new String[]{"GREEN", "SPORE"}; negative.spore = 3;
            negative.control("_sporetime", 30).control("_nChildren", 1).control("_infectedReproduceEnergy", -121).control("_isreproductive", false);
            search(negative, result -> result.success == pass);
            adopt(output, negative);
        }
        Spec abort = infection("spore-virus-delay-abort"); abort.colors = new String[]{"GREEN", "SPORE"}; abort.spore = 3;
        abort.control("_sporetime", 30).control("_nChildren", 3).control("_nVirusChildren", 1).control("_isreproductive", true);
        search(abort, result -> !result.success && result.child._geneticCode == null);
        adopt(output, abort);
        return output.toString();
    }
    private static Spec variant(String name, int modifier, String... colors) {
        Spec spec = new Spec(name);
        spec.spore = modifier;
        spec.colors = colors;
        spec.fullSnapshot = true;
        spec.control("_sporetime", 30).control("_nChildren", 1);
        return spec;
    }
    private static String variants() throws Exception {
        StringBuilder output = new StringBuilder();
        for (String[] pair : new String[][]{
                {"plant-consumer", "GREEN", "RED", "SPORE"},
                {"plant-consumer-moving", "GREEN", "RED", "CYAN", "SPORE"},
                {"nonplant-consumer", "RED", "SPORE"},
                {"plant-still", "GREEN", "SPORE"},
                {"plant-moving", "GREEN", "CYAN", "SPORE"},
                {"unclassified-still", "SPORE"},
                {"unclassified-moving", "CYAN", "SPORE"}}) {
            Spec spec = variant("v3-" + pair[0], 3, Arrays.copyOfRange(pair, 1, pair.length));
            spec.disperse = true;
            adopt(output, spec);
        }
        Spec firstConsumer = variant("v4-consumer-first-draw", 4, "RED", "SPORE");
        firstConsumer.seed = 0;
        int firstTime = execute(firstConsumer).child._timeToReproduce;
        adopt(output, firstConsumer);
        Spec otherConsumer = variant("v4-consumer-other-draw", 4, "RED", "SPORE");
        search(otherConsumer, result -> result.success && result.child._timeToReproduce != firstTime);
        adopt(output, otherConsumer);
        adopt(output, variant("v4-pure-plant", 4, "GREEN", "SPORE"));
        for (String[] pair : new String[][]{
                {"plant-consumer-methanotroph", "GREEN", "RED", "PURPLE", "SPORE"},
                {"plant-consumer-moving-jade", "GREEN", "RED", "CYAN", "JADE", "SPORE"},
                {"plant-consumer-still-jade", "GREEN", "RED", "JADE", "SPORE"},
                {"pure-plant-methanotroph", "GREEN", "PURPLE", "SPORE"},
                {"pretouch-two", "GREEN", "DARK", "SPORE"},
                {"killer", "GREEN", "GRAY", "SPORE"},
                {"fallow", "GREEN", "FALLOW", "SPORE"},
                {"lime", "GREEN", "LIME", "SPORE"},
                {"forest", "GREEN", "FOREST", "SPORE"},
                {"enhanced-grass", "GREEN", "DARKGRAY", "SPORE"},
                {"jade", "GREEN", "JADE", "SPORE"},
                {"jade-blond", "GREEN", "JADE", "BLOND", "SPORE"},
                {"ordinary-green", "GREEN", "SPORE"},
                {"nonplant-consumer", "RED", "SPORE"},
                {"unclassified", "SPORE"},
                {"plant-fungus", "GREEN", "PINK", "SPORE"},
                {"gold", "GREEN", "GOLD", "SPORE"}}) {
            adopt(output, variant("v6-" + pair[0], 6, Arrays.copyOfRange(pair, 1, pair.length)));
        }
        for (int mirror = 0; mirror < 2; mirror++) {
            Spec spec = variant("v6-symmetry-three-mirror-" + mirror, 6, "GREEN", "SPORE");
            spec.symmetry = 3; spec.mirror = mirror;
            adopt(output, spec);
        }
        Spec tiny = variant("v6-no-large-segment", 6, "GREEN", "SPORE"); tiny.geneLength = 0.25;
        adopt(output, tiny);
        for (int modifier = 1; modifier <= 2; modifier++) {
            for (String[] pair : new String[][]{
                    {"plant", "GREEN", "SPORE"},
                    {"mixed", "GREEN", "RED", "SPORE"},
                    {"consumer", "RED", "SPORE"},
                    {"unclassified", "SPORE"}}) {
                adopt(output, variant("v" + modifier + "-classification-" + pair[0], modifier, Arrays.copyOfRange(pair, 1, pair.length)));
            }
            for (double oxygen : new double[]{10, 1, 0}) {
                Spec spec = variant("v" + modifier + "-respiration-oxygen-" + (int) oxygen, modifier, "GREEN", "SPORE");
                spec.setting("SPORE_ENERGY_CONSUMPTION", 5d); spec.oxygenAfterPaste = oxygen;
                adopt(output, spec);
            }
            Spec fallback = variant("v" + modifier + "-respiration-energy-fallback", modifier, "GREEN", "SPORE");
            fallback.setting("SPORE_ENERGY_CONSUMPTION", 5d); fallback.oxygenAfterPaste = 10d;
            fallback.control("_energy", 2d);
            adopt(output, fallback);
            Spec capped = variant("v" + modifier + "-long-spore", modifier, "GREEN", "SPORE"); capped.geneLength = 100d;
            adopt(output, capped);
        }
        for (int modifier = 1; modifier <= 12; modifier++) {
            adopt(output, variant("eye-rebuild-modifier-" + modifier, modifier, "GREEN", "SPORE", "EYE"));
        }
        for (int modifier : new int[]{7, 10}) for (int adapt : new int[]{20, 30, 40, 50, 60, 70}) {
            for (boolean applied : new boolean[]{false, true}) {
                if ((adapt == 20 && !applied) || (adapt == 70 && applied)) continue;
                Spec spec = variant("adapt-" + modifier + "-" + adapt + "-" + applied, modifier, "GREEN", "SPORE");
                spec.adapt = adapt;
                search(spec, result -> result.success && result.child.active != applied);
                adopt(output, spec);
            }
        }
        return output.toString();
    }
    public static void main(String[] args) throws Exception {
        if (args.length < 1 || args.length > 2 || (args.length == 2 && !args[1].equals("variants"))) {
            throw new IllegalArgumentException("Usage: biogenesis.BirthOracle NEW_OUTPUT_DIRECTORY [variants]");
        }
        Path directory = Path.of(args[0]).toAbsolutePath().normalize();
        Path parent = directory.getParent().toRealPath();
        for (String property : List.of("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot")) {
            String value = System.getProperty(property);
            require(value != null && Path.of(value).toRealPath().startsWith(parent), "Explicit isolated profile required: " + property);
        }
        require(!Files.exists(directory), "Output directory must not exist");
        Files.createDirectory(directory);
        boolean variantMode = args.length == 2;
        Files.writeString(directory.resolve(variantMode ? "birth_variants.tsv" : "birth.tsv"), variantMode ? variants() : capture(), StandardCharsets.UTF_8);
        System.out.println("BIRTH_ORACLE PASS cases=" + cases + " search_limit=" + SEARCH_LIMIT + (variantMode ? " mode=variants" : ""));
    }
}
