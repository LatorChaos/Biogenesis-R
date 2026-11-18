package biogenesis;

import java.awt.Color;
import java.awt.Rectangle;
import java.io.BufferedWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Random;

/** Observe actual paste rejection without replacing world or visual collaborators */
public final class PasteBoundaryOracle {
    private static BufferedWriter output;

    private static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }

    private static String bits(double value) {
        return String.format(java.util.Locale.ROOT, "%016x", Double.doubleToRawLongBits(value));
    }

    private static String nullable(String value) { return value == null ? "~" : value; }

    private static void row(Object... values) throws Exception {
        for (int i = 0; i < values.length; i++) {
            if (i != 0) output.write('\t');
            output.write(values[i].toString());
        }
        output.write('\n');
    }

    private static GeneticCode genome(int branch) {
        Gene gene = new Gene();
        gene.setLength(12);
        gene.setTheta(0);
        gene.setColor(Color.GREEN);
        gene.setBranch(branch);
        return new GeneticCode(List.of(gene), 1, 0, 0, 0, 1, 1, 1, 1,
                false, false, false, false, false, false, false, false,
                false, false, false, false, false, false, false);
    }

    private static World world() {
        World world = new World(null);
        world.addCO2(Utils.INITIAL_CO2);
        world.addO2(Utils.INITIAL_O2);
        world.addCH4(Utils.INITIAL_CH4);
        return world;
    }

    private static void observe(String name, int x, int y, int branch, boolean overlap) throws Exception {
        long seed = 827364;
        Utils.random = new Random(seed);
        World world = world();
        row("C", name, seed, world.getWidth(), world.getHeight(), x, y, branch, overlap);
        if (overlap) {
            Organism blocker = new Organism(world, genome(0));
            require(blocker.pasteOrganism(3500, 3500), "Real blocker paste failed");
            world._organisms.add(blocker);
            Rectangle bounds = blocker.getBounds();
            row("K", bounds.x, bounds.y, bounds.width, bounds.height,
                    bits(world.getCO2()), bits(world.getO2()), bits(world.getCH4()), world.NEXT_ID);
        }
        GeneticCode code = genome(branch);
        row("B", bits(world.getCO2()), bits(world.getO2()), bits(world.getCH4()),
                world.NEXT_ID, code.getGeneration(), nullable(code.getcladeID()));
        Organism candidate = new Organism(world, code);
        boolean success = candidate.pasteOrganism(x, y);
        Rectangle bounds = candidate.getBounds();
        row("A", success, bits(world.getCO2()), bits(world.getO2()), bits(world.getCH4()),
                world.NEXT_ID, code.getGeneration(), nullable(code.getcladeID()),
                bounds.x, bounds.y, bounds.width, bounds.height);
        row("R", Utils.random.nextLong(), Utils.random.nextInt(10000), bits(Utils.random.nextDouble()));
        row("E");
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("Usage: biogenesis.PasteBoundaryOracle NEW_OUTPUT_DIRECTORY");
        Path directory = Path.of(args[0]).toAbsolutePath().normalize();
        Path parent = directory.getParent().toRealPath();
        for (String property : List.of("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot")) {
            String value = System.getProperty(property);
            require(value != null && Path.of(value).toRealPath().startsWith(parent), "Explicit isolated profile required: " + property);
        }
        require(!Files.exists(directory), "Output directory must not exist");
        Files.createDirectory(directory);
        try (BufferedWriter writer = Files.newBufferedWriter(directory.resolve("paste_rejection.tsv"), StandardCharsets.UTF_8)) {
            output = writer;
            observe("valid-control", 3500, 3500, 0, false);
            observe("first-branch-999", 3500, 3500, 999, false);
            observe("outside-left", -100, 3500, 0, false);
            observe("overlapping-placement", 3500, 3500, 0, true);
        }
        System.out.println("PASTE_BOUNDARY_ORACLE PASS cases=4");
    }
}
