// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Color;
import java.awt.Graphics2D;
import java.awt.geom.AffineTransform;
import java.awt.image.BufferedImage;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.lang.reflect.Constructor;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Random;

/** Observe original preview drawing independently of actual organism placement */
public final class PreviewBranchOracle {
    private static final long SEED = 827364;
    private static int cases;

    private static void require(boolean condition, String message) {
        if (!condition) throw new IllegalArgumentException(message);
    }

    private static String bits(double value) {
        return String.format("%016x", Double.doubleToRawLongBits(value));
    }

    private static double number(String text) {
        require(text.matches("[0-9a-f]{16}"), "Invalid double bits");
        return Double.longBitsToDouble(Long.parseUnsignedLong(text, 16));
    }

    private static void row(Object... values) {
        StringBuilder result = new StringBuilder();
        for (Object value : values) {
            if (result.length() > 0) result.append('\t');
            result.append(value);
        }
        System.out.println(result);
    }

    private static byte[] serialized(Object value) throws Exception {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (ObjectOutputStream stream = new ObjectOutputStream(bytes)) {
            stream.writeObject(value);
        }
        return bytes.toByteArray();
    }

    private static Random copyRandom() throws Exception {
        try (ObjectInputStream stream = new ObjectInputStream(new ByteArrayInputStream(serialized(Utils.random)))) {
            return (Random) stream.readObject();
        }
    }

    private static void random(String stage) throws Exception {
        Random copy = copyRandom();
        row("R", stage, copy.nextLong(), copy.nextInt(10000), bits(copy.nextDouble()));
    }

    private static Gene gene(String[] input) throws Exception {
        require(input.length == 5 && input[0].equals("G"), "Invalid gene input");
        double length = number(input[1]), theta = number(input[2]);
        require(Double.isFinite(length) && Double.isFinite(theta), "Nonfinite input");
        require(input[3].matches("[0-9a-f]{8}"), "Invalid ARGB");
        Constructor<?> constructor = Arrays.stream(Gene.class.getConstructors())
            .filter(item -> item.getParameterCount() == 40).findFirst().orElseThrow();
        Object[] arguments = new Object[40];
        arguments[0] = length;
        arguments[1] = theta;
        arguments[2] = new Color(Integer.parseUnsignedInt(input[3], 16), true);
        arguments[3] = Integer.parseInt(input[4]);
        for (int index = 4; index < arguments.length; index++) arguments[index] = 0;
        return (Gene) constructor.newInstance(arguments);
    }

    private static void world(String stage, World world, GeneticCode code) {
        row("W", stage, bits(world.getCO2()), bits(world.getO2()), bits(world.getCH4()),
            world.NEXT_ID, code.getGeneration(), code.getcladeID() == null ? "~" : code.getcladeID());
    }

