// SPDX-License-Identifier: GPL-2.0-or-later
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

/** Observe the complete original CORAL touchEffects1 branch on real placed organisms */
public final class CoralOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static int cases,snapshots,calls,newborns;
    private static Method contact;
    private static final class Code {
        String[] colors={"GREEN"};
        int symmetry=1,mirror,rate,spore=1,adapt=1,generation=41;
        double length=12;
        boolean disperse,selfish,plague;
        String clade="root";
        Code colors(String...value){colors=value;return this;}
    }
    private static final class Action {
        final String operation,argument;
        final Map<String,Object> controls=new LinkedHashMap<>();
        Action(String operation,String argument){this.operation=operation;this.argument=argument;}
        Action set(String target,Object value){controls.put(target,value);return this;}
    }
    private static final class Spec {
        final String name;
        final Code parent=new Code().colors("CORAL"),recipient=new Code(),infected=new Code().colors("WHITE");
        final Map<String,Object> settings=new LinkedHashMap<>(),controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        long seed=827364;
        Spec(String name){
            this.name=name;
            controls.put("actor:victim:_energy",20d);controls.put("actor:killer:_energy",0.5d);
            for(String field:new String[]{"_isaplant","_isaconsumer","_isafungus","_isenhanced","_isauburn","_altruist"})controls.put("actor:victim:"+field,false);
            controls.put("actor:victim:_indigo",0);controls.put("actor:victim:_gold",0);
            for(String field:new String[]{"_isaplant","_isaconsumer","_isafungus","_transfersenergy","_isinfectious","_iscoral","_issilver","_dodge","_hasdodged"})controls.put("actor:killer:"+field,false);
            for(String field:new String[]{"_indigo","_antiviral","_lavender","_sporeversion"})controls.put("actor:killer:"+field,0);
            controls.put("actor:killer:active",true);
            controls.put("actor:killer:_infectedGeneticCode","g3");controls.put("actor:killer:_savedGeneticCode","g4");
        }
        Spec set(String target,Object value){controls.put(target,value);return this;}
        Spec setting(String target,Object value){settings.put(target,value);return this;}
        Spec add(Action value){actions.add(value);return this;}
    }
    private static final class Capture {
        final StringBuilder rows=new StringBuilder();
        final Map<String,GeneticCode> genomes=new LinkedHashMap<>();
        final Map<String,Organism> actors=new LinkedHashMap<>();
        final IdentityHashMap<GeneticCode,String> references=new IdentityHashMap<>();
        final IdentityHashMap<Organism,String> actorNames=new IdentityHashMap<>();
        final World world;
        final Spec spec;
        int discovered;
        Capture(Spec spec)throws Exception {
            this.spec=spec;
            for(Field field:Utils.class.getDeclaredFields())if(field.getName().startsWith("DEF_"))Utils.class.getDeclaredField(field.getName().substring(4)).set(null,field.get(null));
            Utils.INITIAL_ORGANISMS=0;Utils.INITIAL_O2=1000;Utils.MIN_MUTATION_RATE=0;Utils.MAX_MUTATION_RATE=10000;Utils.META_MUTATION_RATE=0;
            for(String name:COLORS)Utils.class.getDeclaredField(name+"_PROB").setInt(null,name.equals("GREEN")?7:0);
            for(Map.Entry<String,Object> entry:spec.settings.entrySet())Utils.class.getDeclaredField(entry.getKey()).set(null,entry.getValue());
            Utils.random=new Random(spec.seed);
            VisibleWorld visible=new VisibleWorld(null);world=new World(visible);
            world.addCO2(Utils.INITIAL_CO2);world.addO2(Utils.INITIAL_O2);world.addCH4(Utils.INITIAL_CH4);
            register("g0",genome(spec.parent));register("g1",genome(spec.recipient));register("g2",genome(new Code().colors("GREEN")));
            register("g3",genome(spec.infected));register("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,"coral",spec.seed);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            Organism blocker=paste("selector","g2",1000,1000);visible._selectedOrganism=blocker;
            row("I","setup","visible:selected","String","selector");world.addOrganism(blocker,null);
            Organism parent=paste("victim","g0",3500,3500);world.addOrganism(parent,null);
            Organism peer=paste("killer","g1",5000,5000);world.addOrganism(peer,null);
            for(Map.Entry<String,Object> entry:spec.controls.entrySet())control("setup",entry.getKey(),entry.getValue());
            snapshot("initial");
        }
        void register(String label,GeneticCode code){require(code!=null&&!references.containsKey(code),"Duplicate genome recipe identity");genomes.put(label,code);references.put(code,label);}
        String reference(GeneticCode code){if(code==null)return "~";String label=references.get(code);if(label==null){label="g"+genomes.size();register(label,code);}return label;}
        void discover(String stage)throws Exception {
            for(Organism actor:world._organisms)if(!actorNames.containsKey(actor)){
                String label="newborn"+discovered++;newborns++;actors.put(label,actor);actorNames.put(actor,label);row("D",stage,label,actor.getID(),reference(actor._geneticCode));
            }
            for(Organism actor:actors.values()){reference(actor._geneticCode);reference(actor._infectedGeneticCode);reference(actor._savedGeneticCode);}
            for(Field field:fields(WorldStatistics.class))if(field.getType()==GeneticCode.class)reference((GeneticCode)field.get(world.worldStatistics));
        }
        Organism paste(String role,String code,int x,int y)throws Exception {
            Organism actor=new Organism(world,genomes.get(code));require(actor.pasteOrganism(x,y),"Real actor paste failed: "+role);
            actors.put(role,actor);actorNames.put(actor,role);row("A",role,code,x,y);return actor;
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

        void snapshot(String stage)throws Exception {
            discover(stage);row("P",stage);
            IdentityHashMap<GeneticCode,Boolean> available=new IdentityHashMap<>();
            for(int i=0;i<5;i++)available.put(genomes.get("g"+i),true);
            for(Map.Entry<String,Organism> entry:actors.entrySet()){
                Organism actor=entry.getValue();
                available.put(actor._geneticCode,true);available.put(actor._infectedGeneticCode,true);available.put(actor._savedGeneticCode,true);
                for(Field field:fields(Organism.class)){
                    Object value=field.get(actor);row("O",stage,entry.getKey(),field.getName(),field.getType().getSimpleName(),value instanceof GeneticCode?"present":scalar(value));
                }
                row("O",stage,entry.getKey(),"bounds","Rectangle",scalar(actor.getBounds()));
                row("H",stage,entry.getKey(),"genotype",reference(actor._geneticCode));row("H",stage,entry.getKey(),"infected",reference(actor._infectedGeneticCode));row("H",stage,entry.getKey(),"saved",reference(actor._savedGeneticCode));
            }
            for(Organism actor:world._organisms){available.put(actor._geneticCode,true);available.put(actor._infectedGeneticCode,true);available.put(actor._savedGeneticCode,true);}
            row("W",stage,"_population","int",world.getPopulation());row("W",stage,"organism_count","int",world._organisms.size());row("W",stage,"corpses","int",world.getNCorpses());
            row("W",stage,"_O2","double",bits(world.getO2()));row("W",stage,"_CO2","double",bits(world.getCO2()));row("W",stage,"_CH4","double",bits(world.getCH4()));
            row("W",stage,"NEXT_ID","int",world.NEXT_ID);row("W",stage,"nFrames","long",world.getFrame());row("W",stage,"time","long",world.getTime());
            StringBuilder ids=new StringBuilder();for(Organism actor:world._organisms){if(ids.length()!=0)ids.append(',');ids.append(actor.getID());}row("W",stage,"organism_ids","int[]",ids);
            Field[] worldFields=fields(World.class);require(worldFields.length==15,"Unexpected World instance schema");
            for(Field field:worldFields)row("V",stage,field.getName(),field.getType().getSimpleName(),scalar(field.get(world)));
            for(Map.Entry<String,Organism> entry:actors.entrySet()){Organism actor=entry.getValue();row("Q",stage,entry.getKey(),scalar(world.colDetTree.query(new Envelope(actor.getX(),actor.getMaxX(),actor.getY(),actor.getMaxY()))));}
            Field[] statsFields=fields(WorldStatistics.class);require(statsFields.length==52,"Unexpected statistics instance schema");
            for(Field field:statsFields){Object value=field.get(world.worldStatistics);if(value instanceof GeneticCode code)available.put(code,true);row("S",stage,field.getName(),field.getType().getSimpleName(),scalar(value));}
            Method[] getters=Arrays.stream(WorldStatistics.class.getDeclaredMethods()).filter(method->Modifier.isPublic(method.getModifiers())&&method.getParameterCount()==0&&method.getName().startsWith("get")).sorted(Comparator.comparing(Method::getName)).toArray(Method[]::new);
            require(getters.length==49,"Unexpected statistics getter schema");for(Method method:getters)row("T",stage,method.getName(),method.getReturnType().getSimpleName(),scalar(method.invoke(world.worldStatistics)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                row("Y",stage,entry.getKey(),available.containsKey(entry.getValue())?"observable":"rejected-only");genomeState(stage,entry.getKey(),entry.getValue());
            }
            Random peek=copyRandom(Utils.random);row("R",stage,peek.nextLong(),peek.nextInt(10000),bits(peek.nextDouble()));snapshots++;
        }
        void control(String stage,String target,Object value)throws Exception {
            String[] path=target.split(":",3);Object owner;String name;
            if(path[0].equals("rng")){require(path[1].equals("seed"),"Unknown RNG control");Utils.random=new Random((Long)value);row("I",stage,target,"long",value);return;}
            switch(path[0]){
                case "actor":owner=actors.get(path[1]);name=path[2];break;case "world":owner=world;name=path[1];break;
                case "statistics":owner=world.worldStatistics;name=path[1];break;case "genome":owner=genomes.get(path[1]);name=path[2];break;
                default:throw new IllegalArgumentException("Unknown explicit input target: "+target);
            }
            if(name.equals("bounds")){((Organism)owner).setBounds((Rectangle)value);row("I",stage,target,"Rectangle",scalar(value));return;}
            if(name.startsWith("gene")){
                int dot=name.indexOf('.');int index=Integer.parseInt(name.substring(4,dot));Gene gene=((GeneticCode)owner).getGene(index);String property=name.substring(dot+1);
                if(property.equals("_length")){gene.setLength((Double)value);row("I",stage,target,"double",scalar(value));}
                else if(property.equals("_color")){gene.setColor((Color)value);row("I",stage,target,"Color",scalar(value));}
                else throw new IllegalArgumentException("Unknown original Gene setter");return;
            }
            Field field=owner.getClass().getDeclaredField(name);field.setAccessible(true);
            if(field.getType()==GeneticCode.class){field.set(owner,value.equals("~")?null:genomes.get((String)value));row("I",stage,target,"GeneticCode",value);}
            else {field.set(owner,value);row("I",stage,target,field.getType().getSimpleName(),scalar(value));}
        }
        void action(int index,Action action)throws Exception {
            String stage="step-"+index;
            for(Map.Entry<String,Object> entry:action.controls.entrySet())control(stage,entry.getKey(),entry.getValue());
            String argument=action.argument;
            if(action.operation.equals("build_tree")&&argument.equals("members")){
                StringBuilder names=new StringBuilder();for(Organism member:world._organisms){if(names.length()!=0)names.append(',');names.append(actorNames.get(member));}argument=names.toString();
            }
            Organism parent=actors.get("victim");row("K",stage,action.operation,"victim",argument);
            switch(action.operation){
                case "touchEffects1":{
                    String[] arguments=argument.split(",");require(arguments.length==3&&arguments[0].equals("killer"),"Invalid CORAL contact arguments");
                    int segment=Integer.parseInt(arguments[1]),other=Integer.parseInt(arguments[2]);
                    require(parent._segColor[segment].equals(Utils.ColorCORAL),"Caller must enter the actual CORAL case");
                    contact.invoke(parent,actors.get(arguments[0]),segment,other);break;
                }
                case "build_tree":{
                    world.colDetTree=new STRtree();if(!argument.equals("~"))for(String name:argument.split(",")){
                        Organism actor=actors.get(name);require(actor!=null,"Unknown actual member identity");world.colDetTree.insert(new Envelope(actor.getX(),actor.getMaxX(),actor.getY(),actor.getMaxY()),actor);
                        row("J",stage,name,actor.x,actor.y,actor.width,actor.height);
                    }world.colDetTree.build();break;
                }
                default:throw new IllegalArgumentException("Unknown original operation: "+action.operation);
            }
            calls++;snapshot(stage);
        }
        String finish(){cases++;row("E");return rows.toString();}
    }
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Random copyRandom(Random random)throws Exception {ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception {
        switch(name){case "UNKNOWN":return new Color(0x12345678,true);case "TRANSLUCENT":return new Color(0x12ff00ff,true);case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static GeneticCode genome(Code recipe)throws Exception {
        List<Gene> genes=new ArrayList<>();for(int index=0;index<recipe.colors.length;index++){
            Gene gene=new Gene();gene.setLength(recipe.length);gene.setTheta(index*0.25);gene.setBranch(0);gene.setColor(color(recipe.colors[index]));genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,recipe.symmetry,recipe.mirror,recipe.rate,0,1,1,recipe.spore,recipe.adapt,recipe.plague,recipe.disperse,false,false,false,false,false,false,false,false,false,false,false,false,recipe.selfish);
        code._generation=recipe.generation;code._cladeID=recipe.clade;return code;
    }
    private static Action touch(){return new Action("touchEffects1","killer,0,0");}
    private static Action tree(){return new Action("build_tree","members");}
    private static Color[] phenotype(String...names)throws Exception{Color[] colors=new Color[names.length];for(int i=0;i<names.length;i++)colors[i]=color(names[i]);return colors;}
    private static Spec scenario(String name,String peerColor)throws Exception{return new Spec(name).set("actor:killer:_segColor",phenotype(peerColor)).add(tree()).add(touch());}
    private static Spec blocked(Spec spec){return spec.set("actor:selector:bounds",new Rectangle(0,0,7000,7000));}
    private static Spec fungus(Spec spec){return spec.set("actor:killer:_isafungus",true).set("actor:victim:_isenhanced",true);}
    private static List<Spec> canonical()throws Exception {
        List<Spec> result=new ArrayList<>();
        List<String> palette=new ArrayList<>(Arrays.asList(COLORS));palette.addAll(Arrays.asList(INTERNAL));palette.add("UNKNOWN");palette.add("TRANSLUCENT");
        require(palette.size()==66,"CORAL palette inventory drift");
        for(String name:palette)result.add(scenario("palette-"+name,name));
        for(String kind:new String[]{"plant","consumer","fungus","enhanced-fungus","silver-fungus","plant-fungus","consumer-fungus","silver-ordinary"}){
            Spec spec=scenario("eligibility-"+kind,"WHITE");
            if(kind.contains("plant"))spec.set("actor:killer:_isaplant",true);
            if(kind.contains("consumer"))spec.set("actor:killer:_isaconsumer",true);
            if(kind.contains("fungus"))spec.set("actor:killer:_isafungus",true);
            if(kind.contains("silver"))spec.set("actor:killer:_issilver",true);
            if(!kind.equals("fungus"))spec.set("actor:victim:_isenhanced",true);
            result.add(spec);
        }
        for(String bypass:new String[]{"none","infectious","coral","fungus"}){
            Spec spec=scenario("transfer-gate-"+bypass,"WHITE").set("actor:killer:_transfersenergy",true);
            if(bypass.equals("infectious"))spec.set("actor:killer:_isinfectious",true);
            if(bypass.equals("coral"))spec.set("actor:killer:_iscoral",true);
            if(bypass.equals("fungus"))fungus(spec);
            result.add(spec);
        }
        for(String own:new String[]{"none","indigo","plant","consumer","fungus"}){
            Spec spec=scenario("coral-indigo-own-"+own,"CORAL").set("actor:killer:_indigo",1).set("actor:victim:_indigo",own.equals("none")?0:1);
            if(!own.equals("none")&&!own.equals("indigo"))spec.set("actor:victim:_isa"+own,true);
            result.add(spec);
        }
        result.add(fungus(scenario("coral-indigo-fungus-is-not-protected","CORAL").set("actor:killer:_indigo",1)));
        for(double oxygen:new double[]{1d,0.005d,0d})result.add(scenario("white-indigo-payment-"+bits(oxygen),"WHITE").set("actor:killer:_indigo",1).set("actor:killer:_antiviral",1).set("world:_O2",oxygen));
        result.add(scenario("indigo-uninfected-transfer-still-pays","INDIGO").set("actor:killer:_transfersenergy",true).set("actor:killer:_antiviral",1).setting("INDIGO_ENERGY_CONSUMPTION",2d));
        result.add(scenario("indigo-infectious-converts","INDIGO").set("actor:killer:_isinfectious",true).set("actor:killer:_indigo",1));
        for(int shield:new int[]{1,184,185,186,Integer.MAX_VALUE})result.add(scenario("normal-shield-"+shield,"WHITE").set("actor:killer:_lavender",shield));
        for(int shield:new int[]{2219,2220,2221})result.add(scenario("weak-shield-"+shield,"WHITE").set("actor:victim:_isauburn",true).set("actor:killer:_lavender",shield));
        for(String classification:new String[]{"plant","consumer","fungus","peer-fungus"}){
            Spec spec=scenario("weak-shield-disabled-"+classification,"WHITE").set("actor:victim:_isauburn",true).set("actor:killer:_lavender",186);
            if(classification.equals("peer-fungus"))fungus(spec);else spec.set("actor:victim:_isa"+classification,true);
            result.add(spec);
        }
        result.add(scenario("default-shield-does-not-mark-dodge","GREEN").set("actor:killer:_lavender",186));
        result.add(scenario("spore-six-shield-does-not-mark-dodge","SPORE").set("actor:killer:_sporeversion",6).set("actor:killer:_lavender",186).set("actor:killer:active",false));
        result.add(fungus(scenario("yellow-fungus-shield-does-not-mark-dodge","YELLOW").set("actor:killer:_lavender",186)));
        result.add(fungus(scenario("yellow-inactive-fungus-does-not-attack","YELLOW").set("actor:killer:active",false).set("actor:killer:_antiviral",1)));
        for(String state:new String[]{"full","partial","shortfall","empty","negative","nan","zero-exhaustion"}){
            Spec spec=scenario("dodge-"+state,"WHITE").set("actor:killer:_dodge",true).setting("DODGE_ENERGY_CONSUMPTION",0.25d).setting("CORAL_ENERGY_CONSUMPTION",0.25d).set("actor:victim:_energy",1d).set("actor:killer:_energy",1d);
            if(state.equals("partial"))spec.set("world:_O2",0.125d);
            if(state.equals("shortfall"))spec.set("actor:killer:_energy",0.1d).set("world:_O2",0.125d);
            if(state.equals("empty"))spec.set("world:_O2",0d);
            if(state.equals("negative"))spec.set("world:_O2",-0.125d);
            if(state.equals("nan"))spec.set("world:_O2",Double.NaN);
            if(state.equals("zero-exhaustion"))spec.set("actor:killer:_energy",0d).setting("DODGE_ENERGY_CONSUMPTION",0d);
            result.add(spec);
        }
        for(double oxygen:new double[]{1d,0.125d,0d,-0.125d,Double.NaN})result.add(scenario("direct-payment-oxygen-"+bits(oxygen),"BROKEN").setting("CORAL_ENERGY_CONSUMPTION",0.25d).set("actor:victim:_energy",1d).set("world:_O2",oxygen));
        for(double energy:new double[]{0d,0.125d,0.25d,Double.POSITIVE_INFINITY,Double.NaN})result.add(scenario("direct-payment-energy-"+bits(energy),"BROKEN").setting("CORAL_ENERGY_CONSUMPTION",0.25d).set("actor:victim:_energy",energy));
        result.add(scenario("zero-cost-converts-with-zero-attacker-energy","BROKEN").setting("CORAL_ENERGY_CONSUMPTION",0d).set("actor:victim:_energy",0d).set("world:_O2",-0d));
        for(int shield:new int[]{0,186}){
            Spec spec=scenario("antiviral-mutation-continues-"+shield,"WHITE").set("actor:killer:_antiviral",1).set("actor:killer:_lavender",shield).set("actor:victim:_gold",1);
            spec.parent.colors("CORAL","WHITE","PLAGUE");result.add(spec);
        }
        result.add(scenario("antiviral-protective-active","SKY").set("actor:killer:_antiviral",1));
        result.add(scenario("antiviral-protective-inactive","SKY").set("actor:killer:_antiviral",1).set("actor:killer:active",false));
        result.add(scenario("antiviral-spore-other-version","SPORE").set("actor:killer:_antiviral",1).set("actor:killer:_sporeversion",5));
        result.add(scenario("antiviral-yellow-ordinary","YELLOW").set("actor:killer:_antiviral",1));
        result.add(scenario("injured-bypasses-antiviral-shield-and-dodge","BROKEN").set("actor:killer:_antiviral",1).set("actor:killer:_lavender",10000).set("actor:killer:_dodge",true));
        for(int version:new int[]{0,2})result.add(scenario("fruit-other-version-"+version,"FRUIT").set("actor:killer:_sporeversion",version));
        result.add(scenario("fruit-warning-then-conversion","FRUIT").set("actor:killer:_sporeversion",1).set("actor:killer:_framesColor",0).add(touch()));
        result.add(scenario("fruit-parent-plant-bypasses-warning","FRUIT").set("actor:killer:_sporeversion",1).set("actor:killer:_framesColor",0).set("actor:victim:_isaplant",true));
        for(String classification:new String[]{"ordinary","consumer","fungus"}){
            Spec spec=scenario("fruit-auburn-parent-"+classification,"FRUIT").set("actor:killer:_sporeversion",1).set("actor:killer:_framesColor",0).set("actor:victim:_isauburn",true);
            if(!classification.equals("ordinary"))spec.set("actor:victim:_isa"+classification,true);result.add(spec);
        }
        for(boolean enhanced:new boolean[]{false,true})result.add(scenario("lavender-replenishment-"+enhanced,"LAVENDER").set("actor:killer:_isenhanced",enhanced));
        for(int shield:new int[]{9999,10000})result.add(scenario("lavender-cap-"+shield,"LAVENDER").set("actor:killer:_lavender",shield));
        result.add(scenario("lavender-wrapped-increment","LAVENDER").setting("LAVENDER_SHIELD",Integer.MAX_VALUE).set("actor:killer:_isenhanced",true).set("actor:killer:_lavender",Integer.MAX_VALUE-500));
        for(double oxygen:new double[]{0d,0.05d,Double.NaN})result.add(scenario("lavender-payment-"+bits(oxygen),"LAVENDER").set("world:_O2",oxygen));
        result.add(scenario("lavender-attacker-exhaustion","LAVENDER").set("actor:victim:_energy",0.1d).set("actor:killer:_lavender",9999));
        result.add(scenario("lavender-mutual-exhaustion","LAVENDER").set("actor:victim:_energy",0.1d).set("actor:killer:_energy",0d));
        for(double energy:new double[]{0d,-0.1d,Utils.tol,Double.NaN})result.add(scenario("noop-common-death-"+bits(energy),"BARK").set("actor:killer:_energy",energy));
        result.add(scenario("altruist-break-still-checks-peer-energy","MAGENTA").set("actor:victim:_altruist",true).set("actor:killer:_energy",0d));
        result.add(scenario("ineligible-still-checks-peer-energy","BLUE").set("actor:killer:_isaplant",true).set("actor:killer:_energy",0d));
        result.add(scenario("existing-corpse-is-not-killed-by-common-guard","BROWN").set("actor:killer:alive",false).set("actor:killer:_energy",0d));
        for(double energy:new double[]{Double.longBitsToDouble(0x3fefffffffffffffL),1d,20d})result.add(scenario("transformation-energy-"+bits(energy),"WHITE").set("actor:killer:_energy",energy));
        result.add(blocked(scenario("failed-transformation-still-kills-recipient","WHITE").set("actor:killer:_energy",20d)));
        result.add(scenario("fallback-transformation-placement","WHITE").set("actor:killer:_energy",20d).set("actor:selector:bounds",new Rectangle(4999,4999,2,2)));
        result.add(blocked(scenario("candidate-id-parent-filter-allows-center","WHITE").set("actor:killer:_energy",20d).set("actor:selector:_parentID",0)));
        for(int next:new int[]{Integer.MAX_VALUE,1,-1})result.add(scenario("transformation-next-id-"+next,"WHITE").set("actor:killer:_energy",20d).set("world:NEXT_ID",next));
        result.add(scenario("distinct-recipient-duplicate-id","WHITE").set("actor:killer:_energy",20d).set("actor:killer:_ID",0));
        result.add(scenario("frozen-tree-retains-dead-recipient","WHITE").set("actor:killer:_energy",20d).add(touch()));
        Spec shared=scenario("inherited-gene-edit-and-real-tree-rebuild","WHITE").set("actor:killer:_energy",20d);
        shared.add(tree().set("genome:g0:gene0._length",12.5d).set("genome:g0:_cladeID","coral-edit"));result.add(shared);
        Spec mutated=scenario("fully-mutated-transformation","WHITE").set("actor:killer:_energy",20d);mutated.parent.rate=10000;result.add(mutated);
        result.add(fungus(scenario("yellow-fungus-exhausted-shield-converts-without-dodge","YELLOW").set("actor:killer:_lavender",185)));
        result.add(scenario("spore-six-exhausted-shield-converts-without-dodge","SPORE").set("actor:killer:_sporeversion",6).set("actor:killer:_lavender",185).set("actor:killer:active",false));
        return result;
    }
    private static String capture()throws Exception {
        contact=Organism.class.getDeclaredMethod("touchEffects1",Organism.class,int.class,int.class);contact.setAccessible(true);
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception {
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.CoralOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        Files.writeString(directory.resolve("coral.tsv"),capture(),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("CORAL_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls+" newborns="+newborns);
    }
}
