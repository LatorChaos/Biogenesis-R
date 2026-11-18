// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.*;
import java.awt.geom.*;
import java.awt.image.*;
import java.lang.reflect.*;
import java.util.*;
import java.util.List;

/** Observe production geometry without advancing organism frames */
public final class GeometryOracle {
    static String bits(double value) { return Long.toUnsignedString(Double.doubleToRawLongBits(value), 16); }
    static void out(Object... fields) {
        StringBuilder line = new StringBuilder();
        for (Object value : fields) { if (line.length() > 0) line.append('\t'); line.append(value); }
        System.out.println(line);
    }
    static Gene gene(int index, double length, double theta, int branch, int argb) throws Exception {
        Constructor<?> constructor = Arrays.stream(Gene.class.getConstructors()).filter(c -> c.getParameterCount() == 40).findFirst().orElseThrow();
        Object[] arguments = new Object[40];
        arguments[0] = length; arguments[1] = theta; arguments[2] = new Color(argb, true); arguments[3] = branch;
        for (int reaction = 0; reaction < 36; reaction++) arguments[reaction + 4] = (reaction + index) % 6;
        return (Gene)constructor.newInstance(arguments);
    }
    static List<Gene> genes(int pattern) throws Exception {
        List<Gene> result = new ArrayList<>();
        if (pattern == 0) result.add(gene(0, 3.25, 0.1, 999, 0xffff0000));
        if (pattern == 1) {
            double[] lengths = {2.5, 3.75, 4.5, 1.25, 9.125};
            double[] angles = {-0.8, 1.2, 2.75, -5.4, 0.75};
            int[] branches = {-1, -1, 0, 1, 2};
            for (int i = 0; i < lengths.length; i++) result.add(gene(i, lengths[i], angles[i], branches[i], 0xff008000 + i));
        }
        if (pattern == 2 || pattern == 6) {
            int[] colors = {0xff004040, 0xff004040, 0xffff0000, 0xff004040, 0xff0000ff, 0xff004040};
            int[] branches = {-1, -1, 1, 0, 4, 2};
            for (int i = 0; i < colors.length; i++) result.add(gene(i, 2.0 + i * 1.125, -0.75 + i * 0.4, branches[i], colors[i]));
        }
        if (pattern == 3) {
            double[] lengths = {-3.5, 0.0, 8.25, 1000000000.0};
            for (int i = 0; i < lengths.length; i++) result.add(gene(i, lengths[i], -7.0 + i * 2.35, i == 0 ? -1 : i, 0xff807060 + i));
        }
        if (pattern == 4) {
            for (int i = 0; i < 129; i++) result.add(gene(i, 2.0 + (i % 7) * 0.25, (i % 5 - 2) * 0.31,
                i == 0 ? -1 : (i % 4 == 0 ? 0 : (i % 4 == 1 ? -1 : (i % 4 == 2 ? 1 : i))),
                i % 3 == 1 ? 0xff004040 : 0xff001100 + i));
        }
        if (pattern == 5) {
            result.add(gene(0, 13.0, 0.4, -1, 0x80004040));
            result.add(gene(1, 5.5, -0.9, -1, 0x12345678));
            result.add(gene(2, 7.0, 1.1, 0, 0xff004040));
        }
        return result;
    }
    static final String[] REACTIONS = {"red", "green", "blue", "plague", "scourge", "white", "gray", "silver", "default", "consumer", "plant", "magenta", "pink", "coral", "orange", "bark", "violet", "virus", "maroon", "olive", "mint", "cream", "spike", "fallow", "lightblue", "ochre", "sky", "lilac", "fire", "lightbrown", "greenbrown", "brown", "ice", "broken", "sick", "friend"};
    static void math(double y, double x) {
        out("A", bits(y), bits(x), bits(Math.atan(y)), bits(Math.atan2(y, x)));
    }
    static void reactionRebuild() throws Exception {
        List<Gene> genes = genes(1);
        GeneticCode code = new GeneticCode(); code._genes = genes.toArray(new Gene[0]); code._symmetry = 3; code._mirror = 1;
        Organism organism = new Organism(new World(null), code);
        organism.create(); organism._growthRatio = 16; organism.symmetric();
        out("C", genes.size(), code._symmetry, code._mirror, 16, 1);
        for (Gene gene : genes) {
            StringBuilder line = new StringBuilder("G\t" + bits(gene.getLength()) + "\t" + bits(gene.getTheta()) + "\t" + Integer.toUnsignedString(gene.getColor().getRGB()) + "\t" + gene.getBranch());
            for (String reaction : REACTIONS) line.append('\t').append(Gene.class.getMethod("get" + reaction + "Reaction").invoke(gene));
            System.out.println(line);
        }
        for (int segment = 0; segment < organism._segments; segment++) {
            organism._segColor[segment] = new Color(0x55000000 | ((segment + 1) * 0x1234), true);
            StringBuilder line = new StringBuilder("P\t" + Integer.toUnsignedString(organism._segColor[segment].getRGB()));
            for (int channel = 0; channel < REACTIONS.length; channel++) {
                Field field = Organism.class.getDeclaredField("_seg" + REACTIONS[channel] + "Reaction"); field.setAccessible(true);
                int value = 1000 + segment * 100 + channel;
                ((int[])field.get(organism))[segment] = value;
                line.append('\t').append(value);
            }
            System.out.println(line);
        }
        organism._growthRatio = 1;
        organism.symmetric();
        out("B", organism._sizeRect.x, organism._sizeRect.y, organism._sizeRect.width, organism._sizeRect.height, bits(organism._mass), bits(organism._I), organism._updateEffects);
        for (int segment = 0; segment < organism._segments; segment++) {
            StringBuilder line = new StringBuilder("S\t" + organism._startPointX[segment] + "\t" + organism._startPointY[segment] + "\t" + organism._endPointX[segment] + "\t" + organism._endPointY[segment] + "\t" + Integer.toUnsignedString(organism._segColor[segment].getRGB()) + "\t" + bits(organism._m[segment]) + "\t" + bits(organism._m1[segment]) + "\t" + bits(organism._m2[segment]));
            for (String reaction : REACTIONS) {
                Field field = Organism.class.getDeclaredField("_seg" + reaction + "Reaction"); field.setAccessible(true);
                line.append('\t').append(((int[])field.get(organism))[segment]);
            }
            System.out.println(line);
        }
    }
    public static void main(String[] args) throws Exception {
        if (args.length == 1 && args[0].equals("--reaction-rebuild")) { reactionRebuild(); return; }
        if (args.length != 0) throw new IllegalArgumentException("No arguments expected");
        double[] extremes = {Double.NEGATIVE_INFINITY, -Double.MAX_VALUE, -1.0, -0.5,
            -Double.MIN_NORMAL, -Double.MIN_VALUE, -0.0, 0.0, Double.MIN_VALUE, Double.MIN_NORMAL,
            0.5, 1.0, 2.0, Double.MAX_VALUE, Double.POSITIVE_INFINITY, Double.NaN};
        for (double y : extremes) for (double x : extremes) math(y, x);
        for (int y = -24; y <= 24; y++) for (int x = -24; x <= 24; x++) math(y, x);
        Random mathRandom = new Random(7654321);
        for (int i = 0; i < 512; i++) math(Double.longBitsToDouble(mathRandom.nextLong()), Double.longBitsToDouble(mathRandom.nextLong()));
        for (double value : new double[] {Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY,
                1e30, -1e30, 2147483648.5, -2147483649.5, Double.MAX_VALUE, -Double.MAX_VALUE, Math.nextDown(0.5)}) {
            out("Q", bits(value), (int)Math.round(value));
        }
        int caseId = 0;
        for (double modulus : new double[] {-3.5, 0.0, 2.75}) for (double theta : new double[] {-7.0, 0.0, 13.25}) {
            Vector2D vector = new Vector2D();
            vector.setPolar(modulus, theta); vector.invertX(); vector.addDegree(0.375); vector.invertY();
            out("V", bits(modulus), bits(theta), bits(vector.getModulus()), bits(vector.getTheta()), bits(vector.getX()), bits(vector.getY()));
            vector.setPolar(modulus, theta); vector.invertY(); vector.addDegree(0.375); vector.invertX();
            out("U", bits(modulus), bits(theta), bits(vector.getModulus()), bits(vector.getTheta()), bits(vector.getX()), bits(vector.getY()));
        }
        for (int pattern = 0; pattern < 7; pattern++) for (int symmetry = 1; symmetry <= 8; symmetry++) for (int mirror = 0; mirror <= 1; mirror++) for (int growth = 1; growth <= 16; growth++) {
            if (pattern >= 3 && growth != 1 && growth != 16 && !(pattern == 3 && growth == 8)) continue;
            List<Gene> genes = genes(pattern);
            GeneticCode code = new GeneticCode(); code._genes = genes.toArray(new Gene[0]); code._symmetry = symmetry; code._mirror = mirror;
            Organism organism = new Organism(new World(null), code);
            organism.create();
            if (pattern == 6) for (int i = 0; i < organism._segments; i++) {
                if (i % genes.size() == 0) organism._segColor[i] = Utils.ColorVISION;
                if (i % genes.size() == 1) organism._segColor[i] = Color.WHITE;
                if (i % genes.size() == 4) organism._segColor[i] = Utils.ColorEYE;
            }
            boolean enabled = organism._haseyes;
            boolean good = pattern == 6 || (pattern == 2 && growth > 8);
            if (pattern == 2 && growth == 16) enabled = false;
            int flags = (growth - 1) % 8;
            organism._haseyes = enabled; organism._hasgoodvision = good;
            organism._isaconsumer = (flags & 1) != 0; organism._isafungus = (flags & 2) != 0; organism._isinfectious = (flags & 4) != 0;
            organism._growthRatio = growth; organism._updateEffects = growth % 3 == 0 ? -2 : 0;
            int beforeUpdate = organism._updateEffects;
            double angle = new double[] {0.0, Math.PI / 2, Math.PI, -Math.PI, Math.PI * 10 + 0.7}[growth % 5];
            double cx = growth % 2 == 0 ? -14.75 : 25.75, cy = growth % 2 == 0 ? -9.125 : -0.5;
            int viewportWidth = growth % 2 == 0 ? 65 : 97, viewportHeight = growth % 2 == 0 ? 47 : 53;
            out("C", caseId++, symmetry, mirror, growth, enabled ? 1 : 0, good ? 1 : 0, flags, beforeUpdate, bits(angle), bits(cx), bits(cy), viewportWidth, viewportHeight, genes.size());
            for (Gene g : genes) {
                StringBuilder line = new StringBuilder("G\t" + bits(g.getLength()) + "\t" + bits(g.getTheta()) + "\t" + Integer.toUnsignedString(g.getColor().getRGB()) + "\t" + g.getBranch());
                for (String reaction : REACTIONS) line.append('\t').append(Gene.class.getMethod("get" + reaction + "Reaction").invoke(g));
                System.out.println(line);
            }
            StringBuilder phenotype = new StringBuilder("F");
            for (Color color : organism._segColor) phenotype.append('\t').append(Integer.toUnsignedString(color.getRGB()));
            System.out.println(phenotype);
            organism.symmetric();
            out("B", organism._sizeRect.x, organism._sizeRect.y, organism._sizeRect.width, organism._sizeRect.height, bits(organism._mass), bits(organism._I), organism._haseyes ? 1 : 0, organism._hasgoodvision ? 1 : 0, organism._updateEffects);
            organism._theta = angle; organism._dCenterX = cx; organism._dCenterY = cy; organism._centerX = (int)cx; organism._centerY = (int)cy;
            organism.calculateBounds(true);
            out("W", organism.x, organism.y, organism.width, organism.height, organism._centerX, organism._centerY);
            TraceGraphics graphics = new TraceGraphics(new BufferedImage(viewportWidth, viewportHeight, BufferedImage.TYPE_INT_ARGB).createGraphics());
            code.draw(graphics, viewportWidth, viewportHeight);
            AffineTransform transform = graphics.getTransform();
            out("T", bits(transform.getTranslateX()), bits(transform.getTranslateY()), bits(transform.getScaleX()));
            if (graphics.lines.size() != organism._segments) throw new AssertionError("Preview segment count");
            for (int i = 0; i < organism._segments; i++) {
                int[] preview = graphics.lines.get(i);
                StringBuilder line = new StringBuilder("S\t" + organism._startPointX[i] + "\t" + organism._startPointY[i] + "\t" + organism._endPointX[i] + "\t" + organism._endPointY[i] + "\t" + Integer.toUnsignedString(organism._segColor[i].getRGB()) + "\t" + bits(organism._m[i]) + "\t" + bits(organism._m1[i]) + "\t" + bits(organism._m2[i]) + "\t" + organism.x1[i] + "\t" + organism.y1[i] + "\t" + organism.x2[i] + "\t" + organism.y2[i]);
                for (int coordinate : preview) line.append('\t').append(coordinate);
                for (String reaction : REACTIONS) {
                    Field field = Organism.class.getDeclaredField("_seg" + reaction + "Reaction"); field.setAccessible(true);
                    line.append('\t').append(((int[])field.get(organism))[i]);
                }
                System.out.println(line);
            }
            graphics.dispose();
            out("E");
        }
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
