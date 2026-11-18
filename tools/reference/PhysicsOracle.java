package biogenesis;

import java.awt.Rectangle;
import java.awt.geom.Line2D;
import java.awt.geom.Point2D;
import java.io.BufferedWriter;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Random;

/** Observe original geometry APIs and collision impulses without running contacts or frames */
public final class PhysicsOracle {
    private static final String[] PHYSICAL_FIELDS = {"_mass", "_I", "_dCenterX", "_dCenterY", "dx", "dy", "dtheta", "hasGrown", "_growthRatio"};
    private static BufferedWriter output;
    private static Method impulse;
    private static int cases;

    private static final class Motion {
        double mass = 2, inertia = 4, x = -1, y, dx = 1, dy, rotation;
        int grown, growth = 16;
        Motion other() { mass = 3; inertia = 5; x = 1; dx = -1; return this; }
    }
    private static final class Spec {
        final String name;
        Motion a = new Motion(), b = new Motion().other();
        double[] line = {0, -2, 0, 2}, point = {0, 0};
        double elasticity = 0.8, maximumVelocity = 5, maximumRotation = Math.PI / 16;
        boolean thisOrganism;
        Spec(String name, boolean thisOrganism) { this.name = name; this.thisOrganism = thisOrganism; }
    }

    private static String bits(double value) { return String.format(java.util.Locale.ROOT, "%016x", Double.doubleToRawLongBits(value)); }
    private static void require(boolean condition, String message) { if (!condition) throw new IllegalStateException(message); }
    private static void row(Object... values) throws Exception {
        for (int i = 0; i < values.length; i++) { if (i != 0) output.write('\t'); output.write(values[i].toString()); }
        output.write('\n');
    }
    private static ExLine2DDouble line(double[] values) {
        ExLine2DDouble line = new ExLine2DDouble();
        line.setLine(values[0], values[1], values[2], values[3]);
        return line;
    }
    private static void lineRow(String label, Line2D line) throws Exception {
        row("L", label, bits(line.getX1()), bits(line.getY1()), bits(line.getX2()), bits(line.getY2()));
    }
    private static void rectangleRow(String label, Rectangle rectangle) throws Exception {
        row("Q", label, rectangle.x, rectangle.y, rectangle.width, rectangle.height);
    }
    private static void geometry(ExLine2DDouble a, ExLine2DDouble b, Rectangle first, Rectangle second) throws Exception {
        row("U", "tol", bits(Utils.tol));
        lineRow("a", a); lineRow("b", b);
        rectangleRow("a", first); rectangleRow("b", second);
        Point2D.Double intersection = a.getIntersection(b);
        row("P", "intersection", bits(intersection.x), bits(intersection.y));
        row("B", "lines_intersect", a.intersectsLine(b));
        row("B", "a_intersects_rect_a", first.intersectsLine(a));
        row("B", "b_intersects_rect_a", first.intersectsLine(b));
        row("B", "rectangles_intersect", first.intersects(second));
        row("D", "a.start", bits(intersection.distanceSq(a.getP1())));
        row("D", "a.end", bits(intersection.distanceSq(a.getP2())));
        row("D", "b.start", bits(intersection.distanceSq(b.getP1())));
        row("D", "b.end", bits(intersection.distanceSq(b.getP2())));
        row("E");
        cases++;
    }
    private static void geometry(String name, double[] a, double[] b, Rectangle first, Rectangle second) throws Exception {
        row("C", name, "geometry");
        geometry(line(a), line(b), first, second);
    }
    private static ExLine2DDouble offsetLine(String label, int[] values) throws Exception {
        row("X", label, values[0], values[1], values[2], values[3], values[4], values[5]);
        ExLine2DDouble result = new ExLine2DDouble();
        result.setLine(values[0] + values[4], values[1] + values[5], values[2] + values[4], values[3] + values[5]);
        return result;
    }
    private static void offsets(String name, int[] a, int[] b, Rectangle first, Rectangle second) throws Exception {
        row("C", name, "offsets");
        ExLine2DDouble left = offsetLine("a", a), right = offsetLine("b", b);
        geometry(left, right, first, second);
    }
    private static void input(String owner, Organism organism, Motion values) throws Exception {
        Object[] supplied = {values.mass, values.inertia, values.x, values.y, values.dx, values.dy, values.rotation, values.grown, values.growth};
        for (int i = 0; i < PHYSICAL_FIELDS.length; i++) {
            Field field = Organism.class.getDeclaredField(PHYSICAL_FIELDS[i]); field.setAccessible(true);
            field.set(organism, supplied[i]);
            row("I", owner, field.getName(), field.getType().getSimpleName(), supplied[i] instanceof Double value ? bits(value) : supplied[i]);
        }
    }
    private static void state(String stage, String owner, Organism organism) throws Exception {
        for (String name : PHYSICAL_FIELDS) {
            Field field = Organism.class.getDeclaredField(name); field.setAccessible(true);
            Object value = field.get(organism);
            row("O", stage, owner, name, field.getType().getSimpleName(), value instanceof Double number ? bits(number) : value);
        }
    }
    private static void motion(Spec spec) throws Exception {
        long seed = 506172;
        Utils.random = new Random(seed);
        Utils.ELASTICITY = spec.elasticity;
        Utils.MAX_VEL = spec.maximumVelocity;
        Utils.MAX_ROT = spec.maximumRotation;
        row("C", spec.name, "impulse");
        row("U", "ELASTICITY", bits(Utils.ELASTICITY));
        row("U", "MAX_VEL", bits(Utils.MAX_VEL));
        row("U", "MAX_ROT", bits(Utils.MAX_ROT));
        row("I", "call", "constructor_seed", "long", seed);
        row("I", "call", "thisOrganism", "boolean", spec.thisOrganism);
        World world = new World(null);
        Organism a = new Organism(world), b = new Organism(world);
        input("a", a, spec.a); input("b", b, spec.b);
        ExLine2DDouble impact = line(spec.line);
        Point2D.Double contact = new Point2D.Double(spec.point[0], spec.point[1]);
        lineRow("impact", impact);
        row("P", "contact", bits(contact.x), bits(contact.y));
        state("before", "a", a); state("before", "b", b);
        impulse.invoke(a, b, contact, impact, spec.thisOrganism);
        state("after", "a", a); state("after", "b", b);
        long nextLong = Utils.random.nextLong(); int nextInt = Utils.random.nextInt(10000); double nextDouble = Utils.random.nextDouble();
        Random plain = new Random(seed); plain.nextDouble(); plain.nextDouble();
        require(nextLong == plain.nextLong() && nextInt == plain.nextInt(10000) && bits(nextDouble).equals(bits(plain.nextDouble())), "Impulse consumed unexpected RNG");
        row("R", nextLong, nextInt, bits(nextDouble));
        row("E");
        cases++;
    }
    private static void geometries() throws Exception {
        Rectangle ordinary = new Rectangle(-10, -10, 20, 20), small = new Rectangle(0, 0, 2, 2);
        geometry("asymmetric-positive-determinant", new double[]{0, 0, 8, 2}, new double[]{2, -3, 3, 6}, ordinary, small);
        geometry("asymmetric-negative-determinant", new double[]{0, 0, 8, 2}, new double[]{3, 6, 2, -3}, ordinary, small);
        geometry("parallel-offset", new double[]{0, 0, 8, 0}, new double[]{1, 3, 4, 3}, ordinary, small);
        geometry("collinear-disjoint", new double[]{0, 0, 2, 0}, new double[]{3, 0, 4, 0}, ordinary, small);
        geometry("collinear-overlap", new double[]{0, 0, 4, 0}, new double[]{2, 0, 6, 0}, ordinary, small);
        geometry("collinear-shared-endpoint", new double[]{0, 0, 4, 0}, new double[]{4, 0, 6, 0}, ordinary, small);
        geometry("crossing-endpoint", new double[]{0, 0, 4, 0}, new double[]{4, 0, 4, 5}, ordinary, small);
        geometry("all-endpoint-distances-tie", new double[]{-1, 0, 1, 0}, new double[]{0, -1, 0, 1}, ordinary, small);
        geometry("one-line-nearer-endpoints", new double[]{-2, 0, 2, 0}, new double[]{0, -1, 0, 1}, ordinary, small);
        geometry("point-on-line", new double[]{2, 0, 2, 0}, new double[]{0, 0, 4, 0}, ordinary, small);
        geometry("point-off-line", new double[]{2, 1, 2, 1}, new double[]{0, 0, 4, 0}, ordinary, small);
        geometry("equal-points", new double[]{1, 2, 1, 2}, new double[]{1, 2, 1, 2}, ordinary, small);
        geometry("different-degenerate-points", new double[]{1, 2, 1, 2}, new double[]{9, 7, 9, 7}, ordinary, small);
        for (double determinant : new double[]{-2e-7, -1e-7, -0.5e-7, 0, 0.5e-7, 1e-7, 2e-7}) {
            geometry("determinant-" + bits(determinant), new double[]{0, 0, 1, 0}, new double[]{0, 0, 0, determinant}, ordinary, small);
        }
        double[] across = {-20, 5, 20, 5}, along = {0, 0, 10, 0};
        int[][] rectangles = {{0,0,10,10},{0,0,0,10},{0,0,10,0},{0,0,0,0},{0,0,-1,10},{0,0,10,-1},
                {0,0,-1,-1},{0,0,10,10},{10,0,10,10},{11,0,10,10},
                {Integer.MAX_VALUE-2,0,8,10},{Integer.MIN_VALUE,0,Integer.MAX_VALUE,10},
                {Integer.MAX_VALUE,Integer.MIN_VALUE,Integer.MAX_VALUE,Integer.MAX_VALUE}};
        for (int i = 0; i < rectangles.length; i++) {
            int[] r = rectangles[i];
            geometry("rectangle-boundary-" + i, across, along, new Rectangle(r[0],r[1],r[2],r[3]), new Rectangle(10,0,5,5));
        }
        geometry("rectangle-corner-touch", new double[]{-1,-1,0,0}, new double[]{10,10,11,11}, new Rectangle(0,0,10,10), new Rectangle(10,10,1,1));
        geometry("rectangle-internal-point", new double[]{5,5,5,5}, new double[]{11,11,11,11}, new Rectangle(0,0,10,10), new Rectangle(2,2,1,1));
        double edge = Integer.MAX_VALUE;
        geometry("rectangle-overflow-x-overlap", new double[]{edge-1,-2,edge-1,2}, new double[]{edge-3,0,edge,0},
                new Rectangle(Integer.MAX_VALUE-2,-10,8,20), new Rectangle(Integer.MAX_VALUE-1,-1,1,2));
        geometry("rectangle-overflow-xy-overlap", new double[]{edge-1,edge-1,edge,edge}, new double[]{edge-1,edge-3,edge-1,edge},
                new Rectangle(Integer.MAX_VALUE-2,Integer.MAX_VALUE-2,8,8), new Rectangle(Integer.MAX_VALUE-1,Integer.MAX_VALUE-1,1,1));
        offsets("offsets-ordinary", new int[]{-2,-1,3,4,10,20}, new int[]{-1,2,4,-3,10,20}, ordinary, small);
        offsets("offsets-positive-wrap", new int[]{10,0,20,10,Integer.MAX_VALUE-5,0}, new int[]{0,-2,0,20,Integer.MIN_VALUE+10,0}, new Rectangle(Integer.MIN_VALUE,0,30,20), small);
        offsets("offsets-negative-wrap", new int[]{-10,-20,-1,-2,Integer.MIN_VALUE+3,Integer.MIN_VALUE+5}, new int[]{0,0,8,8,Integer.MAX_VALUE-5,Integer.MAX_VALUE-5}, ordinary, small);
        offsets("offsets-both-extremes", new int[]{Integer.MAX_VALUE,Integer.MIN_VALUE,Integer.MIN_VALUE,Integer.MAX_VALUE,1,-1}, new int[]{Integer.MIN_VALUE,Integer.MAX_VALUE,Integer.MAX_VALUE,Integer.MIN_VALUE,-1,1}, ordinary, small);
        geometry("nonfinite-infinite-line", new double[]{Double.NEGATIVE_INFINITY,0,Double.POSITIVE_INFINITY,0}, new double[]{0,-1,0,1}, ordinary, small);
        geometry("nonfinite-nan-line", new double[]{Double.NaN,0,1,1}, new double[]{0,-1,0,1}, ordinary, small);
    }
    private static void motions() throws Exception {
        for (boolean flag : new boolean[]{false,true}) {
            String suffix = "-" + flag;
            motion(new Spec("approaching" + suffix, flag));
            Spec separating = new Spec("separating" + suffix, flag); separating.a.dx=-1; separating.b.dx=1; motion(separating);
            Spec still = new Spec("stationary" + suffix, flag); still.a.dx=0; still.b.dx=0; motion(still);
            Spec reversed = new Spec("reversed-impact-line" + suffix, flag); reversed.line=new double[]{0,2,0,-2}; motion(reversed);
            Spec horizontal = new Spec("horizontal-impact" + suffix, flag); horizontal.line=new double[]{-2,0,2,0};
            horizontal.a.x=0;horizontal.a.y=-1;horizontal.b.x=0;horizontal.b.y=1;horizontal.a.dx=0;horizontal.a.dy=1;horizontal.b.dx=0;horizontal.b.dy=-1;motion(horizontal);
            Spec skew = new Spec("skew-rotation" + suffix, flag); skew.line=new double[]{1,-2,4,5};skew.point=new double[]{1.5,0.75};
            skew.a.x=-2.25;skew.a.y=1.5;skew.b.x=2.75;skew.b.y=-3;skew.a.dx=1.125;skew.a.dy=-0.875;skew.a.rotation=0.375;skew.b.dx=-0.625;skew.b.dy=0.25;skew.b.rotation=-0.1875;motion(skew);
            Spec tie = new Spec("normal-orientation-tie" + suffix, flag);tie.a.x=0;tie.b.x=0;motion(tie);
            Spec side = new Spec("both-centers-same-side" + suffix, flag);side.b.x=-2;motion(side);
            Spec zero = new Spec("zero-normal" + suffix, flag);zero.line=new double[]{1,2,1,2};zero.point=new double[]{1,2};motion(zero);
            for (int[] growth : new int[][]{{1,0},{0,1},{1,1},{-1,0},{0,-1}}) {
                Spec spec=new Spec("growth-"+growth[0]+"-"+growth[1]+suffix,flag);spec.point=new double[]{0.5,1.25};spec.a.grown=growth[0];spec.b.grown=growth[1];motion(spec);
            }
        }
        Spec asymmetric = new Spec("asymmetric-physical-values",true);asymmetric.a.mass=0.25;asymmetric.a.inertia=0.125;asymmetric.b.mass=123;asymmetric.b.inertia=65536;
        asymmetric.point=new double[]{0.25,0.75};asymmetric.line=new double[]{-1,-2,3,5};motion(asymmetric);
        for (int mode=0;mode<3;mode++) {
            Spec clamp=new Spec("clamping-"+mode,false);clamp.a.dx=100;clamp.a.dy=-100;clamp.b.dx=-100;clamp.b.dy=100;clamp.a.rotation=64;clamp.b.rotation=-64;
            clamp.point=new double[]{0,1};
            if(mode==1){clamp.maximumVelocity=0.25;clamp.maximumRotation=0.125;}
            if(mode==2){clamp.maximumVelocity=0;clamp.maximumRotation=0;}
            motion(clamp);
        }
        for (double elasticity:new double[]{0,1}) {Spec spec=new Spec("elasticity-"+(int)elasticity,true);spec.elasticity=elasticity;motion(spec);}
        Spec signedZero=new Spec("signed-zero-velocities",false);signedZero.a.dx=-0d;signedZero.a.dy=-0d;signedZero.a.rotation=-0d;signedZero.b.dx=0;signedZero.b.dy=-0d;motion(signedZero);
        for (String target:new String[]{"a-mass","b-mass","both-mass","a-inertia-zero-lever","a-inertia-nonzero-lever","both-inertia","mass-and-inertia","negative-zero-mass"}) {
            Spec spec=new Spec("degenerate-"+target,false);
            switch(target){
                case "a-mass":spec.a.mass=0;break;
                case "b-mass":spec.b.mass=0;break;
                case "both-mass":spec.a.mass=0;spec.b.mass=0;break;
                case "a-inertia-zero-lever":spec.a.inertia=0;break;
                case "a-inertia-nonzero-lever":spec.a.inertia=0;spec.point=new double[]{0,1};break;
                case "both-inertia":spec.a.inertia=0;spec.b.inertia=0;spec.point=new double[]{0,1};break;
                case "mass-and-inertia":spec.a.mass=0;spec.a.inertia=0;spec.point=new double[]{0,1};break;
                default:spec.a.mass=-0d;break;
            }
            motion(spec);
        }
        Spec tiny=new Spec("subnormal-mass",false);tiny.a.mass=1e-320;tiny.b.mass=2e-320;motion(tiny);
        Spec infinite=new Spec("controlled-infinite-mass-inertia",false);infinite.a.mass=Double.POSITIVE_INFINITY;infinite.b.mass=Double.POSITIVE_INFINITY;infinite.a.inertia=Double.POSITIVE_INFINITY;infinite.b.inertia=Double.POSITIVE_INFINITY;motion(infinite);
        Spec velocity=new Spec("controlled-infinite-velocity",false);velocity.a.dx=Double.POSITIVE_INFINITY;velocity.a.dy=Double.NEGATIVE_INFINITY;motion(velocity);
        Spec nan=new Spec("controlled-nan-mass",false);nan.a.mass=Double.NaN;motion(nan);
        Spec unbounded=new Spec("controlled-zero-denominator-infinite-caps",false);unbounded.a.mass=1;unbounded.b.mass=1;unbounded.a.inertia=-1;unbounded.b.inertia=-1;
        unbounded.point=new double[]{0,1};unbounded.maximumVelocity=Double.POSITIVE_INFINITY;unbounded.maximumRotation=Double.POSITIVE_INFINITY;motion(unbounded);
    }
    public static void main(String[] args) throws Exception {
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.PhysicsOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        impulse=Organism.class.getDeclaredMethod("touchMove",Organism.class,Point2D.Double.class,Line2D.class,boolean.class);impulse.setAccessible(true);
        try(BufferedWriter writer=Files.newBufferedWriter(directory.resolve("physics.tsv"),StandardCharsets.UTF_8)){
            output=writer;geometries();motions();
        }
        System.out.println("PHYSICS_ORACLE PASS cases="+cases);
    }
}
