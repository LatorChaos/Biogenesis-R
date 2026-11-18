package biogenesis;

import java.awt.Color;
import java.io.BufferedWriter;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Objects;
import java.util.Random;

/** Observe genotype aliases without executing contacts or network operations */
public final class GenotypeIdentityOracle {
    private static BufferedWriter output;

    private static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }

    private static String bits(double value) {
        return String.format(java.util.Locale.ROOT, "%016x", Double.doubleToRawLongBits(value));
    }

    private static String nullable(Object value) { return value == null ? "~" : value.toString(); }

    private static void row(Object... values) throws Exception {
        for (int i = 0; i < values.length; i++) {
            if (i != 0) output.write('\t');
            output.write(values[i].toString());
        }
        output.write('\n');
    }

    private static GeneticCode genome() {
        Gene gene = new Gene();
        gene.setLength(12);
        gene.setTheta(0);
        gene.setColor(Color.GREEN);
        gene.setBranch(0);
        GeneticCode code = new GeneticCode(List.of(gene), 1, 0, 0, 0, 1, 1, 1, 1,
                false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false);
        code._generation = 7;
        code._cladeID = "root";
        code._updateClade = true;
        return code;
    }

    private static boolean sameGeneValues(Gene left, Gene right) throws Exception {
        for (Field field : Gene.class.getDeclaredFields()) {
            if (Modifier.isStatic(field.getModifiers())) continue;
            field.setAccessible(true);
            if (!Objects.equals(field.get(left), field.get(right))) return false;
        }
        return true;
    }

    private static boolean sameGenomeValues(GeneticCode left, GeneticCode right) throws Exception {
        for (Field field : GeneticCode.class.getDeclaredFields()) {
            if (Modifier.isStatic(field.getModifiers()) || field.getName().equals("_genes")) continue;
            field.setAccessible(true);
            if (!Objects.equals(field.get(left), field.get(right))) return false;
        }
        if (left.getNGenes() != right.getNGenes()) return false;
        for (int i = 0; i < left.getNGenes(); i++) {
            if (!sameGeneValues(left.getGene(i), right.getGene(i))) return false;
        }
        return true;
    }

    private static void values(String stage, String owner, GeneticCode code) throws Exception {
        row("V", stage, owner, "present", code != null);
        if (code == null) return;
        row("V", stage, owner, "_generation", code.getGeneration());
        row("V", stage, owner, "_cladeID", nullable(code.getcladeID()));
        row("V", stage, owner, "_updateClade", code.getUpdateClade());
        row("V", stage, owner, "gene0._length", bits(code.getGene(0).getLength()));
        row("V", stage, owner, "gene0._redreaction", code.getGene(0).getredReaction());
    }

    private static void observe(String stage, World world, Organism a, Organism b, Organism c) throws Exception {
        row("B", stage, "a_genotype_is_b_genotype", a._geneticCode == b._geneticCode);
        row("B", stage, "a_genotype_is_c_genotype", a._geneticCode == c._geneticCode);
        row("B", stage, "a_genotype_values_equal_c", sameGenomeValues(a._geneticCode, c._geneticCode));
        row("B", stage, "a_gene_array_is_b_gene_array", a._geneticCode._genes == b._geneticCode._genes);
        row("B", stage, "a_gene_array_is_c_gene_array", a._geneticCode._genes == c._geneticCode._genes);
        row("B", stage, "a_gene0_is_b_gene0", a._geneticCode.getGene(0) == b._geneticCode.getGene(0));
        row("B", stage, "a_gene0_is_c_gene0", a._geneticCode.getGene(0) == c._geneticCode.getGene(0));
        row("B", stage, "a_color0_is_c_color0", a._geneticCode.getGene(0).getColor() == c._geneticCode.getGene(0).getColor());
        row("B", stage, "b_infected_is_a_genotype", b._infectedGeneticCode == a._geneticCode);
        row("B", stage, "b_infected_is_b_genotype", b._infectedGeneticCode == b._geneticCode);
        row("B", stage, "b_infected_is_c_genotype", b._infectedGeneticCode == c._geneticCode);
        row("B", stage, "b_saved_is_null", b._savedGeneticCode == null);
        row("B", stage, "c_saved_is_a_genotype", c._savedGeneticCode == a._geneticCode);
        row("B", stage, "statistics_last_infected_is_b_genotype", world.worldStatistics.getLastInfectedBeing() == b._geneticCode);
        row("B", stage, "statistics_most_infections_is_a_genotype", world.worldStatistics.getBeingMostInfections() == a._geneticCode);
        row("S", stage, "a._nTotalInfected", a.getTotalInfected());
        row("S", stage, "c._nTotalInfected", c.getTotalInfected());
        row("S", stage, "b._infectedReproduceEnergy", b._infectedReproduceEnergy);
        row("S", stage, "statistics.beingMostInfectionsNumber", world.worldStatistics.getBeingMostInfectionsNumber());
        Field infections = WorldStatistics.class.getDeclaredField("infectionsSum");
        infections.setAccessible(true);
        row("S", stage, "statistics.infectionsSum", infections.getLong(world.worldStatistics));
        values(stage, "a.genotype", a._geneticCode);
        values(stage, "b.genotype", b._geneticCode);
        values(stage, "c.genotype", c._geneticCode);
        values(stage, "b.infected", b._infectedGeneticCode);
        values(stage, "c.saved", c._savedGeneticCode);
        values(stage, "statistics.lastInfected", world.worldStatistics.getLastInfectedBeing());
        values(stage, "statistics.mostInfections", world.worldStatistics.getBeingMostInfections());
    }

    private static void capture() throws Exception {
        long seed = 506172;
        Utils.random = new Random(seed);
        VisibleWorld visible = new VisibleWorld(null);
        World world = new World(visible);
        GeneticCode shared = genome();
        GeneticCode copy = (GeneticCode) shared.clone();
        Organism a = new Organism(world, shared);
        Organism b = new Organism(world, shared);
        Organism c = new Organism(world, copy);
        visible._selectedOrganism = b;
        row("C", "shared-genotype", seed);
        row("I", "setup", "visible._selectedOrganism", "b");
        row("I", "setup", "a._reproduceEnergy", 123);
        row("I", "setup", "c._reproduceEnergy", -41);
        row("I", "setup", "b._infectedReproduceEnergy", -17);
        a._reproduceEnergy = 123;
        c._reproduceEnergy = -41;
        b._infectedReproduceEnergy = -17;
        observe("constructed", world, a, b, c);

        row("I", "infection-organism-shared", "b._savedGeneticCode", "c.genotype");
        b._savedGeneticCode = copy;
        b.infectedBy(a);
        observe("infection-organism-shared", world, a, b, c);

        row("I", "shared-edit", "a.genotype._cladeID", "fixture:root");
        row("I", "shared-edit", "a.genotype._generation", 42);
        row("I", "shared-edit", "a.genotype._updateClade", false);
        row("I", "shared-edit", "a.genotype.gene0._length", bits(25.5));
        row("I", "shared-edit", "a.genotype.gene0._redreaction", -7);
        row("I", "shared-edit", "c._savedGeneticCode", "a.genotype");
        shared._cladeID = "fixture:root";
        shared._generation = 42;
        shared._updateClade = false;
        shared.getGene(0).setLength(25.5);
        shared.getGene(0).setredReaction(-7);
        c._savedGeneticCode = shared;
        observe("shared-edit", world, a, b, c);

        row("I", "infection-organism-clone", "b._savedGeneticCode", "a.genotype");
        b._savedGeneticCode = shared;
        b.infectedBy(c);
        observe("infection-organism-clone", world, a, b, c);

        row("I", "infection-code-shared", "b._savedGeneticCode", "c.genotype");
        b._savedGeneticCode = copy;
        b.infectedBy(shared);
        observe("infection-code-shared", world, a, b, c);

        row("I", "shared-second-edit", "a.genotype._cladeID", "fixture:root.2");
        row("I", "shared-second-edit", "a.genotype._generation", -5);
        row("I", "shared-second-edit", "a.genotype._updateClade", true);
        shared._cladeID = "fixture:root.2";
        shared._generation = -5;
        shared._updateClade = true;
        observe("shared-second-edit", world, a, b, c);

        Random plain = new Random(seed);
        for (int i = 0; i < 3; i++) plain.nextDouble();
        long nextLong = Utils.random.nextLong();
        int nextInt = Utils.random.nextInt(10000);
        double nextDouble = Utils.random.nextDouble();
        require(nextLong == plain.nextLong() && nextInt == plain.nextInt(10000)
                && bits(nextDouble).equals(bits(plain.nextDouble())), "Unexpected RNG draws");
        row("R", nextLong, nextInt, bits(nextDouble));
        row("E");
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("Usage: biogenesis.GenotypeIdentityOracle NEW_OUTPUT_DIRECTORY");
        Path directory = Path.of(args[0]).toAbsolutePath().normalize();
        Path parent = directory.getParent().toRealPath();
        for (String property : List.of("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot")) {
            String value = System.getProperty(property);
            require(value != null && Path.of(value).toRealPath().startsWith(parent), "Explicit isolated profile required: " + property);
        }
        require(!Files.exists(directory), "Output directory must not exist");
        Files.createDirectory(directory);
        try (BufferedWriter writer = Files.newBufferedWriter(directory.resolve("genotype_identity.tsv"), StandardCharsets.UTF_8)) {
            output = writer;
            capture();
        }
        System.out.println("GENOTYPE_IDENTITY_ORACLE PASS cases=1 stages=6");
    }
}
