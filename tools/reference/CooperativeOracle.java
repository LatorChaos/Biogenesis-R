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

/** Observe six original touchEffects2 branches with actual held-world reproduction */
public final class CooperativeOracle {
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
        final String name,weapon;
        final Code parent=new Code(),recipient=new Code(),infected=new Code().colors("WHITE");
        final Map<String,Object> settings=new LinkedHashMap<>(),controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        long seed=827364;
        Spec(String name,String weapon,String target)throws Exception {
            this.name=name;this.weapon=weapon;parent.colors(weapon);
            for(String role:new String[]{"victim","killer"}) {
                controls.put("actor:"+role+":_energy",100d);
                controls.put("actor:"+role+":active",true);
                controls.put("actor:"+role+":_framesColor",0);
                for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus","_isenhanced","_isinfectious","_iscoral","_isgray","_isinjured","_isfrozen","_allfrozen","_altruist"})controls.put("actor:"+role+":"+flag,false);
                for(String field:new String[]{"_plagueversion","_fallowversion","_sporeversion","_antiviral","_lavender","_createlavender"})controls.put("actor:"+role+":"+field,0);
            }
            controls.put("actor:killer:_segColor",phenotype(target));
            if(List.of("ROSE","MINT","LAVENDER","MAGENTA").contains(weapon)){
                controls.put("actor:victim:_altruist",true);controls.put("actor:killer:_altruist",true);
            }
            actions.add(tree());actions.add(new Action("touchEffects2."+weapon,"killer,0,0"));
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
            row("C",spec.name,"cooperative",spec.seed);
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
            if(owner instanceof Organism actor&&field.getType().isArray())require(Array.getLength(value)==actor._segments,"Explicit array input length must match actual segments");
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
                case "touchEffects2.BLUE":case "touchEffects2.YELLOW":case "touchEffects2.ROSE":
                case "touchEffects2.MINT":case "touchEffects2.LAVENDER":case "touchEffects2.MAGENTA":{
                    String[] arguments=argument.split(",");require(arguments.length==3&&arguments[0].equals("killer"),"Invalid cooperative contact arguments");
                    int segment=Integer.parseInt(arguments[1]),other=Integer.parseInt(arguments[2]);
                    require(parent._segColor[segment].equals(color(action.operation.substring("touchEffects2.".length()))),"Caller must enter the exact labelled cooperative case");
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
    private static Action tree(){return new Action("build_tree","members");}
    private static Color[] phenotype(String...names)throws Exception{Color[] colors=new Color[names.length];for(int i=0;i<names.length;i++)colors[i]=color(names[i]);return colors;}
    private static Spec scenario(String name,String weapon,String target)throws Exception{return new Spec(name,weapon,target);}
    private static List<Spec> canonical()throws Exception {
        List<Spec> result=new ArrayList<>();
        for(String target:COLORS)result.add(scenario("magenta-palette-"+target,"MAGENTA",target).set("actor:killer:_isinjured",true));
        for(String target:INTERNAL)result.add(scenario("magenta-palette-"+target,"MAGENTA",target).set("actor:killer:_isinjured",true));
        for(String target:new String[]{"UNKNOWN","TRANSLUCENT"})result.add(scenario("magenta-palette-"+target,"MAGENTA",target).set("actor:killer:_isinjured",true));
        for(String target:new String[]{"BROWN","BLUE","MINT","GREEN","UNKNOWN","TRANSLUCENT"})result.add(scenario("mint-baseline-"+target,"MINT",target));
        for(String target:new String[]{"WHITE","PLAGUE","CORAL","CREAM","FALLOW","FRUIT","BROWN","GREEN","LAVENDER","UNKNOWN","TRANSLUCENT"})result.add(scenario("lavender-palette-"+target,"LAVENDER",target).set("actor:victim:_lavender",10000).set("actor:victim:_createlavender",51));
        for(String target:new String[]{"BROWN","GREEN","UNKNOWN","TRANSLUCENT"})result.add(scenario("rose-baseline-"+target,"ROSE",target).set("actor:victim:_growthRatio",1).set("actor:killer:_energy",5d));
        for(String target:new String[]{"BROWN","BLUE","MINT","GREEN","UNKNOWN","TRANSLUCENT"})result.add(scenario("yellow-baseline-"+target,"YELLOW",target).set("actor:victim:_isinfectious",true).set("actor:killer:_isaplant",true));
        for(boolean active:new boolean[]{false,true})for(String target:new String[]{"RED","GREEN","CREAM"})result.add(scenario("blue-active-"+active+"-"+target,"BLUE",target).set("actor:victim:active",active));
        for(String weapon:new String[]{"BLUE","YELLOW","ROSE","MINT","LAVENDER","MAGENTA"})for(double energy:new double[]{0d,0.0000001d,Math.nextDown(0.0000001d),Double.NaN})result.add(scenario("death-"+weapon+"-"+Double.toHexString(energy),weapon,"BROWN").set("actor:killer:_energy",energy));
        for(String flag:new String[]{"_isaconsumer","_isaplant","_isafungus"})result.add(scenario("yellow-own-classification-blocks-"+flag,"YELLOW","GREEN").set("actor:victim:_isinfectious",true).set("actor:victim:"+flag,true).set("actor:killer:_isaplant",true));
        for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus"})result.add(scenario("yellow-host-countdown-"+flag,"YELLOW","GREEN").set("actor:victim:_isinfectious",true).set("actor:killer:"+flag,true).set("actor:victim:_timeToReproduce",0).set("actor:killer:_timeToReproduce",1).set("actor:killer:_timeToReproduceMax",23));
        for(String target:new String[]{"GREEN","BLUE"})for(boolean enhanced:new boolean[]{false,true}){
            Spec born=scenario("yellow-live-birth-"+target+"-"+enhanced,"YELLOW",target).set("actor:victim:_isinfectious",true).set("actor:victim:_yellowCounter",2)
                .set("actor:killer:_isaplant",true).set("actor:killer:_energy",1000d).set("actor:killer:_infectedGeneticCode","g0").set("actor:killer:_savedGeneticCode","g4").set("actor:killer:_antiviral",1)
                .set("actor:killer:_isenhanced",enhanced).setting("VIRUS_ENERGY_CONSUMPTION",0.125d).setting("BLUE_ENERGY_CONSUMPTION",1001d);
            result.add(born);
        }
        for(double energy:new double[]{0.125d,Math.nextUp(0.125d)})result.add(scenario("yellow-virus-energy-threshold-"+Double.toHexString(energy),"YELLOW","GREEN").set("actor:victim:_isinfectious",true).set("actor:victim:_energy",energy).set("actor:killer:_isaplant",true).set("actor:killer:_infectedGeneticCode","g0").setting("VIRUS_ENERGY_CONSUMPTION",0.125d));
        for(int children:new int[]{-2,-1,0,1,7,Integer.MAX_VALUE})result.add(scenario("yellow-child-count-"+children,"YELLOW","GREEN").set("actor:victim:_isinfectious",true).set("actor:victim:_yellowCounter",children).set("actor:killer:_isaplant",true).set("actor:killer:_infectedGeneticCode","g0").set("actor:killer:_energy",1000d));
        result.add(scenario("yellow-blocked-birth","YELLOW","GREEN").set("actor:victim:_isinfectious",true).set("actor:killer:_isaplant",true).set("actor:killer:_infectedGeneticCode","g0").set("actor:selector:bounds",new Rectangle(0,0,7000,7000)));
        result.add(scenario("yellow-inactive-half-threshold","YELLOW","GREEN").set("actor:victim:_isinfectious",true).set("actor:killer:_isaplant",true).set("actor:killer:active",false).set("actor:killer:_infectedGeneticCode","g0").set("actor:killer:_energy",30d).set("actor:killer:_reproduceEnergy",43));
        result.add(scenario("yellow-equal-valued-distinct-code","YELLOW","GREEN").set("actor:victim:_isinfectious",true).set("actor:killer:_isaplant",true).set("actor:killer:_infectedGeneticCode","g4"));
        result.add(scenario("yellow-blue-double-death","YELLOW","BLUE").set("actor:victim:_isinfectious",true).set("actor:victim:_energy",0.25d).set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true).set("actor:killer:_energy",0.25d).set("actor:killer:_m",new double[]{16d}).setting("BLUE_ENERGY_CONSUMPTION",0.25d));
        result.add(scenario("yellow-blue-partial-shield-oxygen","YELLOW","BLUE").set("actor:victim:_isinfectious",true).set("actor:killer:_isaplant",true).set("world:_O2",0.125d).setting("BLUE_ENERGY_CONSUMPTION",0.25d));
        result.add(scenario("yellow-mint-paid-neutralization","YELLOW","MINT").set("actor:victim:_isinfectious",true).setting("MINT_ENERGY_CONSUMPTION",2d));
        result.add(scenario("yellow-mint-partial-oxygen","YELLOW","MINT").set("actor:victim:_isinfectious",true).setting("MINT_ENERGY_CONSUMPTION",2d).set("world:_O2",0.125d));
        for(boolean actorAltruist:new boolean[]{false,true})for(boolean peerAltruist:new boolean[]{false,true})result.add(scenario("rose-altruists-"+actorAltruist+"-"+peerAltruist,"ROSE","GREEN").set("actor:victim:_growthRatio",1).set("actor:killer:_energy",5d).set("actor:victim:_altruist",actorAltruist).set("actor:killer:_altruist",peerAltruist));
        result.add(scenario("rose-integer-thresholds","ROSE","GREEN").set("actor:victim:_growthRatio",1).set("actor:victim:_reproduceEnergy",101).set("actor:killer:_reproduceEnergy",21).set("actor:victim:_energy",50.25d).set("actor:killer:_energy",9.75d));
        for(double energy:new double[]{7d,Math.nextUp(7d),Double.NaN,Double.POSITIVE_INFINITY})result.add(scenario("rose-difference-threshold-"+Double.toHexString(energy),"ROSE","GREEN").set("actor:victim:_growthRatio",1).set("actor:victim:_reproduceEnergy",0).set("actor:victim:_energy",energy).set("actor:killer:_energy",5d));
        for(String target:new String[]{"BROWN","MINT","GREEN"})for(boolean infectious:new boolean[]{false,true})result.add(scenario("mint-own-cure-"+target+"-"+infectious,"MINT",target).set("actor:killer:_isinfectious",infectious).set("actor:victim:_infectedGeneticCode","g1").set("actor:victim:_savedGeneticCode","g4"));
        for(String target:new String[]{"BLUE","GREEN"})for(String flag:new String[]{"_isinfectious","_iscoral","_plagueversion","_fallowversion"}){
            Object value=flag.endsWith("version")?Integer.valueOf(4):Boolean.TRUE;
            result.add(scenario("mint-neutralize-"+target+"-"+flag,"MINT",target).set("actor:killer:"+flag,value).set("actor:killer:_infectedGeneticCode","g3").set("actor:victim:_infectedGeneticCode","g1"));
        }
        for(boolean consumer:new boolean[]{false,true})for(boolean enhanced:new boolean[]{false,true})result.add(scenario("mint-blue-plant-gate-"+consumer+"-"+enhanced,"MINT","BLUE").set("actor:killer:_isaplant",true).set("actor:killer:_isinfectious",true).set("actor:victim:_isaconsumer",consumer).set("actor:victim:_isenhanced",enhanced).setting("BLUE_ENERGY_CONSUMPTION",0.25d));
        for(double oxygen:new double[]{0d,0.125d,1d,2d,Double.NaN})result.add(scenario("mint-separate-ally-payments-"+Double.toHexString(oxygen),"MINT","GREEN").set("actor:killer:_infectedGeneticCode","g3").set("actor:killer:_savedGeneticCode","g4").set("actor:killer:_reproducelate",-3).setting("MINT_ENERGY_CONSUMPTION",2d).set("world:_O2",oxygen));
        result.add(scenario("mint-retains-own-code-in-ally","MINT","GREEN").set("actor:killer:_infectedGeneticCode","g0").set("actor:killer:_savedGeneticCode","g4").set("actor:killer:_reproducelate",-3));
        for(int version:new int[]{1,2,5})for(boolean frozen:new boolean[]{false,true}){
            Spec spore=scenario("mint-inactive-spore-"+version+"-"+frozen,"MINT",version==5?"CREAM":"FRUIT").set("actor:killer:active",false).set("actor:killer:_sporeversion",version).set("actor:killer:_allfrozen",frozen).set("actor:killer:_isaconsumer",true)
                .set("actor:killer:_segColor",phenotype(version==5?"CREAM":"FRUIT","DARKFIRE","GREEN")).set("actor:killer:_infectedGeneticCode","g0").set("actor:victim:_infectedGeneticCode","g1").set("actor:victim:_savedGeneticCode","g4");
            spore.recipient.colors("GREEN","RED","YELLOW");result.add(spore);
        }
        for(int addition:new int[]{0,19,20,51,-21,Integer.MAX_VALUE})result.add(scenario("lavender-wrapped-timers-"+addition,"LAVENDER","GREEN").set("actor:victim:_lavender",10000).set("actor:victim:_createlavender",addition).set("actor:killer:_lavender",1).set("actor:victim:_timeToReproduce",0).set("actor:killer:_timeToReproduceMax",7).set("actor:killer:_timeToReproduce",100));
        for(boolean actorAltruist:new boolean[]{false,true})for(boolean peerAltruist:new boolean[]{false,true})result.add(scenario("lavender-altruists-"+actorAltruist+"-"+peerAltruist,"LAVENDER","GREEN").set("actor:victim:_lavender",10000).set("actor:victim:_createlavender",51).set("actor:victim:_altruist",actorAltruist).set("actor:killer:_altruist",peerAltruist));
        for(double oxygen:new double[]{0d,0.5d,1d,2d,2.5d,4d,Double.NaN}){
            Spec heal=scenario("magenta-partial-healing-"+Double.toHexString(oxygen),"MAGENTA","BROKEN").set("actor:killer:_segColor",phenotype("BROKEN","LIGHTBROWN")).set("actor:killer:_isinjured",true).set("actor:killer:_isfrozen",true).set("actor:killer:_allfrozen",true).setting("MAGENTA_ENERGY_CONSUMPTION",2d).set("world:_O2",oxygen);
            heal.recipient.colors("GREEN","RED");result.add(heal);
        }
        result.add(scenario("magenta-inactive-does-not-heal","MAGENTA","BROKEN").set("actor:killer:_isinjured",true).set("actor:killer:active",false));
        result.add(scenario("magenta-nonaltruist-does-not-heal","MAGENTA","BROKEN").set("actor:killer:_isinjured",true).set("actor:killer:_altruist",false));
        return result;
    }
    private static String capture()throws Exception {
        contact=Organism.class.getDeclaredMethod("touchEffects2",Organism.class,int.class,int.class);contact.setAccessible(true);
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception {
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.CooperativeOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        Files.writeString(directory.resolve("cooperative.tsv"),capture(),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("COOPERATIVE_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls+" newborns="+newborns);
    }
}
