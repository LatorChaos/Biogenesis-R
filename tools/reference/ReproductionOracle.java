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

/** Observe actual reproduction callers, transformation, and their world-owned side effects */
public final class ReproductionOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final int SEARCH_LIMIT=1024;
    private static Method grow,offset;
    private static int cases,snapshots,calls,searchAttempts;

    private static final class Code {
        String[] colors={"GREEN","CYAN","EYE"};
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
        final Code parent=new Code(),infected=new Code();
        final Map<String,Object> settings=new LinkedHashMap<>(),controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        Spec(String name){
            this.name=name;infected.colors("WHITE");infected.generation=80;infected.clade="virus";
            controls.put("actor:victim:_energy",10000d);
            controls.put("actor:victim:dx",1.25d);controls.put("actor:victim:dy",-0.75d);controls.put("actor:victim:dtheta",0.125d);
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
        final Map<String,Boolean> candidateResults=new LinkedHashMap<>();
        final World world;
        final Spec spec;
        final boolean observed;
        int newborns,candidates;
        boolean lastSuccess;
        Organism lastCandidate;
        Capture(Spec spec,String mode,boolean observed)throws Exception {
            this.spec=spec;this.observed=observed;
            for(Field field:Utils.class.getDeclaredFields())if(field.getName().startsWith("DEF_"))Utils.class.getDeclaredField(field.getName().substring(4)).set(null,field.get(null));
            Utils.INITIAL_ORGANISMS=0;Utils.INITIAL_O2=1000000;Utils.MIN_MUTATION_RATE=0;Utils.MAX_MUTATION_RATE=10000;Utils.META_MUTATION_RATE=0;
            for(String name:COLORS)Utils.class.getDeclaredField(name+"_PROB").setInt(null,name.equals("GREEN")?7:0);
            for(Map.Entry<String,Object> entry:spec.settings.entrySet())Utils.class.getDeclaredField(entry.getKey()).set(null,entry.getValue());
            Utils.random=new Random(827364);
            VisibleWorld visible=new VisibleWorld(null);world=new World(visible);
            world.addCO2(Utils.INITIAL_CO2);world.addO2(Utils.INITIAL_O2);world.addCH4(Utils.INITIAL_CH4);
            register("g0",genome(spec.parent));register("g1",genome(new Code().colors("GREEN")));register("g2",genome(new Code().colors("GREEN")));
            register("g3",genome(spec.infected));register("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,mode,827364);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            Organism selector=paste("selector","g2",1000,1000);visible._selectedOrganism=selector;
            row("I","setup","visible:selected","String","selector");world.addOrganism(selector,null);
            Organism victim=paste("victim","g0",3500,3500);world.addOrganism(victim,null);
            Organism killer=paste("killer","g1",2000,2000);world.addOrganism(killer,null);
            for(Map.Entry<String,Object> entry:spec.controls.entrySet())control("setup",entry.getKey(),entry.getValue());
            snapshot("initial");
        }
        void register(String label,GeneticCode code){require(code!=null&&!references.containsKey(code),"Duplicate genome recipe identity");genomes.put(label,code);references.put(code,label);}
        String reference(GeneticCode code){if(code==null)return "~";String label=references.get(code);if(label==null){label="g"+genomes.size();register(label,code);}return label;}
        void discover(String stage)throws Exception {
            for(Organism actor:world._organisms)if(!actorNames.containsKey(actor)){
                String label="newborn"+newborns++;actors.put(label,actor);actorNames.put(actor,label);row("D",stage,label,actor.getID(),reference(actor._geneticCode));
            }
            for(Organism actor:actors.values()){reference(actor._geneticCode);reference(actor._infectedGeneticCode);reference(actor._savedGeneticCode);}
            for(Field field:fields(WorldStatistics.class))if(field.getType()==GeneticCode.class)reference((GeneticCode)field.get(world.worldStatistics));
        }
        Organism paste(String role,String code,int x,int y)throws Exception {
            Organism actor=new Organism(world,genomes.get(code));require(actor.pasteOrganism(x,y),"Real actor paste failed: "+role);
            actors.put(role,actor);actorNames.put(actor,role);row("A",role,code,x,y);return actor;
        }
        void row(Object...values){if(!observed)return;for(int i=0;i<values.length;i++){if(i!=0)rows.append('\t');rows.append(values[i]);}rows.append('\n');}
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
            discover(stage);if(!observed)return;row("P",stage);
            IdentityHashMap<GeneticCode,Boolean> available=new IdentityHashMap<>();
            for(int i=0;i<5;i++)available.put(genomes.get("g"+i),true);
            for(Map.Entry<String,Organism> entry:actors.entrySet()){
                Organism actor=entry.getValue();
                if(!Boolean.FALSE.equals(candidateResults.get(entry.getKey()))){available.put(actor._geneticCode,true);available.put(actor._infectedGeneticCode,true);available.put(actor._savedGeneticCode,true);}
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
            Field field=owner.getClass().getDeclaredField(name);field.setAccessible(true);
            if(field.getType()==GeneticCode.class){field.set(owner,value.equals("~")?null:genomes.get((String)value));row("I",stage,target,"GeneticCode",value);}
            else {field.set(owner,value);row("I",stage,target,field.getType().getSimpleName(),scalar(value));}
        }
        void action(int index,Action action)throws Exception {
            String stage="step-"+index;
            for(Map.Entry<String,Object> entry:action.controls.entrySet())control(stage,entry.getKey(),entry.getValue());
            Organism victim=actors.get("victim");row("K",stage,action.operation,"victim",action.argument);
            switch(action.operation){
                case "reproduce":victim.reproduce();break;case "reproduceEarly":victim.reproduceEarly();break;
                case "reproduceEarlyPlant":victim.reproduceEarlyPlant();break;case "reproduceEarlyVirus":victim.reproduceEarlyVirus();break;
                case "reproduceSpore":victim.reproduceSpore();break;case "reproduceYellow":victim.reproduceYellow();break;
                case "reproduceVirus":victim.reproduceVirus();break;case "reproduceforeignVirus":victim.reproduceforeignVirus();break;
                case "transform":victim.transform();break;
                case "inheritTransformation":{
                    Organism child=new Organism(world);String name="candidate"+candidates++;
                    actors.put(name,child);actorNames.put(child,name);lastSuccess=child.inheritTransformation(victim,Boolean.parseBoolean(action.argument));lastCandidate=child;
                    candidateResults.put(name,lastSuccess);row("B",stage,"inheritTransformation",lastSuccess);row("Z",stage,name,lastSuccess?"available":"rejected");break;
                }
                case "build_tree":{
                    world.colDetTree=new STRtree();if(!action.argument.equals("~"))for(String name:action.argument.split(",")){
                        Organism actor=actors.get(name);world.colDetTree.insert(new Envelope(actor.getX(),actor.getMaxX(),actor.getY(),actor.getMaxY()),actor);
                        row("J",stage,name,actor.x,actor.y,actor.width,actor.height);
                    }world.colDetTree.build();break;
                }
                case "blueSpore":victim.blueSpore();break;case "cyanSpore":victim.cyanSpore();break;
                case "grow":grow.invoke(victim);break;
                case "offset_velocity":offset.invoke(victim,victim.dx,victim.dy,victim.dtheta);break;
                case "bounds":victim.calculateBounds(Boolean.parseBoolean(action.argument));break;
                default:throw new IllegalArgumentException("Unknown original operation: "+action.operation);
            }
            if(observed)calls++;snapshot(stage);
        }
        String finish(){if(observed)cases++;row("E");return rows.toString();}
    }
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Random copyRandom(Random random)throws Exception {ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception {
        switch(name){case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static GeneticCode genome(Code recipe)throws Exception {
        List<Gene> genes=new ArrayList<>();for(int index=0;index<recipe.colors.length;index++){
            Gene gene=new Gene();gene.setLength(recipe.length);gene.setTheta(index*0.25);gene.setBranch(0);gene.setColor(color(recipe.colors[index]));genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,recipe.symmetry,recipe.mirror,recipe.rate,0,1,1,recipe.spore,recipe.adapt,recipe.plague,recipe.disperse,false,false,false,false,false,false,false,false,false,false,false,false,recipe.selfish);
        code._generation=recipe.generation;code._cladeID=recipe.clade;return code;
    }
    private static Action call(String operation){return new Action(operation,"~");}
    private static Action tree(){return new Action("build_tree","selector,victim,killer");}
    private static Spec infection(Spec spec){return spec.set("actor:victim:_infectedGeneticCode","g3").set("actor:victim:_infectedReproduceEnergy",101);}
    private static Spec blocked(Spec spec){return spec.set("actor:killer:bounds",new Rectangle(0,0,7000,7000));}
    private static Capture execute(Spec spec,String mode,boolean observed)throws Exception {
        Capture capture=new Capture(spec,mode,observed);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index));return capture;
    }
    private static void adopt(StringBuilder output,Spec spec,String mode)throws Exception {output.append(execute(spec,mode,true).finish());}
    private static long search(Spec spec,String mode,java.util.function.Predicate<Capture> predicate)throws Exception {
        Action action=spec.actions.get(spec.actions.size()-1);
        for(long seed=0;seed<SEARCH_LIMIT;seed++){
            action.set("rng:seed",seed);Capture actual=execute(spec,mode,false);searchAttempts++;
            if(predicate.test(actual))return seed;
        }
        throw new IllegalStateException("Actual-source bounded search exhausted: "+spec.name);
    }

    private static Spec caller(String name,String operation){return new Spec(name).add(tree()).add(call(operation));}
    private static Spec thresholds(String name,int total,boolean consumer){
        return blocked(caller(name,"reproduce")).set("actor:victim:_issilver",true).set("actor:victim:_isaconsumer",consumer)
            .set("actor:victim:_isafungus",false).set("actor:victim:_isinfectious",false).set("actor:victim:_nTotalChildren",total);
    }
    private static String reproduction()throws Exception {
        StringBuilder output=new StringBuilder();
        String[] methods={"reproduce","reproduceEarly","reproduceEarlyPlant","reproduceEarlyVirus","reproduceSpore","reproduceYellow","reproduceVirus","reproduceforeignVirus"};
        for(String method:methods){
            Spec success=caller(method+"-success",method),failure=blocked(caller(method+"-blocked",method));
            if(method.equals("reproduceYellow")||method.equals("reproduceVirus")||method.equals("reproduceforeignVirus")){infection(success);infection(failure);success.set("actor:victim:_nVirusChildren",4);failure.set("actor:victim:_nVirusChildren",4);}
            adopt(output,success,"reproduction");adopt(output,failure,"reproduction");
        }
        for(String method:new String[]{"reproduce","reproduceEarly","reproduceEarlyPlant","reproduceYellow"}){
            Spec multi=caller(method+"-eight",method).set("actor:victim:_yellowCounter",7).set("actor:victim:_nVirusChildren",8);
            if(method.equals("reproduceYellow"))infection(multi);
            adopt(output,multi,"reproduction");
            Spec partial=caller(method+"-partial-placement",method).set("actor:victim:_yellowCounter",7).set("actor:victim:_nVirusChildren",8)
                .set("actor:killer:bounds",new Rectangle(0,0,7000,3500));
            if(method.equals("reproduceYellow"))infection(partial);
            adopt(output,partial,"reproduction");
        }
        for(int total:new int[]{-1,0,1,9,36,100,225,441,783,784,Integer.MAX_VALUE})adopt(output,thresholds("silver-consumer-"+total,total,true),"reproduction");
        for(int total:new int[]{-1,1,2,5,9,14,20,26,27,Integer.MAX_VALUE})adopt(output,thresholds("silver-plant-"+total,total,false),"reproduction");
        adopt(output,thresholds("silver-fungus",36,false).set("actor:victim:_isafungus",true),"reproduction");
        adopt(output,thresholds("silver-infectious",100,false).set("actor:victim:_isinfectious",true),"reproduction");
        adopt(output,caller("children-counter-wrap-before-statistics","reproduce").set("actor:victim:_nTotalChildren",Integer.MAX_VALUE).set("statistics:time",7L),"reproduction");
        adopt(output,blocked(caller("yellow-partial-payment","reproduce")).setting("YELLOW_ENERGY_CONSUMPTION",0.75d)
            .set("actor:victim:_yellowCounter",3).set("world:_O2",0.25d),"reproduction");
        adopt(output,infection(blocked(caller("auburn-partial-payment","reproduce"))).setting("AUBURN_ENERGY_CONSUMPTION",1.25d)
            .set("actor:victim:_isauburn",true).set("world:_O2",0.25d),"reproduction");
        adopt(output,infection(blocked(caller("ordered-yellow-auburn-payments","reproduce"))).setting("YELLOW_ENERGY_CONSUMPTION",0.75d).setting("AUBURN_ENERGY_CONSUMPTION",1.25d)
            .set("actor:victim:_yellowCounter",3).set("actor:victim:_isauburn",true).set("world:_O2",1d),"reproduction");
        adopt(output,caller("blond-once-after-eight-births","reproduceEarly").setting("BLOND_ENERGY_CONSUMPTION",4.5d)
            .set("actor:victim:_isblond",true).set("actor:victim:_yellowCounter",7).set("world:_O2",0.25d),"reproduction");
        for(String kind:new String[]{"ordinary","c4","late"}){
            Spec plant=caller("early-plant-blond-"+kind,"reproduceEarlyPlant").setting("BLOND_ENERGY_CONSUMPTION",4.5d)
                .set("actor:victim:_isblond",true).set("actor:victim:_earlyReproduceEnergy",31).set("actor:victim:_reproduceEnergy",101);
            if(kind.equals("c4"))plant.set("actor:victim:_isonlyc4",true);if(kind.equals("late"))plant.set("actor:victim:_reproducelate",1);
            adopt(output,plant,"reproduction");
        }
        adopt(output,caller("early-plant-wrapped-energy-difference","reproduceEarlyPlant").setting("BLOND_ENERGY_CONSUMPTION",0.5d)
            .set("actor:victim:_isblond",true).set("actor:victim:_energy",1e12d).set("actor:victim:_reproduceEnergy",Integer.MIN_VALUE).set("actor:victim:_earlyReproduceEnergy",Integer.MAX_VALUE),"reproduction");
        adopt(output,caller("early-virus-blond-and-child-cache","reproduceEarlyVirus").setting("BLOND_ENERGY_CONSUMPTION",4.5d)
            .set("actor:victim:_isblond",true).set("world:_O2",0.25d),"reproduction");
        adopt(output,caller("early-virus-version-five-clears-cache","reproduceEarlyVirus").set("actor:victim:_sporeversion",5).set("actor:victim:_sporetime",100)
            .set("actor:victim:_savedGeneticCode","g4"),"reproduction");
        for(int modifier:new int[]{9,12}){
            Spec forced=new Spec("spore-caller-forces-dormancy-"+modifier);forced.parent.colors("GREEN","SPORE","EYE");forced.parent.spore=modifier;forced.parent.adapt=70;
            forced.add(call(modifier==9?"blueSpore":"cyanSpore")).add(tree()).add(call("reproduceSpore").set("actor:victim:_infectedGeneticCode","g3").set("actor:victim:_savedGeneticCode","g4"));
            adopt(output,forced,"reproduction");
        }
        adopt(output,blocked(caller("spore-blocked-keeps-uninfected-saved-code","reproduceSpore")).set("actor:victim:_savedGeneticCode","g4"),"reproduction");
        adopt(output,caller("yellow-without-infection-still-pays","reproduceYellow").setting("YELLOW_ENERGY_CONSUMPTION",0.75d).setting("AUBURN_ENERGY_CONSUMPTION",1.25d)
            .set("actor:victim:_isauburn",true).set("actor:victim:_yellowCounter",3).set("actor:victim:_nVirusChildren",8).set("actor:victim:_timeToReproduce",999),"reproduction");
        adopt(output,infection(caller("virus-compound-positive-fraction","reproduceVirus")).set("actor:victim:_yellowCounter",3),"reproduction");
        Spec compound=infection(caller("virus-compound-int-saturation","reproduceVirus")).set("actor:victim:_yellowCounter",Integer.MAX_VALUE);adopt(output,compound,"reproduction");
        Spec foreign=infection(new Spec("foreign-virus-once-cost-and-retry")).setting("AUBURN_ENERGY_CONSUMPTION",1.25d).set("actor:victim:_isauburn",true);
        foreign.add(tree()).add(call("reproduceforeignVirus")).add(call("reproduceforeignVirus").set("actor:victim:_infectedGeneticCode","g3"));adopt(output,foreign,"reproduction");
        Spec dynamic=infection(caller("dynamic-self-inherit-loop-limit","reproduce")).set("actor:victim:_isauburn",true).set("actor:victim:_yellowCounter",5);
        dynamic.parent.colors("GREEN","SPORE","EYE");dynamic.parent.spore=1;dynamic.parent.length=120;dynamic.actions.set(0,new Action("build_tree","~"));
        search(dynamic,"reproduction",actual->actual.newborns>1&&actual.newborns==actual.actors.get("victim")._nChildren&&actual.world._organisms.get(3)._geneticCode==actual.actors.get("victim")._geneticCode);
        adopt(output,dynamic,"reproduction");
        Spec shared=caller("ordinary-spore-shares-whole-parent-genotype","reproduce");shared.parent.colors("GREEN","SPORE","EYE");shared.parent.spore=2;shared.parent.length=120;
        shared.set("genome:g0:_updateClade",true);adopt(output,shared,"reproduction");
        for(int version=1;version<=6;version++){
            Spec mismatch=infection(caller("ordinary-spore-guard-mismatch-"+version,"reproduce"));mismatch.infected.colors("GREEN","SPORE","EYE");mismatch.infected.spore=version;mismatch.infected.length=120;
            adopt(output,mismatch,"reproduction");
        }
        return output.toString();
    }
    private static Spec transformed(String name,boolean primitive,boolean first){
        return infection(new Spec(name)).set("actor:victim:_savedGeneticCode","g4").add(tree())
            .add(primitive?new Action("inheritTransformation",Boolean.toString(first)):call("transform"));
    }
    private static Spec centerBlocked(Spec spec){return spec.set("actor:killer:bounds",new Rectangle(3499,3499,2,2));}
    private static String transformation()throws Exception {
        StringBuilder output=new StringBuilder();
        adopt(output,transformed("wrapper-center-success",false,true),"transformation");
        adopt(output,transformed("direct-center-success",true,true),"transformation");
        adopt(output,transformed("direct-first-false",true,false),"transformation");
        adopt(output,blocked(transformed("direct-all-positions-blocked",true,true)),"transformation");
        adopt(output,blocked(transformed("wrapper-all-positions-blocked",false,true)),"transformation");
        adopt(output,centerBlocked(transformed("default-zero-parent-exemption",true,true)).set("actor:killer:_parentID",0),"transformation");
        adopt(output,centerBlocked(transformed("equal-id-is-not-object-identity",true,true)).set("actor:killer:_ID",0),"transformation");
        adopt(output,centerBlocked(transformed("wrapped-parent-id-exemption",true,true)).set("actor:victim:_ID",Integer.MIN_VALUE).set("actor:killer:_ID",Integer.MIN_VALUE),"transformation");
        for(int sx=-1;sx<=1;sx++)for(int sy=-1;sy<=1;sy++)if(sx!=0||sy!=0){
            final int x=sx,y=sy;Spec side=centerBlocked(transformed("fallback-direction-"+sx+"-"+sy,true,true));
            search(side,"transformation",actual->actual.lastSuccess&&Integer.signum(actual.lastCandidate._centerX-actual.actors.get("victim")._centerX)==x&&Integer.signum(actual.lastCandidate._centerY-actual.actors.get("victim")._centerY)==y);
            adopt(output,side,"transformation");
        }
        Spec wrap=transformed("allocated-id-wrapping",false,true).set("world:NEXT_ID",Integer.MAX_VALUE);
        wrap.add(call("transform").set("actor:victim:_infectedGeneticCode","g3").set("actor:victim:_energy",10000d));adopt(output,wrap,"transformation");
        for(int modifier=1;modifier<=12;modifier++){
            Spec spore=transformed("transformation-spore-"+modifier,true,true);spore.parent.disperse=true;spore.infected.colors("GREEN","SPORE","EYE");spore.infected.spore=modifier;spore.infected.length=120;
            adopt(output,spore,"transformation");
        }
        for(int adapt:new int[]{20,30,40,50,60,70}){
            for(boolean dormant:new boolean[]{true,false}){
                if(adapt==20&&!dormant||adapt==70&&dormant)continue;
                Spec adaptive=transformed("adaptation-"+adapt+"-"+dormant,true,true);adaptive.infected.colors("GREEN","SPORE","EYE");adaptive.infected.spore=7;adaptive.infected.adapt=adapt;adaptive.infected.length=120;
                search(adaptive,"transformation",actual->actual.lastSuccess&&(!actual.lastCandidate.active)==dormant);adopt(output,adaptive,"transformation");
            }
        }
        for(String kind:new String[]{"increase-genes","decrease-genes","increase-symmetry","decrease-symmetry"}){
            Spec lineage=transformed("lineage-"+kind,true,true);lineage.infected.colors("GREEN","GREEN","GREEN","GREEN");lineage.infected.symmetry=4;lineage.infected.rate=5000;
            long seed=search(lineage,"transformation",actual->{
                if(!actual.lastSuccess)return false;GeneticCode child=actual.lastCandidate._geneticCode,parent=actual.genomes.get("g3");
                switch(kind){case "increase-genes":return child.getNGenes()>parent.getNGenes();case "decrease-genes":return child.getNGenes()<parent.getNGenes();case "increase-symmetry":return child.getSymmetry()>parent.getSymmetry();case "decrease-symmetry":return child.getSymmetry()<parent.getSymmetry();default:return child.getNGenes()==parent.getNGenes()&&child.getSymmetry()==parent.getSymmetry();}
            });
            adopt(output,lineage,"transformation");
            if(kind.equals("increase-genes"))for(int limit:new int[]{0,1,2}){
                Spec capped=transformed("lineage-marker-limit-"+limit,true,true);capped.infected.colors("GREEN","GREEN","GREEN","GREEN");capped.infected.symmetry=4;capped.infected.rate=5000;capped.infected.clade="virus+3";capped.setting("CLADE_COMPLEXITY",limit);
                capped.actions.get(capped.actions.size()-1).set("rng:seed",seed);adopt(output,capped,"transformation");
            }
        }
        Spec unchanged=transformed("lineage-equal-structure-redraw",true,false);unchanged.infected.colors("GREEN","GREEN","GREEN","GREEN");unchanged.infected.symmetry=4;unchanged.infected.rate=5000;
        long unchangedSeed=search(unchanged,"transformation",actual->{GeneticCode child=actual.lastCandidate._geneticCode,parent=actual.genomes.get("g3");return !actual.lastSuccess&&child.getUpdateClade()&&child.getNGenes()==parent.getNGenes()&&child.getSymmetry()==parent.getSymmetry();});
        unchanged.add(new Action("inheritTransformation","true").set("rng:seed",unchangedSeed));adopt(output,unchanged,"transformation");
        Spec nonfinite=infection(new Spec("generated-nan-failed-candidate-world-event"));nonfinite.parent.colors("GREEN");nonfinite.parent.length=0;
        nonfinite.add(tree()).add(call("grow").set("actor:victim:_age",7).set("actor:victim:_energy",0d)).add(call("offset_velocity")).add(new Action("bounds","true")).add(new Action("inheritTransformation","true"));
        adopt(output,nonfinite,"transformation");
        return output.toString();
    }
    public static void main(String[]args)throws Exception {
        if(args.length!=2||!List.of("reproduction","transformation").contains(args[1]))throw new IllegalArgumentException("Usage: biogenesis.ReproductionOracle NEW_OUTPUT_DIRECTORY reproduction|transformation");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        grow=Organism.class.getDeclaredMethod("grow");grow.setAccessible(true);offset=Organism.class.getDeclaredMethod("offset",double.class,double.class,double.class);offset.setAccessible(true);
        String text=args[1].equals("reproduction")?reproduction():transformation();
        Files.writeString(directory.resolve(args[1]+".tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("REPRODUCTION_ORACLE PASS mode="+args[1]+" cases="+cases+" snapshots="+snapshots+" calls="+calls+" bounded_search_attempts="+searchAttempts);
    }
}
