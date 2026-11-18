// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Rectangle;
import java.io.BufferedWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import org.locationtech.jts.JTSVersion;
import org.locationtech.jts.geom.Envelope;
import org.locationtech.jts.index.strtree.STRtree;

/** Observe actual bundled JTS without advancing simulation or replacing game collaborators */
public final class SpatialOracle {
    private static BufferedWriter output;
    private static int cases;
    private static int queries;

    private record Item(int token, Rectangle bounds) {}

    private static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }

    private static void row(Object... values) throws Exception {
        for (int index = 0; index < values.length; index++) {
            if (index != 0) output.write('\t');
            output.write(values[index].toString());
        }
        output.write('\n');
    }

    private static String bits(double value) {
        return String.format(java.util.Locale.ROOT, "%016x", Double.doubleToRawLongBits(value));
    }

    private static Envelope envelope(Rectangle bounds) {
        return new Envelope(bounds.getX(), bounds.getMaxX(), bounds.getY(), bounds.getMaxY());
    }

    private static String tokens(List<?> values) {
        if (values.isEmpty()) return "~";
        StringBuilder text = new StringBuilder();
        for (Object value : values) {
            if (text.length() > 0) text.append(',');
            text.append(value);
        }
        return text.toString();
    }

    private static Map<Object, Integer> frequencies(List<?> values) {
        Map<Object, Integer> result = new HashMap<>();
        for (Object value : values) result.merge(value, 1, Integer::sum);
        return result;
    }

    private static void query(World world, List<Item> items, String name, Rectangle bounds) throws Exception {
        Envelope query = envelope(bounds);
        List<?> actual = world.colDetTree.query(query);
        List<Integer> membership = new ArrayList<>();
        for (Item item : items) {
            if (envelope(item.bounds).intersects(query)) membership.add(item.token);
        }
        require(frequencies(actual).equals(frequencies(membership)), "Query membership differs: " + name);
        require(actual.equals(world.colDetTree.query(query)), "Repeated query order changed: " + name);
        row("Q", name, bounds.x, bounds.y, bounds.width, bounds.height,
                bits(query.getMinX()), bits(query.getMaxX()), bits(query.getMinY()), bits(query.getMaxY()), tokens(actual));
        queries++;
    }

    private static void capture(String name, List<Item> items) throws Exception {
        row("C", name);
        World world = new World(null);
        world.colDetTree = new STRtree();
        for (Item item : items) {
            Rectangle b = item.bounds;
            Envelope e = envelope(b);
            world.colDetTree.insert(e, item.token);
            row("I", item.token, b.x, b.y, b.width, b.height,
                    bits(e.getMinX()), bits(e.getMaxX()), bits(e.getMinY()), bits(e.getMaxY()));
        }
        query(world, items, "all-normal", new Rectangle(-10000, -10000, 20000, 20000));
        query(world, items, "origin-point", new Rectangle(0, 0, 0, 0));
        query(world, items, "origin-negative", new Rectangle(40, 40, -80, -80));
        query(world, items, "far-away", new Rectangle(900000, 900000, 1, 1));
        query(world, items, "positive-maximum", new Rectangle(Integer.MAX_VALUE, Integer.MAX_VALUE, Integer.MAX_VALUE, Integer.MAX_VALUE));
        query(world, items, "negative-minimum", new Rectangle(Integer.MIN_VALUE, Integer.MIN_VALUE, Integer.MIN_VALUE, Integer.MIN_VALUE));
        int observed = 0;
        for (Item item : items) {
            if (observed++ == 4) break;
            Rectangle b = item.bounds;
            query(world, items, "item-" + observed, b);
            query(world, items, "item-origin-" + observed, new Rectangle(b.x, b.y, 0, 0));
            if (b.width >= 0 && b.x <= Integer.MAX_VALUE - b.width) {
                query(world, items, "item-right-edge-" + observed, new Rectangle(b.x + b.width, b.y, 0, 0));
                if (b.x + b.width < Integer.MAX_VALUE) {
                    query(world, items, "item-after-edge-" + observed, new Rectangle(b.x + b.width + 1, b.y, 0, 0));
                }
            }
        }
        Random random = new Random(880301);
        for (int index = 0; index < 16; index++) {
            query(world, items, "region-" + index, new Rectangle(random.nextInt(1200) - 600,
                    random.nextInt(1200) - 600, random.nextInt(500) - 200, random.nextInt(500) - 200));
        }
        row("E");
        cases++;
    }

    private static List<Item> generated(int count, int pattern, int order) {
        List<Item> result = new ArrayList<>();
        Random random = new Random(104729L + count * 37L + pattern);
        for (int index = 0; index < count; index++) {
            int width = (index % 9 - 2) * 2;
            int height = (index % 7 - 1) * 2;
            int centerX = pattern == 0 ? 0 : random.nextInt(201) - 100;
            int centerY = pattern == 1 ? 0 : random.nextInt(201) - 100;
            if (pattern == 3) { centerX = 0; centerY = 0; }
            result.add(new Item(11 + index * 17, new Rectangle(centerX - width / 2, centerY - height / 2, width, height)));
        }
        if (order == 1) Collections.reverse(result);
        if (order == 2) Collections.shuffle(result, new Random(1009 + count));
        return result;
    }

    private static void capture() throws Exception {
        String version = JTSVersion.CURRENT_VERSION.toString();
        require(version.equals("1.15.0"), "Unexpected JTS version: " + version);
        row("M", "JTSVersion", version);
        int[] counts = {0, 1, 9, 10, 11, 20, 21, 99, 100, 101, 257};
        for (int count : counts) {
            for (int pattern = 0; pattern < 3; pattern++) {
                for (int order = 0; order < 3; order++) {
                    capture("count-" + count + "-pattern-" + pattern + "-order-" + order, generated(count, pattern, order));
                }
            }
        }
        for (int count : new int[]{11, 101, 257}) {
            for (int order = 0; order < 3; order++) {
                capture("both-centers-tied-" + count + "-order-" + order, generated(count, 3, order));
            }
        }
        capture("nested-1001", generated(1001, 2, 2));
        capture("reference-scale-12500", generated(12500, 2, 2));
        capture("closed-boundaries-and-negative-dimensions", List.of(
                new Item(3, new Rectangle(10, 20, 30, 40)), new Item(5, new Rectangle(40, 60, 0, 0)),
                new Item(8, new Rectangle(50, 70, -10, -10)), new Item(13, new Rectangle(10, 20, 0, 40)),
                new Item(21, new Rectangle(10, 20, 30, 0)), new Item(34, new Rectangle(0, 0, -10, -10))));
        capture("integer-extremes", List.of(
                new Item(1, new Rectangle(Integer.MAX_VALUE, Integer.MAX_VALUE, Integer.MAX_VALUE, Integer.MAX_VALUE)),
                new Item(2, new Rectangle(Integer.MIN_VALUE, Integer.MIN_VALUE, Integer.MIN_VALUE, Integer.MIN_VALUE)),
                new Item(4, new Rectangle(Integer.MAX_VALUE, Integer.MIN_VALUE, Integer.MIN_VALUE, Integer.MAX_VALUE)),
                new Item(8, new Rectangle(Integer.MIN_VALUE, Integer.MAX_VALUE, Integer.MAX_VALUE, Integer.MIN_VALUE))));
        capture("duplicate-token-occurrences", List.of(new Item(7, new Rectangle(0, 0, 0, 0)),
                new Item(7, new Rectangle(0, 0, 0, 0)), new Item(7, new Rectangle(1, 1, -1, -1))));
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("Usage: biogenesis.SpatialOracle NEW_OUTPUT_DIRECTORY");
        Path directory = Path.of(args[0]).toAbsolutePath().normalize();
        Path parent = directory.getParent().toRealPath();
        for (String property : List.of("user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot")) {
            String value = System.getProperty(property);
            require(value != null && Path.of(value).toRealPath().startsWith(parent), "Explicit isolated profile required: " + property);
        }
        require(!Files.exists(directory), "Output directory must not exist");
        Files.createDirectory(directory);
        try (BufferedWriter writer = Files.newBufferedWriter(directory.resolve("spatial.tsv"), StandardCharsets.UTF_8)) {
            output = writer;
            capture();
        }
        System.out.println("SPATIAL_ORACLE PASS cases=" + cases + " queries=" + queries);
    }
}
