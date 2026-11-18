package biogenesis;

import java.awt.Color;
import java.awt.Rectangle;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.lang.reflect.Array;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
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
import org.locationtech.jts.geom.Envelope;

/** Observe original movement primitives with real bodies and explicit state inputs */
public final class MovementOracle {
    private static final String[] COLORS = ("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static Method grow, rubbing, offset;
    private static int cases;

    private static final class Action {
        final String operation;
        final Object[] args;
        final Map<String,Object> controls = new LinkedHashMap<>();
        String energyRule;
        Action(String operation, Object... args) { this.operation = operation; this.args = args; }
        Action set(String name, Object value) { controls.put(name, value); return this; }
        Action energy(String value) { energyRule = value; return this; }
    }
    private static final class Spec {
        final String name;
        String setup = "pasted";
        String[] colors = {"GREEN","BLUE","CYAN"};
        int symmetry = 3, mirror = 1, spore = 1;
        Double length;
        double friction = 0.98;
        long seed = 827364;
        final List<Action> actions = new ArrayList<>();
        Spec(String name) { this.name = name; }
        Spec add(Action action) { actions.add(action); return this; }
    }
    private static final class Capture {
        final Spec spec;
        final StringBuilder rows = new StringBuilder();
        final IdentityHashMap<GeneticCode,String> references = new IdentityHashMap<>();
        World world;
        Organism focus;
        Capture(Spec spec) { this.spec = spec; }
        void row(Object... values) {
            for(int i=0;i<values.length;i++){if(i!=0)rows.append('\t');rows.append(values[i]);}rows.append('\n');
        }
        String reference(GeneticCode code) {
            if(code==null)return "~";
            return references.computeIfAbsent(code, ignored -> "g"+references.size());
        }
        void genome(String stage,String owner,GeneticCode code)throws Exception {
            row("G",stage,owner,"_reference",reference(code));
            if(code==null)return;
            row("G",stage,owner,"_nGenes",code.getNGenes());
            for(Field field:fields(GeneticCode.class))if(!field.getName().equals("_genes"))row("G",stage,owner,field.getName(),scalar(field.get(code)));
            for(int i=0;i<code.getNGenes();i++)for(Field field:fields(Gene.class))row("N",stage,owner,i,field.getName(),scalar(field.get(code.getGene(i))));
        }
        void snapshot(String stage)throws Exception {
            for(Field field:fields(Organism.class))row("O",stage,field.getName(),field.getType().getSimpleName(),scalar(field.get(focus)));
            row("O",stage,"bounds","Rectangle",scalar(focus.getBounds()));
            genome(stage,"genotype",focus._geneticCode);
            genome(stage,"infected",focus._infectedGeneticCode);
            genome(stage,"saved",focus._savedGeneticCode);
            Random peek=copyRandom(Utils.random);
            row("R",stage,peek.nextLong(),peek.nextInt(10000),bits(peek.nextDouble()));
        }
        void input(String stage,String name,Object value)throws Exception {
            if(name.equals("gene0._length")){
                focus._geneticCode.getGene(0).setLength((double)value);
                row("I",stage,name,"double",bits((double)value));return;
            }
            Field field=Organism.class.getDeclaredField(name);field.setAccessible(true);
            if(field.getType()==GeneticCode.class){
                String mode=(String)value;
                GeneticCode code=mode.equals("shared")?focus._geneticCode:(GeneticCode)focus._geneticCode.clone();
                require(mode.equals("shared")||mode.equals("clone"),"Unsupported genotype input");
                field.set(focus,code);row("I",stage,name,"GeneticCode",mode);
            }else{
                field.set(focus,value);row("I",stage,name,field.getType().getSimpleName(),scalar(value));
            }
        }
        void action(int index,Action action)throws Exception {
            String stage="step-"+index;
            for(Map.Entry<String,Object> control:action.controls.entrySet())input(stage,control.getKey(),control.getValue());
            if(action.energyRule!=null){
                String[] rule=action.energyRule.split(":");double energy=focus.getMass()/Integer.parseInt(rule[0]);
                if(rule[1].equals("below"))energy=Math.nextDown(energy);
                if(rule[1].equals("above"))energy=Math.nextUp(energy);
                input(stage,"_energy",energy);
            }
            switch(action.operation){
                case "symmetric":row("K",stage,"symmetric");focus.symmetric();break;
                case "bounds":row("K",stage,"bounds",action.args[0]);focus.calculateBounds((boolean)action.args[0]);break;
                case "grow":row("K",stage,"grow");grow.invoke(focus);break;
                case "rubbing":row("K",stage,"rubbing");rubbing.invoke(focus);break;
                case "offset":
                    row("K",stage,"offset",bits((double)action.args[0]),bits((double)action.args[1]),bits((double)action.args[2]));
                    offset.invoke(focus,action.args);break;
                case "offset_velocity":
                    row("K",stage,"offset_velocity");offset.invoke(focus,focus.dx,focus.dy,focus.dtheta);break;
                default:throw new IllegalArgumentException("Unknown action: "+action.operation);
            }
            snapshot(stage);
        }
    }

    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(f->!Modifier.isStatic(f.getModifiers())).peek(f->f.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static String scalar(Object value){
        if(value==null)return "~";
        if(value instanceof Double number)return bits(number);
        if(value instanceof Color color)return String.format(java.util.Locale.ROOT,"%08x",color.getRGB());
        if(value instanceof Rectangle r)return r.x+","+r.y+","+r.width+","+r.height;
        if(value instanceof World||value instanceof VisibleWorld||value instanceof GeneticCode)return "present";
        if(value.getClass().isArray()){
            StringBuilder text=new StringBuilder();for(int i=0;i<Array.getLength(value);i++){if(i!=0)text.append(',');text.append(scalar(Array.get(value,i)));}return text.toString();
        }
        return value.toString();
    }
    private static Random copyRandom(Random random)throws Exception{
        ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream stream=new ObjectOutputStream(bytes)){stream.writeObject(random);}
        try(ObjectInputStream stream=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)stream.readObject();}
    }
    private static Color color(String name)throws Exception{
        switch(name){case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static GeneticCode genome(Spec spec)throws Exception{
        List<Gene> genes=new ArrayList<>();
        for(int i=0;i<spec.colors.length;i++){
            Gene gene=new Gene();gene.setLength(spec.length==null?12+2*i:spec.length);gene.setTheta(i*0.375);gene.setColor(color(spec.colors[i]));gene.setBranch(-1);genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,spec.symmetry,spec.mirror,0,0,1,1,spec.spore,1,
                false,false,false,false,false,false,false,false,false,false,false,false,false,false,false);
        code._generation=41;code._cladeID="root";return code;
    }
    private static Capture prepare(Spec spec)throws Exception{
        Capture capture=new Capture(spec);
        for(Field original:Utils.class.getDeclaredFields())if(original.getName().startsWith("DEF_"))Utils.class.getDeclaredField(original.getName().substring(4)).set(null,original.get(null));
        Utils.MIN_MUTATION_RATE=0;Utils.MAX_MUTATION_RATE=10000;Utils.META_MUTATION_RATE=0;Utils.CLADE_COMPLEXITY=25;Utils.RUBBING=spec.friction;
        for(String name:COLORS)Utils.class.getDeclaredField(name+"_PROB").setInt(null,name.equals("GREEN")?7:0);
        Utils.random=new Random(spec.seed);
        capture.world=new World(null);capture.world.addCO2(Utils.INITIAL_CO2);capture.world.addO2(Utils.INITIAL_O2);capture.world.addCH4(Utils.INITIAL_CH4);
        GeneticCode code=genome(spec);Organism parent=new Organism(capture.world,code);require(parent.pasteOrganism(3500,3500),"Real paste failed");
        capture.world._organisms.add(parent);capture.focus=parent;
        capture.row("C",spec.name,spec.setup,spec.seed);
        for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())capture.row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
        capture.genome("input","genotype",code);
        if(spec.setup.equals("spore_birth")){
            Map<String,Object> parentInputs=new LinkedHashMap<>();
            parentInputs.put("_energy",1000d);parentInputs.put("_reproduceEnergy",101);parentInputs.put("_earlyReproduceEnergy",0);
            parentInputs.put("_nChildren",1);parentInputs.put("_nVirusChildren",0);parentInputs.put("dx",1.25d);parentInputs.put("dy",-0.5d);parentInputs.put("_sporetime",30);
            for(Map.Entry<String,Object> input:parentInputs.entrySet())capture.input("parent-input",input.getKey(),input.getValue());
            capture.world.colDetTree.insert(new Envelope(parent.getX(),parent.getMaxX(),parent.getY(),parent.getMaxY()),parent);capture.world.colDetTree.build();
            capture.row("F","parent_tree_bounds",scalar(parent.getBounds()));
            Utils.random=new Random(spec.seed);capture.row("F","birth_seed_reset",spec.seed);
            Organism child=new Organism(capture.world);require(child.inherit(parent,true),"Actual spore birth failed");capture.world._organisms.add(child);capture.focus=child;
        }
        capture.snapshot("initial");return capture;
    }
    private static Action motion(Action action){return action.set("dx",1.25d).set("dy",-0.75d).set("dtheta",0.125d).set("hasGrown",7);}
    private static Spec ratio(String name,int ratio){
        return new Spec(name).add(new Action("symmetric").set("_growthRatio",ratio)).add(new Action("bounds",true));
    }
    private static Spec spore(String name,int modifier){
        Spec spec=ratio(name,15);spec.symmetry=1;spec.mirror=0;spec.colors=new String[]{"GREEN","SPORE","EYE"};spec.spore=modifier;return spec;
    }
    private static Action starvation(){return new Action("grow").set("_energy",0d).set("_age",8).set("_sporeversion",-1).set("active",true).set("_framesColor",0).set("_isinfectious",false).set("_isblond",false).set("_isfrozen",true).set("_isinjured",true).set("_allfrozen",true);}
    private static List<Spec> specifications(){
        List<Spec> result=new ArrayList<>();
        for(int ratio=1;ratio<=16;ratio++)result.add(ratio("growth-ratio-"+ratio,ratio).add(motion(new Action("grow")).set("_energy",1000d).set("_age",7).set("active",true)).add(new Action("bounds",true)));
        for(int divisor:new int[]{10,12})for(String side:new String[]{"below","equal","above"})result.add(ratio("energy-"+divisor+"-"+side,8).add(motion(new Action("grow")).set("_age",7).set("active",true).energy(divisor+":"+side)));
        for(int age:new int[]{8,15})result.add(ratio("growth-age-"+age,8).add(motion(new Action("grow")).set("_energy",1000d).set("_age",age).set("active",true)));
        for(int ratio:new int[]{15,16})result.add(ratio("inactive-newborn-ratio-"+ratio,ratio).add(motion(new Action("grow")).set("_energy",1000d).set("_age",7).set("active",false)));
        for(int ratio:new int[]{1,14,15,16})result.add(ratio("shrink-ratio-"+ratio,ratio).add(motion(new Action("grow")).set("_energy",0d).set("_age",8).set("active",true)).add(new Action("bounds",true)));
        result.add(ratio("inactive-shrink",14).add(motion(new Action("grow")).set("_energy",0d).set("_age",8).set("active",false)));
        for(int version:new int[]{-1,-2,-3})result.add(ratio("growth-spore-version-"+version,16).add(motion(new Action("grow")).set("_energy",1000d).set("_age",7).set("active",false).set("_sporeversion",version)));
        for(int modifier:new int[]{7,10})for(int frames:new int[]{0,9,10})for(int flags=0;flags<4;flags++){
            Action action=starvation().set("_framesColor",frames).set("_isinfectious",(flags&1)!=0).set("_isblond",(flags&2)!=0);
            result.add(spore("sporify-"+modifier+"-frames-"+frames+"-flags-"+flags,modifier).add(action).add(new Action("bounds",false)).add(new Action("bounds",true)));
        }
        result.add(spore("sporify-inactive",7).add(starvation().set("active",false)));
        for(int version:new int[]{-2,-3})result.add(spore("sporify-wrong-version-"+version,10).add(starvation().set("_sporeversion",version)));
        result.add(spore("sporify-saved-without-infection",7).add(starvation().set("_savedGeneticCode","clone")));
        result.add(spore("sporify-clears-infected-and-saved",10).add(starvation().set("_infectedGeneticCode","clone").set("_savedGeneticCode","clone")));
        result.add(spore("sporify-clears-shared-references",7).add(starvation().set("_infectedGeneticCode","shared").set("_savedGeneticCode","shared")));
        Spec chain=spore("shrink-then-sporify",10);chain.actions.clear();chain.add(new Action("symmetric").set("_growthRatio",14)).add(new Action("bounds",true));
        chain.add(motion(starvation())).add(new Action("grow")).add(new Action("bounds",false)).add(new Action("bounds",true));result.add(chain);

        Spec changed=new Spec("cache-after-gene-change");changed.add(new Action("symmetric").set("gene0._length",27.5d)).add(new Action("bounds",false)).add(new Action("bounds",true));result.add(changed);
        for(boolean negative:new boolean[]{false,true}){
            Spec cache=new Spec("cache-signed-zero-"+negative);cache.symmetry=1;cache.mirror=0;cache.colors=new String[]{"GREEN"};
            Action first=new Action("bounds",false).set("_theta",negative?-0d:0d).set("_lastTheta",negative?0d:-0d)
                    .set("x1",new int[]{-13}).set("y1",new int[]{-17}).set("x2",new int[]{-3}).set("y2",new int[]{-5})
                    .set("_dCenterX",-0.25d).set("_dCenterY",-0.75d).set("_centerX",123).set("_centerY",-456);
            cache.add(first).add(new Action("bounds",true));result.add(cache);
        }
        Spec nanAngle=new Spec("bounds-nan-angle");nanAngle.add(new Action("bounds",false).set("_theta",Double.NaN).set("_lastTheta",Double.NaN));result.add(nanAngle);
        double[][] offsets={{0.25,-0.75,0},{-3500.75,-3500.25,0},{-3501.25,-3499.75,-0.375},{0,0,0},{-0d,-0d,-0d},{0,0,Math.PI/2},{1e100,-1e100,0},{Double.NaN,Double.POSITIVE_INFINITY,Double.NEGATIVE_INFINITY}};
        for(int i=0;i<offsets.length;i++){
            double[] values=offsets[i];result.add(new Spec("offset-"+i).add(new Action("offset",values[0],values[1],values[2])).add(new Action("bounds",false)));
        }
        for(double value:new double[]{-0d,0,Math.nextDown(1e-7),1e-7,Math.nextUp(1e-7),-Math.nextDown(1e-7),-1e-7,-Math.nextUp(1e-7),Double.MIN_VALUE}){
            Spec friction=new Spec("friction-boundary-"+bits(value));friction.friction=1;
            friction.add(new Action("rubbing").set("dx",value).set("dy",value).set("dtheta",value));result.add(friction);
        }
        result.add(new Spec("friction-default").add(new Action("rubbing").set("dx",1.25d).set("dy",-0.75d).set("dtheta",0.125d)));
        result.add(new Spec("friction-nonfinite").add(new Action("rubbing").set("dx",Double.NaN).set("dy",Double.POSITIVE_INFINITY).set("dtheta",Double.NEGATIVE_INFINITY)));
        Spec zeroFriction=new Spec("friction-zero-nonfinite");zeroFriction.friction=0;zeroFriction.add(new Action("rubbing").set("dx",Double.POSITIVE_INFINITY).set("dy",Double.NEGATIVE_INFINITY).set("dtheta",Double.NaN));result.add(zeroFriction);
        for(boolean shrink:new boolean[]{false,true}){
            Spec zero=ratio("zero-mass-"+(shrink?"shrink":"growth"),shrink?14:16);zero.symmetry=1;zero.mirror=0;zero.colors=new String[]{"GREEN"};zero.length=0d;
            zero.add(motion(new Action("grow")).set("_energy",shrink?-1d:0d).set("_age",7).set("active",true)).add(new Action("rubbing")).add(new Action("offset_velocity")).add(new Action("bounds",true));result.add(zero);
        }
        for(int modifier:new int[]{1,2,3,6,7,10}){
            Spec child=new Spec("real-spore-birth-cache-"+modifier);child.setup="spore_birth";child.symmetry=1;child.mirror=0;child.colors=new String[]{"GREEN","SPORE","EYE"};child.spore=modifier;
            child.add(new Action("bounds",false)).add(new Action("bounds",true));result.add(child);
        }
        return result;
    }
    public static void main(String[] args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.MovementOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        grow=Organism.class.getDeclaredMethod("grow");grow.setAccessible(true);rubbing=Organism.class.getDeclaredMethod("rubbingFramesEffects");rubbing.setAccessible(true);
        offset=Organism.class.getDeclaredMethod("offset",double.class,double.class,double.class);offset.setAccessible(true);
        StringBuilder output=new StringBuilder();
        for(Spec spec:specifications()){
            Capture capture=prepare(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));capture.row("E");output.append(capture.rows);cases++;
        }
        Files.writeString(directory.resolve("movement.tsv"),output.toString(),StandardCharsets.UTF_8);
        System.out.println("MOVEMENT_ORACLE PASS cases="+cases);
    }
}