    private static void observe(String[] input, List<Gene> genes) throws Exception {
        require(input.length == 7 && input[0].equals("C"), "Invalid case input");
        int symmetry = Integer.parseInt(input[2]), mirror = Integer.parseInt(input[3]);
        int width = Integer.parseInt(input[4]), height = Integer.parseInt(input[5]);
        require(symmetry >= 1 && symmetry <= 8 && (mirror == 0 || mirror == 1), "Invalid symmetry or mirror");
        require(width > 0 && height > 0 && width <= 100 && height <= 100, "Invalid viewport");
        require(genes.size() == Integer.parseInt(input[6]), "Gene count differs");
        require(genes.size() <= 129, "Input gene bound");
        GeneticCode code = new GeneticCode(genes, symmetry, mirror, 0, 0, 1, 1, 1, 1,
            false, false, false, false, false, false, false, false,
            false, false, false, false, false, false, false);
        row((Object[]) input);
        for (Gene gene : genes) {
            row("G", bits(gene.getLength()), bits(gene.getTheta()),
                String.format("%08x", gene.getColor().getRGB()), gene.getBranch());
        }
        Utils.random = new Random(SEED);
        random("before-preview");
        byte[] before = serialized(code);
        TraceGraphics graphics = new TraceGraphics(new BufferedImage(width, height, BufferedImage.TYPE_INT_ARGB).createGraphics());
        try {
            code.draw(graphics, width, height);
            row("B", "preview", true);
        } catch (ArrayIndexOutOfBoundsException error) {
            row("B", "preview", false);
            row("X", "preview", error.getClass().getName());
        }
        AffineTransform transform = graphics.getTransform();
        row("T", bits(transform.getTranslateX()), bits(transform.getTranslateY()),
            bits(transform.getScaleX()), bits(transform.getScaleY()));
        for (int[] line : graphics.lines) {
            row("L", line[0], line[1], line[2], line[3], String.format("%08x", line[4]));
        }
        graphics.dispose();
        row("A", "preview_genotype_unchanged", Arrays.equals(before, serialized(code)));
        random("after-preview");

        Utils.WORLD_WIDTH = 7000;
        Utils.WORLD_HEIGHT = 7000;
        Utils.INITIAL_ORGANISMS = 0;
        Utils.INITIAL_O2 = 0;
        Utils.INITIAL_CO2 = 100000;
        Utils.INITIAL_CH4 = 0;
        Utils.INITIAL_ENERGY = 40;
        World world = new World(null);
        world("before-paste", world, code);
        Organism organism = new Organism(world, code);
        try {
            row("B", "paste", organism.pasteOrganism(3500, 3500));
        } catch (ArrayIndexOutOfBoundsException error) {
            row("X", "paste", error.getClass().getName());
        }
        world("after-paste", world, code);
        random("after-paste");
        row("E");
        cases++;
    }

    public static void main(String[] arguments) throws Exception {
        require(arguments.length == 1, "Expected immutable input path");
        Path input = Path.of(arguments[0]);
        require(Files.isRegularFile(input) && Files.size(input) <= 100000, "Input path or bound");
        String[] current = null;
        List<Gene> genes = new ArrayList<>();
        for (String line : Files.readAllLines(input)) {
            String[] fields = line.split("\\t", -1);
            switch (fields[0]) {
                case "C":
                    require(current == null, "Nested case");
                    current = fields;
                    genes = new ArrayList<>();
                    break;
                case "G":
                    require(current != null, "Gene outside case");
                    genes.add(gene(fields));
                    break;
                case "E":
                    require(fields.length == 1 && current != null, "Invalid case end");
                    observe(current, genes);
                    current = null;
                    break;
                default:
                    throw new IllegalArgumentException("Unknown input tag");
            }
        }
        require(current == null && cases == 31, "Incomplete case input");
        System.err.println("PREVIEW_BRANCH_ORACLE PASS cases=" + cases);
    }


