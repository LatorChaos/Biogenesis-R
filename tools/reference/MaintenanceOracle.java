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
import org.locationtech.jts.index.strtree.STRtree;

/** Observe original respiration, spore maintenance, hatching, and decomposition */
public final class MaintenanceOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");

    private static int cases,snapshots,calls;

    private static final class Action{
        final String operation,argument;
        final Map<String,Object> controls=new LinkedHashMap<>();
        Action(String operation,String argument){this.operation=operation;this.argument=argument;}
        Action set(String target,Object value){controls.put(target,value);return this;}
    }
    private static final class Spec{
        final String name;
        String[] colors={"GREEN","CYAN","EYE"};
        int spore=9;
        boolean zero;
        final Map<String,Object> settings=new LinkedHashMap<>();
        final Map<String,Object> controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        Spec(String name){this.name=name;controls.put("actor:victim:_energy",20d);controls.put("actor:victim:_maintenance",0.1d);controls.put("actor:victim:_age",0);controls.put("actor:victim:_max_age",10);controls.put("actor:victim:_sporeversion",0);}
        Spec add(Action action){actions.add(action);return this;}
        Spec set(String target,Object value){controls.put(target,value);return this;}
        Spec setting(String name,Object value){settings.put(name,value);return this;}
    }
    private static final class Capture{
        final StringBuilder rows=new StringBuilder();
        final Map<String,GeneticCode> genomes=new LinkedHashMap<>();
        final Map<String,Organism> actors=new LinkedHashMap<>();
        final IdentityHashMap<GeneticCode,String> references=new IdentityHashMap<>();
        final IdentityHashMap<Organism,String> actorNames=new IdentityHashMap<>();
        final World world;
        Capture(Spec spec)throws Exception{
            for(Field original:Utils.class.getDeclaredFields())if(original.getName().startsWith("DEF_"))Utils.class.getDeclaredField(original.getName().substring(4)).set(null,original.get(null));
            Utils.INITIAL_ORGANISMS=0;Utils.INITIAL_O2=1000;Utils.MIN_MUTATION_RATE=0;Utils.MAX_MUTATION_RATE=10000;Utils.META_MUTATION_RATE=0;
            for(String name:COLORS)Utils.class.getDeclaredField(name+"_PROB").setInt(null,name.equals("GREEN")?7:0);
            for(Map.Entry<String,Object> entry:spec.settings.entrySet())Utils.class.getDeclaredField(entry.getKey()).set(null,entry.getValue());
            Utils.random=new Random(827364);
            VisibleWorld visible=new VisibleWorld(null);world=new World(visible);
            world.addCO2(Utils.INITIAL_CO2);world.addO2(Utils.INITIAL_O2);world.addCH4(Utils.INITIAL_CH4);
            register("g0",genome(spec.colors,spec.spore,spec.zero));
            register("g1",genome(new String[]{"GREEN"},1,false));register("g2",genome(new String[]{"GREEN"},1,false));
            register("g3",(GeneticCode)genomes.get("g0").clone());register("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,"canonical",827364);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g3")||entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            Organism selector=paste("selector","g2",1000,1000);visible._selectedOrganism=selector;
            row("I","setup","visible:selected","String","selector");world.addOrganism(selector,null);
            Organism victim=paste("victim","g0",3500,3500);world.addOrganism(victim,null);
            Organism killer=paste("killer","g1",2000,2000);world.addOrganism(killer,null);
            for(Map.Entry<String,Object> entry:spec.controls.entrySet())control("setup",entry.getKey(),entry.getValue());
            snapshot("initial");
        }
        void register(String label,GeneticCode code){genomes.put(label,code);references.put(code,label);}
        String reference(GeneticCode code){if(code==null)return "~";String label=references.get(code);if(label==null){label="g"+genomes.size();register(label,code);}return label;}
        void discover(String stage){
            for(Organism actor:world._organisms)if(!actorNames.containsKey(actor)){
                String label="newborn"+(actors.size()-3);actors.put(label,actor);actorNames.put(actor,label);row("D",stage,label,actor.getID(),reference(actor._geneticCode));
            }
            for(Organism actor:actors.values()){reference(actor._geneticCode);reference(actor._infectedGeneticCode);reference(actor._savedGeneticCode);}
        }
        Organism paste(String role,String code,int x,int y)throws Exception{
            Organism actor=new Organism(world,genomes.get(code));require(actor.pasteOrganism(x,y),"Real actor paste failed");actors.put(role,actor);actorNames.put(actor,role);row("A",role,code,x,y);return actor;
        }
        void row(Object...values){for(int i=0;i<values.length;i++){if(i!=0)rows.append('\t');rows.append(values[i]);}rows.append('\n');}
        String scalar(Object value){
            if(value==null)return "~";
            if(value instanceof Double number)return bits(number);
            if(value instanceof Color color)return String.format(java.util.Locale.ROOT,"%08x",color.getRGB());
            if(value instanceof GeneticCode code)return reference(code);
            if(value instanceof Organism actor){String label=actorNames.get(actor);require(label!=null,"Unknown actor reference");return label;}
            if(value instanceof Rectangle r)return r.x+","+r.y+","+r.width+","+r.height;
            if(value instanceof World||value instanceof VisibleWorld||value instanceof WorldStatistics)return "present";
            if(value instanceof STRtree tree)return "present,"+tree.size();
            if(value instanceof List<?> list){StringBuilder text=new StringBuilder();for(Object item:list){if(text.length()!=0)text.append(',');text.append(scalar(item));}return text.toString();}
            if(value.getClass().isArray()){StringBuilder text=new StringBuilder();for(int i=0;i<Array.getLength(value);i++){if(i!=0)text.append(',');text.append(scalar(Array.get(value,i)));}return text.toString();}
            return value.toString();
        }
        void genomeState(String stage,String name,GeneticCode code)throws Exception{
            row("G",stage,name,"_reference",references.get(code));row("G",stage,name,"_nGenes",code.getNGenes());
            for(Field field:fields(GeneticCode.class))if(!field.getName().equals("_genes"))row("G",stage,name,field.getName(),scalar(field.get(code)));
            for(int i=0;i<code.getNGenes();i++)for(Field field:fields(Gene.class))row("N",stage,name,i,field.getName(),scalar(field.get(code.getGene(i))));
        }
        void snapshot(String stage)throws Exception{
            discover(stage);row("P",stage);
            for(Map.Entry<String,Organism> entry:actors.entrySet()){
                for(Field field:fields(Organism.class)){
                    Object value=field.get(entry.getValue());
                    row("O",stage,entry.getKey(),field.getName(),field.getType().getSimpleName(),value instanceof GeneticCode?"present":scalar(value));
                }
                row("O",stage,entry.getKey(),"bounds","Rectangle",scalar(entry.getValue().getBounds()));
                row("H",stage,entry.getKey(),"genotype",entry.getValue()._geneticCode==null?"~":references.get(entry.getValue()._geneticCode));
                row("H",stage,entry.getKey(),"infected",entry.getValue()._infectedGeneticCode==null?"~":references.get(entry.getValue()._infectedGeneticCode));
                row("H",stage,entry.getKey(),"saved",entry.getValue()._savedGeneticCode==null?"~":references.get(entry.getValue()._savedGeneticCode));
            }
            row("W",stage,"_population","int",world.getPopulation());row("W",stage,"organism_count","int",world._organisms.size());
            row("W",stage,"corpses","int",world.getNCorpses());row("W",stage,"_O2","double",bits(world.getO2()));
            row("W",stage,"_CO2","double",bits(world.getCO2()));row("W",stage,"_CH4","double",bits(world.getCH4()));
            row("W",stage,"NEXT_ID","int",world.NEXT_ID);row("W",stage,"nFrames","long",world.getFrame());row("W",stage,"time","long",world.getTime());
            StringBuilder ids=new StringBuilder();for(Organism actor:world._organisms){if(ids.length()!=0)ids.append(',');ids.append(actor.getID());}row("W",stage,"organism_ids","int[]",ids);
            Field[] worldFields=fields(World.class);require(worldFields.length==15,"Unexpected world schema");
            for(Field field:worldFields)row("V",stage,field.getName(),field.getType().getSimpleName(),scalar(field.get(world)));
            for(Map.Entry<String,Organism> entry:actors.entrySet()){Organism actor=entry.getValue();row("Q",stage,entry.getKey(),scalar(world.colDetTree.query(new Envelope(actor.getX(),actor.getMaxX(),actor.getY(),actor.getMaxY()))));}
            Field[] statsFields=fields(WorldStatistics.class);require(statsFields.length==52,"Unexpected statistics schema");
            for(Field field:statsFields)row("S",stage,field.getName(),field.getType().getSimpleName(),scalar(field.get(world.worldStatistics)));
            Method[] getters=Arrays.stream(WorldStatistics.class.getDeclaredMethods()).filter(method->Modifier.isPublic(method.getModifiers())&&method.getParameterCount()==0&&method.getName().startsWith("get")).sorted(Comparator.comparing(Method::getName)).toArray(Method[]::new);
            require(getters.length==49,"Unexpected statistics getter schema");for(Method method:getters)row("T",stage,method.getName(),method.getReturnType().getSimpleName(),scalar(method.invoke(world.worldStatistics)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet())genomeState(stage,entry.getKey(),entry.getValue());
            Random peek=copyRandom(Utils.random);row("R",stage,peek.nextLong(),peek.nextInt(10000),bits(peek.nextDouble()));snapshots++;
        }
        void control(String stage,String target,Object value)throws Exception{
            String[] path=target.split(":",3);Object owner;String name;
            if(path[0].equals("rng")){require(path[1].equals("seed"),"Unknown RNG control");Utils.random=new Random((Long)value);row("I",stage,target,"long",value);return;}
            switch(path[0]){case "actor":owner=actors.get(path[1]);name=path[2];break;case "world":owner=world;name=path[1];break;case "statistics":owner=world.worldStatistics;name=path[1];break;case "genome":owner=genomes.get(path[1]);name=path[2];break;case "utils":owner=null;name=path[1];break;default:throw new IllegalArgumentException("Unknown control target");}
            if(name.equals("bounds")){((Organism)owner).setBounds((Rectangle)value);row("I",stage,target,"Rectangle",scalar(value));return;}
            Field field=(owner==null?Utils.class:owner.getClass()).getDeclaredField(name);field.setAccessible(true);
            if(field.getType()==GeneticCode.class){field.set(owner,value.equals("~")?null:genomes.get((String)value));row("I",stage,target,"GeneticCode",value);}
            else{field.set(owner,value);row("I",stage,target,field.getType().getSimpleName(),scalar(value));}
        }
        void action(int index,Action action)throws Exception{
            String stage="step-"+index;
            for(Map.Entry<String,Object> input:action.controls.entrySet())control(stage,input.getKey(),input.getValue());
            Organism victim=actors.get("victim");row("K",stage,action.operation,"victim",action.argument);
            switch(action.operation){
                case "breath":victim.breath();break;
                case "decay":victim.decay();break;
                case "hatch":victim.hatch();break;
                case "useDecayEnergy":row("B",stage,"useDecayEnergy",victim.useDecayEnergy(Double.longBitsToDouble(Long.parseUnsignedLong(action.argument,16))));break;
                case "die":victim.die(null);break;
                case "symmetric_bounds":victim.symmetric();victim.calculateBounds(true);break;
                case "blueSpore":victim.blueSpore();break;
                case "cyanSpore":victim.cyanSpore();break;
                case "build_tree":
                    world.colDetTree=new STRtree();
                    if(!action.argument.equals("~"))for(String label:action.argument.split(",")){
                        Organism actor=actors.get(label);world.colDetTree.insert(new Envelope(actor.getX(),actor.getMaxX(),actor.getY(),actor.getMaxY()),actor);
                        row("J",stage,label,actor.x,actor.y,actor.width,actor.height);
                    }
                    world.colDetTree.build();break;
                default:throw new IllegalArgumentException("Unknown actual operation");
            }
            calls++;snapshot(stage);
        }
        String finish(){row("E");cases++;return rows.toString();}
    }
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static Random copyRandom(Random random)throws Exception{ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception{
        switch(name){case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static GeneticCode genome(String[] colors,int spore,boolean zero)throws Exception{
        List<Gene> genes=new ArrayList<>();for(int i=0;i<colors.length;i++){
            Gene gene=new Gene();gene.setLength(zero?0:12);gene.setTheta(i*0.25d);gene.setBranch(0);gene.setColor(color(colors[i]));genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,1,0,0,0,1,1,spore,1,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false);code._generation=41;code._cladeID="root";return code;
    }
    private static Action call(String name){return new Action(name,"~");}
    private static Spec breath(String name){return new Spec(name).add(call("breath"));}
    private static Spec decay(String name,int version){return new Spec(name).set("actor:victim:active",false).set("actor:victim:_sporeversion",version).set("actor:victim:_timeToReproduce",3).set("actor:victim:_gold",20).add(call("decay"));}
    private static Spec hatch(String name,int modifier){Spec spec=new Spec(name);spec.spore=modifier;return spec.set("actor:victim:active",false).add(call("hatch"));}
    private static void aliases(Spec spec){spec.set("actor:victim:_infectedGeneticCode","g3").set("actor:victim:_savedGeneticCode","g4");}
    private static Spec blocked(Spec spec){spec.actions.add(0,new Action("build_tree","victim,killer").set("actor:killer:bounds",new Rectangle(3400,3400,200,200)));return spec;}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        result.add(breath("breath-normal"));
        result.add(breath("breath-age-shift-tie").set("actor:victim:_age",255).set("actor:victim:_max_age",1));
        result.add(breath("breath-age-out").set("actor:victim:_age",511).set("actor:victim:_max_age",1));
        result.add(breath("breath-age-wrap").set("actor:victim:_age",Integer.MAX_VALUE));
        result.add(breath("breath-negative-age").set("actor:victim:_age",-1));
        result.add(breath("breath-partial-oxygen").set("world:_O2",0.05d));
        result.add(breath("breath-empty-oxygen").set("world:_O2",0d));
        Spec exhausted=breath("breath-exhaustion-retains-phenotype").set("actor:victim:_energy",0d);aliases(exhausted);result.add(exhausted);
        result.add(breath("breath-exact-tolerance").set("actor:victim:_energy",Utils.tol).set("actor:victim:_maintenance",0d));
        result.add(breath("breath-negative-energy").set("actor:victim:_energy",-1d));
        result.add(breath("breath-nan-energy").set("actor:victim:_energy",Double.longBitsToDouble(0x7ff8000000000042L)));
        result.add(breath("breath-positive-infinite-energy").set("actor:victim:_energy",Double.POSITIVE_INFINITY));
        result.add(breath("breath-negative-infinite-energy").set("actor:victim:_energy",Double.NEGATIVE_INFINITY));
        result.add(breath("breath-nan-maintenance").set("actor:victim:_maintenance",Double.longBitsToDouble(0xfff8000000000042L)));
        result.add(breath("breath-infinite-maintenance").set("actor:victim:_maintenance",Double.POSITIVE_INFINITY));
        result.add(breath("breath-fractional-excess").set("actor:victim:_energy",70.75d).set("actor:victim:_maintenance",0d));
        result.add(breath("breath-double-excess-countdown").set("actor:victim:_energy",120d).set("actor:victim:_reproducelate",10).set("actor:victim:_timeToReproduce",5));
        result.add(breath("breath-late-threshold-tie").set("actor:victim:_energy",88d).set("actor:victim:_maintenance",0d).set("actor:victim:_reproducelate",10).set("actor:victim:_timeToReproduce",5));
        result.add(breath("breath-late-threshold-above").set("actor:victim:_energy",88.25d).set("actor:victim:_maintenance",0d).set("actor:victim:_reproducelate",10).set("actor:victim:_timeToReproduce",5));
        result.add(breath("breath-infectious-excess").set("actor:victim:_energy",70d).set("actor:victim:_reproducelate",10).set("actor:victim:_isinfectious",true).set("actor:victim:_timeToReproduce",5));
        result.add(breath("breath-double-threshold-tie").set("actor:victim:_energy",98d).set("actor:victim:_maintenance",0d));
        result.add(breath("breath-reproduction-int-multiply-wrap").set("actor:victim:_energy",2147483648d).set("actor:victim:_reproduceEnergy",1073741824).set("actor:victim:_maintenance",0d));
        result.add(breath("breath-reproduction-int-subtract-wrap").set("actor:victim:_energy",2147483648d).set("actor:victim:_reproduceEnergy",Integer.MAX_VALUE).set("actor:victim:_maintenance",0d));
        result.add(breath("breath-negative-reproduction-threshold").set("actor:victim:_energy",1d).set("actor:victim:_reproduceEnergy",-1).set("actor:victim:_maintenance",0d));
        for(String variant:new String[]{"success9","success10","blocked","oxygen-only","half-tie","integer-half-above","not-grown","infected-saved","saved-only"}){
            Spec spec=breath("dying-spore-"+variant).set("actor:victim:_age",511).set("actor:victim:_max_age",1).set("actor:victim:_sporeversion",-1).set("actor:victim:_energy",100d).set("actor:victim:_growthRatio",1).set("actor:victim:_reproduceEnergy",101);
            if(variant.equals("success10"))spec.spore=10;
            if(variant.equals("oxygen-only"))spec.set("actor:victim:_age",0).set("world:_O2",0d);
            if(variant.equals("half-tie"))spec.set("actor:victim:_energy",50d).set("actor:victim:_maintenance",0d);
            if(variant.equals("integer-half-above"))spec.set("actor:victim:_energy",50.25d).set("actor:victim:_maintenance",0d);
            if(variant.equals("not-grown"))spec.set("actor:victim:_growthRatio",2);
            if(variant.equals("infected-saved"))aliases(spec);
            if(variant.equals("saved-only"))spec.set("actor:victim:_savedGeneticCode","g4");
            if(variant.equals("blocked"))blocked(spec);
            spec.actions.add(0,call("symmetric_bounds"));result.add(spec);
        }
        for(int version:new int[]{2,1,5,6,3,4,7,-3,-2,-1,0})result.add(decay("decay-version-"+version,version));
        for(int version:new int[]{2,1,5,6})result.add(decay("decay-age-tie-"+version,version).set("actor:victim:_age",255).set("actor:victim:_max_age",1).set("actor:victim:_isfrozen",true));
        for(int version:new int[]{2,6})result.add(decay("decay-age-above-"+version,version).set("actor:victim:_age",511).set("actor:victim:_max_age",1));
        for(int version:new int[]{2,1,5,6,3,-3,-2}){
            Spec spec=decay("decay-exhausted-"+version,version).set("actor:victim:_energy",0d);aliases(spec);result.add(spec);
        }
        result.add(decay("decay-age-wrapping",2).set("actor:victim:_age",Integer.MAX_VALUE));
        result.add(decay("decay-mobile-v1",1).set("rng:seed",57L).set("actor:victim:_isfrozen",false));
        result.add(decay("decay-frozen-v1",1).set("rng:seed",57L).set("actor:victim:_isfrozen",true));
        result.add(decay("decay-mobile-v5",5).set("rng:seed",57L).set("actor:victim:_isfrozen",false));
        result.add(decay("decay-frozen-v5",5).set("rng:seed",57L).set("actor:victim:_isfrozen",true));
        result.add(decay("decay-partial-respiration-v1",1).set("world:_O2",0.0015d));
        result.add(decay("decay-partial-respiration-v5",5).set("world:_O2",0.001d));
        result.add(decay("decay-v5-half-threshold-tie",5).set("actor:victim:_energy",24.5027d).set("actor:victim:_isfrozen",true));
        result.add(decay("decay-v5-hatch-success",5).set("actor:victim:_energy",30d).set("actor:victim:_isfrozen",true).setting("SPORE_ENERGY_CONSUMPTION",2d));
        result.add(decay("decay-v5-hatch-payment-partial",5).set("actor:victim:_energy",30d).set("world:_O2",1d).set("actor:victim:_isfrozen",true).setting("SPORE_ENERGY_CONSUMPTION",2d));
        for(int version:new int[]{6,3,-3,-2})result.add(decay("decay-injury-blocks-"+version,version).set("actor:victim:_isinjured",true).set("world:_O2",0.0005d));
        result.add(decay("decay-injured-egg-keeps-timer",4).set("actor:victim:_isinjured",true).set("world:_O2",1d));
        result.add(decay("decay-egg-color-low-frame",4).set("actor:victim:_framesColor",1));
        result.add(decay("decay-egg-color-high-frame",4).set("actor:victim:_framesColor",2));
        result.add(decay("decay-egg-negative-timer",4).set("actor:victim:_timeToReproduce",-1));
        result.add(decay("decay-egg-timer-wrap",4).set("actor:victim:_timeToReproduce",Integer.MIN_VALUE));
        result.add(decay("decay-v3-hatch",3).set("actor:victim:_timeToReproduce",0).setting("SPORE_ENERGY_CONSUMPTION",2d));
        result.add(decay("decay-v3-hatch-payment-partial",3).set("actor:victim:_timeToReproduce",0).set("world:_O2",1d).setting("SPORE_ENERGY_CONSUMPTION",2d));
        result.add(decay("decay-v7-hatch",7).set("actor:victim:_timeToReproduce",0));
        result.add(decay("decay-cyan-mobile",-3).set("rng:seed",57L).set("actor:victim:_framesColor",1).set("actor:victim:_isfrozen",false));
        result.add(decay("decay-cyan-retains-temp-color",-3).set("rng:seed",57L).set("actor:victim:_framesColor",2).set("actor:victim:_color",Color.MAGENTA).set("actor:victim:_isfrozen",false));
        result.add(decay("decay-cyan-frozen",-3).set("rng:seed",57L).set("actor:victim:_isfrozen",true));
        result.add(decay("decay-cyan-partial-respiration",-3).set("world:_O2",0.0005d));
        for(int version:new int[]{-3,-2,-1,0}){
            result.add(decay("decay-ready-hatch-"+version,version).set("actor:victim:_timeToReproduce",0));
        }
        result.add(blocked(decay("decay-blocked-gold",-2).set("actor:victim:_timeToReproduce",0).set("actor:victim:_gold",2)));
        result.add(blocked(decay("decay-blocked-zero-gold-bypass",-2).set("actor:victim:_timeToReproduce",0).set("actor:victim:_gold",0)));
        result.add(blocked(decay("decay-blocked-negative-gold",-2).set("actor:victim:_timeToReproduce",0).set("actor:victim:_gold",-1)));
        result.add(decay("decay-injured-ready-positive-gold",-2).set("actor:victim:_timeToReproduce",0).set("actor:victim:_gold",2).set("actor:victim:_isinjured",true));
        result.add(decay("decay-injured-ready-zero-gold",-2).set("actor:victim:_timeToReproduce",0).set("actor:victim:_gold",0).set("actor:victim:_isinjured",true));
        result.add(decay("decay-negative-timer",-2).set("actor:victim:_timeToReproduce",-1));
        Spec zero=decay("decay-zero-mass-mobile",1).set("rng:seed",57L);zero.zero=true;result.add(zero);
        for(String oxygen:new String[]{"full","partial","empty"}){
            Spec spec=decay("corpse-decomposition-"+oxygen,0).set("actor:victim:alive",false).set("actor:victim:_energy",1d);
            if(oxygen.equals("partial"))spec.set("world:_O2",0.05d);if(oxygen.equals("empty"))spec.set("world:_O2",0d);result.add(spec);
        }
        result.add(decay("corpse-negative-energy",0).set("actor:victim:alive",false).set("actor:victim:_energy",-1d));
        result.add(decay("corpse-nan-energy",0).set("actor:victim:alive",false).set("actor:victim:_energy",Double.longBitsToDouble(0x7ff8000000000042L)));
        for(String payment:new String[]{"full","insufficient-energy","partial-oxygen","empty-oxygen","negative-q","nan-q","infinite-q"}){
            double q=1d;Spec spec=new Spec("use-decay-energy-"+payment);
            if(payment.equals("insufficient-energy"))spec.set("actor:victim:_energy",0.5d);
            if(payment.equals("partial-oxygen"))spec.set("world:_O2",0.5d);
            if(payment.equals("empty-oxygen"))spec.set("world:_O2",0d);
            if(payment.equals("negative-q"))q=-1d;
            if(payment.equals("nan-q"))q=Double.longBitsToDouble(0xfff8000000000042L);
            if(payment.equals("infinite-q"))q=Double.POSITIVE_INFINITY;
            spec.add(new Action("useDecayEnergy",bits(q)));result.add(spec);
        }
        for(int modifier:new int[]{9,10}){
            Spec spec=hatch("hatch-modifier-"+modifier,modifier).set("actor:victim:_age",0).set("actor:victim:_framesColor",9).set("actor:victim:_color",Color.MAGENTA);
            spec.actions.add(0,call(modifier==9?"blueSpore":"cyanSpore"));result.add(spec);
        }
        result.add(hatch("hatch-increments-existing-age",9).set("actor:victim:_age",1));
        result.add(hatch("hatch-age-wrap",10).set("actor:victim:_age",Integer.MAX_VALUE));
        Spec retained=hatch("hatch-coefficients-references-cache",9).set("actor:victim:_age",1).set("actor:victim:_photosynthesis",9d).set("actor:victim:_methanotrophy",7d).set("actor:victim:_leafphoto",5d).set("actor:victim:_forestphoto",3d).set("actor:victim:_colonyPhotosynthesis",2d).set("actor:victim:_isfrozen",true).set("actor:victim:_isinjured",true).set("actor:victim:_allfrozen",true).set("actor:victim:_framesColor",9).set("actor:victim:_color",Color.MAGENTA);aliases(retained);result.add(retained);
        result.add(hatch("hatch-partial-first-payment",9).setting("SPORE_ENERGY_CONSUMPTION",2d).set("world:_O2",1d));
        result.add(hatch("hatch-first-energy-guard-full-fallback",9).setting("SPORE_ENERGY_CONSUMPTION",2d).set("actor:victim:_energy",1d));
        result.add(hatch("hatch-partial-fallback",9).setting("SPORE_ENERGY_CONSUMPTION",2d).set("actor:victim:_energy",1d).set("world:_O2",0.005d));
        Spec fallback=hatch("hatch-fallback-exhaustion-retains-phenotype",9).setting("SPORE_ENERGY_CONSUMPTION",2d).set("actor:victim:_energy",0.005d);aliases(fallback);result.add(fallback);
        result.add(hatch("hatch-empty-oxygen",9).setting("SPORE_ENERGY_CONSUMPTION",2d).set("world:_O2",0d));
        result.add(decay("v6-methanotrophy-eighth-frame",6).set("actor:victim:_age",7).set("actor:victim:_methanotrophy",2d).set("world:_CH4",5000d));
        result.add(decay("v6-methanotrophy-other-frame",6).set("actor:victim:_age",6).set("actor:victim:_methanotrophy",2d).set("world:_CH4",5000d));
        result.add(decay("v6-injured-no-photo",6).set("actor:victim:_isinjured",true).set("actor:victim:_photosynthesis",2d));
        result.add(decay("v6-lime-alone",6).set("actor:victim:_islime",true).set("actor:victim:_photosynthesis",2.5d).set("actor:victim:_leafphoto",1.25d));
        result.add(blocked(decay("v6-lime-crowded",6).set("actor:victim:_islime",true).set("actor:victim:_photosynthesis",2.5d).set("actor:victim:_leafphoto",1.25d)));
        result.add(decay("v6-colony-photo-reset",6).set("actor:victim:_photosynthesis",2.5d).set("actor:victim:_colonyPhotosynthesis",1.25d));
        result.add(decay("v6-ordinary-photo",6).set("actor:victim:_photosynthesis",2.5d));
        result.add(decay("v6-late-hatch-countdown",6).set("actor:victim:_energy",30d).set("actor:victim:_timeToReproduce",2));
        result.add(decay("v6-late-hatch-negative-countdown",6).set("actor:victim:_energy",30d).set("actor:victim:_timeToReproduce",-1));
        result.add(decay("v6-ready-hatch",6).set("actor:victim:_energy",30d).set("actor:victim:_timeToReproduce",0).set("actor:victim:_jadefactor",4).set("actor:victim:_gold",-2));
        result.add(decay("v6-hatch-payment-partial",6).set("actor:victim:_energy",30d).set("actor:victim:_timeToReproduce",0).setting("SPORE_ENERGY_CONSUMPTION",2d).set("world:_O2",1d));
        result.add(decay("v6-blond-early-threshold",6).set("actor:victim:_isblond",true).set("actor:victim:_gold",0).set("actor:victim:_earlyReproduceEnergy",10).set("actor:victim:_energy",10d).set("actor:victim:_timeToReproduce",0));
        Spec viral=decay("v6-infected-viral-birth",6).set("actor:victim:_isblond",true).set("actor:victim:_sporetime",-1).set("actor:victim:_earlyReproduceEnergy",10).set("actor:victim:_energy",100d);aliases(viral);result.add(viral);
        Spec viralBlocked=decay("v6-infected-viral-blocked",6).set("actor:victim:_isblond",true).set("actor:victim:_sporetime",-1).set("actor:victim:_earlyReproduceEnergy",10).set("actor:victim:_energy",100d);aliases(viralBlocked);result.add(blocked(viralBlocked));
        Spec viralDrain=decay("v6-infected-viral-exhaustion",6).set("actor:victim:_isblond",true).set("actor:victim:_sporetime",-1).set("actor:victim:_earlyReproduceEnergy",10).set("actor:victim:_reproduceEnergy",1000).set("actor:victim:_infectedReproduceEnergy",1000).set("actor:victim:_energy",10d);aliases(viralDrain);result.add(viralDrain);
        Spec failedBlue=hatch("hatch-failure-after-real-blue-spore",9).setting("SPORE_ENERGY_CONSUMPTION",2d).set("world:_O2",0d).set("actor:victim:_color",Color.MAGENTA).set("actor:victim:_framesColor",9);failedBlue.actions.add(0,call("blueSpore"));result.add(failedBlue);
        for(String idMode:new String[]{"wrap","duplicate"}){
            Spec identity=decay("v6-newborn-id-"+idMode,6).set("actor:victim:_isblond",true).set("actor:victim:_sporetime",-1).set("actor:victim:_earlyReproduceEnergy",10).set("actor:victim:_energy",100d).set("world:NEXT_ID",idMode.equals("wrap")?Integer.MAX_VALUE:1);aliases(identity);result.add(identity);
        }
        for(Spec spec:result)if(spec.name.startsWith("corpse-")){spec.controls.remove("actor:victim:alive");spec.actions.add(0,call("die"));}
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.MaintenanceOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("maintenance.tsv"),text,StandardCharsets.UTF_8);
        System.out.println("MAINTENANCE_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