    static final class TraceGraphics extends Graphics2D {
        final Graphics2D delegate;
        final List<int[]> lines = new ArrayList<>();
        TraceGraphics(Graphics2D delegate) { this.delegate = delegate; }
        @Override public void draw(java.awt.Shape a0) { delegate.draw(a0); }
        @Override public boolean drawImage(java.awt.Image a0, java.awt.geom.AffineTransform a1, java.awt.image.ImageObserver a2) { return delegate.drawImage(a0, a1, a2); }
        @Override public void drawImage(java.awt.image.BufferedImage a0, java.awt.image.BufferedImageOp a1, int a2, int a3) { delegate.drawImage(a0, a1, a2, a3); }
        @Override public void drawRenderedImage(java.awt.image.RenderedImage a0, java.awt.geom.AffineTransform a1) { delegate.drawRenderedImage(a0, a1); }
        @Override public void drawRenderableImage(java.awt.image.renderable.RenderableImage a0, java.awt.geom.AffineTransform a1) { delegate.drawRenderableImage(a0, a1); }
        @Override public void drawString(java.lang.String a0, int a1, int a2) { delegate.drawString(a0, a1, a2); }
        @Override public void drawString(java.lang.String a0, float a1, float a2) { delegate.drawString(a0, a1, a2); }
        @Override public void drawString(java.text.AttributedCharacterIterator a0, int a1, int a2) { delegate.drawString(a0, a1, a2); }
        @Override public void drawString(java.text.AttributedCharacterIterator a0, float a1, float a2) { delegate.drawString(a0, a1, a2); }
        @Override public void drawGlyphVector(java.awt.font.GlyphVector a0, float a1, float a2) { delegate.drawGlyphVector(a0, a1, a2); }
        @Override public void fill(java.awt.Shape a0) { delegate.fill(a0); }
        @Override public boolean hit(java.awt.Rectangle a0, java.awt.Shape a1, boolean a2) { return delegate.hit(a0, a1, a2); }
        @Override public java.awt.GraphicsConfiguration getDeviceConfiguration() { return delegate.getDeviceConfiguration(); }
        @Override public void setComposite(java.awt.Composite a0) { delegate.setComposite(a0); }
        @Override public void setPaint(java.awt.Paint a0) { delegate.setPaint(a0); }
        @Override public void setStroke(java.awt.Stroke a0) { delegate.setStroke(a0); }
        @Override public void setRenderingHint(java.awt.RenderingHints.Key a0, java.lang.Object a1) { delegate.setRenderingHint(a0, a1); }
        @Override public java.lang.Object getRenderingHint(java.awt.RenderingHints.Key a0) { return delegate.getRenderingHint(a0); }
        @Override public void setRenderingHints(java.util.Map<?, ?> a0) { delegate.setRenderingHints(a0); }
        @Override public void addRenderingHints(java.util.Map<?, ?> a0) { delegate.addRenderingHints(a0); }
        @Override public java.awt.RenderingHints getRenderingHints() { return delegate.getRenderingHints(); }
        @Override public void translate(int a0, int a1) { delegate.translate(a0, a1); }
        @Override public void translate(double a0, double a1) { delegate.translate(a0, a1); }
        @Override public void rotate(double a0) { delegate.rotate(a0); }
        @Override public void rotate(double a0, double a1, double a2) { delegate.rotate(a0, a1, a2); }
        @Override public void scale(double a0, double a1) { delegate.scale(a0, a1); }
        @Override public void shear(double a0, double a1) { delegate.shear(a0, a1); }
        @Override public void transform(java.awt.geom.AffineTransform a0) { delegate.transform(a0); }
        @Override public void setTransform(java.awt.geom.AffineTransform a0) { delegate.setTransform(a0); }
        @Override public java.awt.geom.AffineTransform getTransform() { return delegate.getTransform(); }
        @Override public java.awt.Paint getPaint() { return delegate.getPaint(); }
        @Override public java.awt.Composite getComposite() { return delegate.getComposite(); }
        @Override public void setBackground(java.awt.Color a0) { delegate.setBackground(a0); }
        @Override public java.awt.Color getBackground() { return delegate.getBackground(); }
        @Override public java.awt.Stroke getStroke() { return delegate.getStroke(); }
        @Override public void clip(java.awt.Shape a0) { delegate.clip(a0); }
        @Override public java.awt.font.FontRenderContext getFontRenderContext() { return delegate.getFontRenderContext(); }
        @Override public java.awt.Graphics create() { return delegate.create(); }
        @Override public java.awt.Color getColor() { return delegate.getColor(); }
        @Override public void setColor(java.awt.Color a0) { delegate.setColor(a0); }
        @Override public void setPaintMode() { delegate.setPaintMode(); }
        @Override public void setXORMode(java.awt.Color a0) { delegate.setXORMode(a0); }
        @Override public java.awt.Font getFont() { return delegate.getFont(); }
        @Override public void setFont(java.awt.Font a0) { delegate.setFont(a0); }
        @Override public java.awt.FontMetrics getFontMetrics(java.awt.Font a0) { return delegate.getFontMetrics(a0); }
        @Override public java.awt.Rectangle getClipBounds() { return delegate.getClipBounds(); }
        @Override public void clipRect(int a0, int a1, int a2, int a3) { delegate.clipRect(a0, a1, a2, a3); }
        @Override public void setClip(int a0, int a1, int a2, int a3) { delegate.setClip(a0, a1, a2, a3); }
        @Override public java.awt.Shape getClip() { return delegate.getClip(); }
        @Override public void setClip(java.awt.Shape a0) { delegate.setClip(a0); }
        @Override public void copyArea(int a0, int a1, int a2, int a3, int a4, int a5) { delegate.copyArea(a0, a1, a2, a3, a4, a5); }
        @Override public void drawLine(int a0, int a1, int a2, int a3) { lines.add(new int[] {a0, a1, a2, a3, delegate.getColor().getRGB()}); delegate.drawLine(a0, a1, a2, a3); }
        @Override public void fillRect(int a0, int a1, int a2, int a3) { delegate.fillRect(a0, a1, a2, a3); }
        @Override public void clearRect(int a0, int a1, int a2, int a3) { delegate.clearRect(a0, a1, a2, a3); }
        @Override public void drawRoundRect(int a0, int a1, int a2, int a3, int a4, int a5) { delegate.drawRoundRect(a0, a1, a2, a3, a4, a5); }
        @Override public void fillRoundRect(int a0, int a1, int a2, int a3, int a4, int a5) { delegate.fillRoundRect(a0, a1, a2, a3, a4, a5); }
        @Override public void drawOval(int a0, int a1, int a2, int a3) { delegate.drawOval(a0, a1, a2, a3); }
        @Override public void fillOval(int a0, int a1, int a2, int a3) { delegate.fillOval(a0, a1, a2, a3); }
        @Override public void drawArc(int a0, int a1, int a2, int a3, int a4, int a5) { delegate.drawArc(a0, a1, a2, a3, a4, a5); }
        @Override public void fillArc(int a0, int a1, int a2, int a3, int a4, int a5) { delegate.fillArc(a0, a1, a2, a3, a4, a5); }
        @Override public void drawPolyline(int[] a0, int[] a1, int a2) { delegate.drawPolyline(a0, a1, a2); }
        @Override public void drawPolygon(int[] a0, int[] a1, int a2) { delegate.drawPolygon(a0, a1, a2); }
        @Override public void fillPolygon(int[] a0, int[] a1, int a2) { delegate.fillPolygon(a0, a1, a2); }
        @Override public boolean drawImage(java.awt.Image a0, int a1, int a2, java.awt.image.ImageObserver a3) { return delegate.drawImage(a0, a1, a2, a3); }
        @Override public boolean drawImage(java.awt.Image a0, int a1, int a2, int a3, int a4, java.awt.image.ImageObserver a5) { return delegate.drawImage(a0, a1, a2, a3, a4, a5); }
        @Override public boolean drawImage(java.awt.Image a0, int a1, int a2, java.awt.Color a3, java.awt.image.ImageObserver a4) { return delegate.drawImage(a0, a1, a2, a3, a4); }
        @Override public boolean drawImage(java.awt.Image a0, int a1, int a2, int a3, int a4, java.awt.Color a5, java.awt.image.ImageObserver a6) { return delegate.drawImage(a0, a1, a2, a3, a4, a5, a6); }
        @Override public boolean drawImage(java.awt.Image a0, int a1, int a2, int a3, int a4, int a5, int a6, int a7, int a8, java.awt.image.ImageObserver a9) { return delegate.drawImage(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9); }
        @Override public boolean drawImage(java.awt.Image a0, int a1, int a2, int a3, int a4, int a5, int a6, int a7, int a8, java.awt.Color a9, java.awt.image.ImageObserver a10) { return delegate.drawImage(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10); }
        @Override public void dispose() { delegate.dispose(); }
    }
}
