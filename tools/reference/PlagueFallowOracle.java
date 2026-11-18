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

/** Observe two original touchEffects2 branches with actual held-world reproduction */
public final class PlagueFallowOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static int cases,snapshots,calls,newborns;
    private static Method contact;
    private static final class Code {
        String[] colors={"GREEN"};
        int symmetry=1,mirror,rate,fallow=1,spore=1,adapt=1,generation=41;
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
            require(List.of("PLAGUE","FALLOW").contains(weapon),"Unsupported PLAGUE/FALLOW actor");this.name=name;this.weapon=weapon;parent.colors(weapon);
            for(String role:new String[]{"victim","killer"}) {
                controls.put("actor:"+role+":_energy",100d);
                controls.put("actor:"+role+":active",true);
                controls.put("actor:"+role+":_framesColor",0);
                for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus","_isenhanced","_isinfectious","_iscoral","_isgray","_isinjured","_isfrozen","_allfrozen","_altruist","_isauburn","_issilver","_isblond","_isakiller","_isonlyc4","_isreproductive","_dodge","_hasdodged","_transfersenergy"})controls.put("actor:"+role+":"+flag,false);
                for(String field:new String[]{"_plagueversion","_fallowversion","_sporeversion","_antiviral","_lavender","_createlavender","_indigo","_jadefactor","_nVirusChildren","_nTotalInfected","_sporetime","_reproducelate"})controls.put("actor:"+role+":"+field,0);
            }
            controls.put("actor:killer:_segColor",phenotype(target));
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
            row("C",spec.name,"plague-fallow",spec.seed);
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
                case "touchEffects2.PLAGUE":case "touchEffects2.FALLOW":{
                    String[] arguments=argument.split(",");require(arguments.length==3&&arguments[0].equals("killer"),"Invalid PLAGUE/FALLOW contact arguments");
                    int segment=Integer.parseInt(arguments[1]),other=Integer.parseInt(arguments[2]);
                    require(parent._segColor[segment].equals(color(action.operation.substring("touchEffects2.".length()))),"Caller must enter the exact labelled PLAGUE/FALLOW case");
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
        GeneticCode code=new GeneticCode(genes,recipe.symmetry,recipe.mirror,recipe.rate,0,1,recipe.fallow,recipe.spore,recipe.adapt,recipe.plague,recipe.disperse,false,false,false,false,false,false,false,false,false,false,false,false,recipe.selfish);
        code._generation=recipe.generation;code._cladeID=recipe.clade;return code;
    }
    private static Action tree(){return new Action("build_tree","members");}
    private static Color[] phenotype(String...names)throws Exception{Color[] colors=new Color[names.length];for(int i=0;i<names.length;i++)colors[i]=color(names[i]);return colors;}
    private static Spec scenario(String name,String weapon,String target)throws Exception{
        Spec spec=new Spec(name,weapon,target);
        spec.setting("PLAGUE_ENERGY_CONSUMPTION",1.25d).setting("SCOURGE_ENERGY_CONSUMPTION",0.25d).setting("FALLOW_ENERGY_CONSUMPTION",0.125d)
            .setting("DODGE_ENERGY_CONSUMPTION",0.5d).setting("BLUE_ENERGY_CONSUMPTION",0.75d).setting("MAGENTA_ENERGY_CONSUMPTION",0.375d);
        spec.set("actor:killer:_timeToReproduce",0).set("actor:killer:_timeToReproduceMax",12);
        spec.set("actor:killer:_energy",1000d).set("actor:killer:_reproduceEnergy",43).set("actor:killer:_infectedReproduceEnergy",43);
        spec.set("actor:killer:_savedGeneticCode","g4");
        if(weapon.equals("PLAGUE")){
            spec.set("actor:victim:_isinfectious",true).set("actor:victim:_isaplant",true).set("actor:victim:_plagueversion",1);
            spec.set("actor:killer:_isaplant",true).set("actor:killer:_infectedGeneticCode","g0").set("actor:killer:_isinfectious",true);
        }else{
            spec.parent.fallow=2;
            spec.set("actor:victim:_fallowversion",2).set("actor:killer:_fallowversion",0).set("actor:killer:_isaplant",true);
        }
        return spec;
    }
    private static Spec blocked(Spec spec){return spec.set("actor:selector:bounds",new Rectangle(0,0,7000,7000));}
    private static Spec foreign(Spec spec){return spec.set("actor:victim:_isinfectious",false).set("actor:killer:_infectedGeneticCode","g3");}
    private static Spec version(Spec spec,int version){spec.parent.fallow=version;return spec.set("actor:victim:_fallowversion",version);}
    private static Action touch(Spec spec){return new Action("touchEffects2."+spec.weapon,"killer,0,0");}
    private static Spec plagueVersion(Spec spec,int value){spec.parent.plague=value==2;return spec.set("actor:victim:_plagueversion",value);}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        List<String> palette=new ArrayList<>(Arrays.asList(COLORS));palette.addAll(Arrays.asList(INTERNAL));palette.addAll(List.of("UNKNOWN","TRANSLUCENT"));
        require(palette.size()==66,"Palette inventory drift");
        for(String target:palette)result.add(scenario("plague-palette-"+target,"PLAGUE",target));
        for(String target:new String[]{"FALLOW","WHITE","JADE","GRASS","GREEN","DARKJADE","DARKGREEN","ROSE","MAGENTA","BLUE","RED","LAVENDER","MINT","CREAM","SPIKEPOINT","OLIVE","FLOWER","DARKGRAY","CORAL","FRUIT","BROWN","VISION","BARK","OLDBARK","UNKNOWN","TRANSLUCENT"})result.add(scenario("fallow-group-"+target,"FALLOW",target));

        for(int variant:new int[]{0,1,2,3})for(String classification:new String[]{"plant","consumer","fungus","plain"}){
            Spec spec=plagueVersion(scenario("plague-version-class-"+variant+"-"+classification,"PLAGUE","UNKNOWN"),variant).set("actor:killer:_isaplant",false);
            if(!classification.equals("plain"))spec.set("actor:killer:"+(classification.equals("plant")?"_isaplant":classification.equals("consumer")?"_isaconsumer":"_isafungus"),true);
            result.add(spec);
        }
        for(String identity:new String[]{"g0","g3","g4","~"})for(boolean infectious:new boolean[]{false,true})result.add(scenario("plague-identity-"+identity+"-infectious-"+infectious,"PLAGUE","GREEN").set("actor:killer:_infectedGeneticCode",identity).set("actor:victim:_isinfectious",infectious));
        for(String peer:new String[]{"WHITE","PLAGUE"})for(boolean infectious:new boolean[]{false,true})result.add(scenario("plague-peer-infectious-"+peer+"-"+infectious,"PLAGUE",peer).set("actor:killer:_isinfectious",infectious));
        for(int count:new int[]{9,10,11})result.add(scenario("plague-silver-infection-count-"+count,"PLAGUE","SILVER").set("actor:victim:_nTotalInfected",10).set("actor:killer:_nTotalInfected",count));
        result.add(scenario("plague-silver-enhancement-bypasses-count","PLAGUE","SILVER").set("actor:victim:_nTotalInfected",-1).set("actor:killer:_nTotalInfected",Integer.MAX_VALUE).set("actor:killer:_isenhanced",true));
        for(String peer:new String[]{"MAROON","ORANGE","FIRE","RED","CREAM","PINK","SPIKE","SPIKEPOINT","SPORE","CYAN","MAGENTA","UNKNOWN"})result.add(plagueVersion(scenario("plague-consumer-group-"+peer,"PLAGUE",peer),2).set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",true).set("actor:killer:_isenhanced",true).set("actor:killer:_sporeversion",1).set("genome:g1:_modifiespink",true));
        for(String peer:new String[]{"SKY","DEEPSKY","FALLOW","RED","CREAM"})for(boolean enhanced:new boolean[]{false,true})result.add(scenario("plague-protective-enhancement-"+peer+"-"+enhanced,"PLAGUE",peer).set("actor:victim:_isenhanced",enhanced));
        for(String peer:new String[]{"SKY","RED"})result.add(scenario("plague-create-lavender-blocks-"+peer,"PLAGUE",peer).set("actor:victim:_isenhanced",true).set("actor:killer:_createlavender",1));
        result.add(scenario("plague-pink-pure-auburn-bypass","PLAGUE","PINK").set("actor:victim:_isauburn",true).set("actor:victim:_isaplant",false));
        result.add(scenario("plague-spike-plain-peer-protected","PLAGUE","SPIKE").set("actor:killer:_isaplant",false));
        for(String peer:new String[]{"GREEN","JADE","DARKJADE","GRASS"})result.add(scenario("plague-jade-factor-"+peer,"PLAGUE",peer).set("actor:killer:_jadefactor",2).set("actor:killer:_antiviral",1));
        for(String peer:new String[]{"CYAN","GREEN","SPORE","MAGENTA"})result.add(scenario("plague-paid-dodge-"+peer,"PLAGUE",peer).set("actor:killer:_dodge",true).set("actor:killer:_isaconsumer",true).set("actor:killer:_sporeversion",1));
        result.add(scenario("plague-partial-dodge","PLAGUE","GREEN").set("actor:killer:_dodge",true).set("world:_O2",0.25d));
        result.add(scenario("plague-altruist-magenta","PLAGUE","MAGENTA").set("actor:victim:_altruist",true));
        for(int variant:new int[]{0,1,6})result.add(plagueVersion(scenario("plague-spore-consumer-version-"+variant,"PLAGUE","SPORE"),2).set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",true).set("actor:killer:_sporeversion",variant));
        result.add(scenario("plague-plant-spore-only-tints","PLAGUE","SPORE").set("actor:killer:_sporeversion",6));
        for(int shield:new int[]{184,185,186})result.add(scenario("plague-normal-shield-"+shield,"PLAGUE","GREEN").set("actor:killer:_lavender",shield));
        for(int shield:new int[]{2219,2220,2221})result.add(foreign(scenario("plague-weak-shield-"+shield,"PLAGUE","GREEN")).set("actor:victim:_isenhanced",true).set("actor:killer:_lavender",shield));
        result.add(foreign(scenario("plague-enhanced-consumer-uses-normal-shield","PLAGUE","GREEN")).set("actor:victim:_isenhanced",true).set("actor:victim:_isaconsumer",true).set("actor:killer:_lavender",186));
        for(boolean plant:new boolean[]{false,true})for(double energy:new double[]{0.25d,Math.nextUp(0.25d),1.25d,Math.nextUp(1.25d)})result.add(scenario("plague-energy-gate-"+plant+"-"+bits(energy),"PLAGUE","GREEN").set("actor:victim:_isaplant",plant).set("actor:victim:_energy",energy));
        result.add(foreign(scenario("plague-foreign-birth","PLAGUE","GREEN")));
        result.add(blocked(scenario("plague-blocked-viral-birth","PLAGUE","GREEN")));
        result.add(blocked(foreign(scenario("plague-blocked-foreign-birth","PLAGUE","GREEN"))));
        for(int marker:new int[]{-1,1,Integer.MIN_VALUE})for(boolean infectious:new boolean[]{false,true})result.add(scenario("plague-stale-marker-"+marker+"-"+infectious,"PLAGUE","BROWN").set("actor:killer:_nVirusChildren",marker).set("actor:victim:_isinfectious",infectious).set("actor:killer:_antiviral",1));
        result.add(scenario("plague-stale-marker-failed-post-payment","PLAGUE","BROWN").set("actor:killer:_nVirusChildren",1).set("actor:victim:_energy",0d).set("actor:killer:_antiviral",1));
        for(double oxygen:new double[]{0d,0.125d,0.625d,Double.longBitsToDouble(0xfff8000000000042L)})result.add(scenario("plague-birth-post-payment-oxygen-"+bits(oxygen),"PLAGUE","GREEN").set("world:_O2",oxygen).set("actor:killer:_antiviral",1));
        for(boolean infectious:new boolean[]{false,true})for(boolean plant:new boolean[]{false,true})result.add(scenario("plague-indigo-cost-"+infectious+"-"+plant,"PLAGUE","INDIGO").set("actor:victim:_isinfectious",infectious).set("actor:victim:_isaplant",plant).set("actor:killer:_isaplant",false).set("actor:killer:_antiviral",1));
        result.add(scenario("plague-own-indigo-blocks-ordinary-cost","PLAGUE","INDIGO").set("actor:victim:_isaplant",false).set("actor:victim:_indigo",1));
        for(double coefficient:new double[]{0d,0.5d})result.add(scenario("plague-source-indigo-global-"+bits(coefficient),"PLAGUE","INDIGO").setting("INDIGO_ENERGY_CONSUMPTION",coefficient));
        result.add(scenario("plague-blue-full-shield","PLAGUE","BLUE"));
        result.add(scenario("plague-blue-insufficient-energy","PLAGUE","BLUE").set("actor:killer:_energy",0.5d));
        result.add(scenario("plague-blue-partial-oxygen","PLAGUE","BLUE").set("world:_O2",0.375d));
        result.add(scenario("plague-blue-mutual-death","PLAGUE","BLUE").set("actor:killer:_isenhanced",true).set("actor:victim:_energy",0.25d).set("actor:killer:_energy",0.75d));
        for(boolean enhanced:new boolean[]{false,true})result.add(scenario("plague-lavender-refill-"+enhanced,"PLAGUE","LAVENDER").set("actor:killer:_isenhanced",enhanced).set("actor:killer:_infectedGeneticCode","g0"));
        result.add(scenario("plague-lavender-keeps-foreign-infection","PLAGUE","LAVENDER").set("actor:killer:_infectedGeneticCode","g3"));
        result.add(scenario("plague-lavender-failed-payment-still-refills","PLAGUE","LAVENDER").set("world:_O2",0d));
        result.add(scenario("plague-lavender-source-zero-global","PLAGUE","LAVENDER").setting("LAVENDER_ENERGY_CONSUMPTION",0d));
        result.add(scenario("plague-lavender-wrapped-shield","PLAGUE","LAVENDER").setting("LAVENDER_SHIELD",Integer.MAX_VALUE).set("actor:killer:_lavender",Integer.MAX_VALUE-500).set("actor:killer:_isenhanced",true));
        for(int next:new int[]{Integer.MAX_VALUE,1,-1})result.add(scenario("plague-newborn-id-"+next,"PLAGUE","GREEN").set("world:NEXT_ID",next));
        result.add(scenario("plague-total-children-wrap","PLAGUE","GREEN").set("actor:killer:_nTotalChildren",Integer.MAX_VALUE));
        Spec shared=scenario("plague-shared-gene-edit-and-real-tree-rebuild","PLAGUE","GREEN");shared.add(tree().set("genome:g0:gene0._length",12.5d).set("genome:g0:_cladeID","plague-edit"));result.add(shared);
        Spec mutated=scenario("plague-full-mutation-birth","PLAGUE","GREEN");mutated.parent.rate=10000;result.add(mutated);

        for(int value:new int[]{-1,0,1,2,3,4,5})result.add(version(scenario("fallow-version-"+value,"FALLOW","WHITE"),value));
        for(String reason:new String[]{"killer","blond","neither"}){
            Spec spec=version(scenario("fallow-peer-fallow-"+reason,"FALLOW","FALLOW"),2);
            if(!reason.equals("neither"))spec.set("actor:killer:"+(reason.equals("killer")?"_isakiller":"_isblond"),true);result.add(spec);
        }
        for(String bypass:new String[]{"none","infectious","enhanced","pure"}){
            Spec spec=scenario("fallow-same-version-gate-"+bypass,"FALLOW","WHITE").set("actor:victim:_isaconsumer",true).set("actor:killer:_fallowversion",2);
            if(bypass.equals("infectious"))spec.set("actor:killer:_isinfectious",true);
            if(bypass.equals("enhanced"))spec.set("actor:victim:_isenhanced",true);
            if(bypass.equals("pure"))spec.set("actor:victim:_isaconsumer",false);result.add(spec);
        }
        for(boolean plant:new boolean[]{false,true})for(int antiviral:new int[]{0,1})result.add(scenario("fallow-fungus-classification-"+plant+"-"+antiviral,"FALLOW","WHITE").set("actor:killer:_isaplant",false).set("actor:killer:_isafungus",true).set("actor:victim:_isaplant",plant).set("actor:killer:_antiviral",antiviral));
        for(String code:new String[]{"g0","g3","g4","~"})result.add(scenario("fallow-infection-identity-"+code,"FALLOW","WHITE").set("actor:killer:_infectedGeneticCode",code).set("actor:killer:_timeToReproduce",20));
        for(int countdown:new int[]{3,4,5})result.add(scenario("fallow-own-infection-countdown-"+countdown,"FALLOW","WHITE").set("actor:killer:_infectedGeneticCode","g0").set("actor:killer:_timeToReproduce",countdown));
        for(int variant:new int[]{2,4,5})for(int countdown:new int[]{12,13,16,17,18,21,22,23})result.add(version(scenario("fallow-threshold-"+variant+"-"+countdown,"FALLOW","WHITE"),variant).set("actor:victim:_m",new double[]{1d}).set("actor:killer:_timeToReproduce",countdown));
        for(int maximum:new int[]{Integer.MAX_VALUE,Integer.MIN_VALUE})for(int variant:new int[]{2,4,5})result.add(version(scenario("fallow-wrapped-countdown-"+maximum+"-"+variant,"FALLOW","WHITE"),variant).set("actor:victim:_m",new double[]{1d}).set("actor:killer:_timeToReproduceMax",maximum));
        for(double length:new double[]{-1d,-0d,Double.POSITIVE_INFINITY,Double.NEGATIVE_INFINITY,Double.longBitsToDouble(0xfff8000000000042L),214748364.8d})for(int variant:new int[]{2,3,4})result.add(version(scenario("fallow-length-cast-"+variant+"-"+bits(length),"FALLOW","WHITE"),variant).set("actor:victim:_m",new double[]{length}));
        for(boolean infectious:new boolean[]{false,true})for(int late:new int[]{-3,-2,-1,0,1})result.add(version(scenario("fallow-late-"+infectious+"-"+late,"FALLOW","WHITE"),3).set("actor:victim:_isinfectious",infectious).set("actor:victim:_m",new double[]{2d}).set("actor:killer:_reproducelate",late));
        for(int time:new int[]{Integer.MIN_VALUE,-1,0,1,Integer.MAX_VALUE})result.add(version(scenario("fallow-spore-clock-"+time,"FALLOW","SPORE"),1).set("actor:killer:_sporeversion",1).set("actor:killer:_sporetime",time));
        for(String peer:new String[]{"WHITE","GREEN","FALLOW","RED","ROSE","BLUE"})result.add(version(scenario("fallow-inhibition-"+peer,"FALLOW",peer),4).set("actor:killer:_isreproductive",true).set("actor:killer:_isakiller",true).set("actor:killer:_healing",1));
        for(String peer:new String[]{"SPORE","AUBURN","INDIGO","YELLOW","BLOND"})for(boolean plant:new boolean[]{false,true})result.add(version(scenario("fallow-antiviral-damage-"+peer+"-"+plant,"FALLOW",peer),4).set("actor:killer:_isreproductive",true).set("actor:killer:_isaplant",plant).set("actor:killer:_isaconsumer",!plant).set("actor:killer:_antiviral",1).set("actor:killer:_sporeversion",1).set("actor:killer:_updateEffects",0));
        result.add(version(scenario("fallow-antiviral-spore-zero-is-not-damaged","FALLOW","SPORE"),4).set("actor:killer:_isreproductive",true).set("actor:killer:_antiviral",1));
        for(int shield:new int[]{185,186})result.add(scenario("fallow-normal-shield-"+shield,"FALLOW","WHITE").set("actor:killer:_lavender",shield));
        for(int shield:new int[]{2220,2221})result.add(scenario("fallow-weak-shield-"+shield,"FALLOW","WHITE").set("actor:victim:_isenhanced",true).set("actor:victim:_isaconsumer",true).set("actor:killer:_lavender",shield));
        result.add(scenario("fallow-infectious-enhancement-uses-normal-shield","FALLOW","WHITE").set("actor:victim:_isenhanced",true).set("actor:victim:_isinfectious",true).set("actor:killer:_lavender",186));
        for(boolean onlyC4:new boolean[]{false,true})for(int late:new int[]{0,1})result.add(scenario("fallow-leading-dodge-flag-"+onlyC4+"-"+late,"FALLOW","WHITE").set("actor:killer:_isonlyc4",onlyC4).set("actor:killer:_reproducelate",late).set("actor:killer:_dodge",true));
        result.add(scenario("fallow-partial-dodge","FALLOW","WHITE").set("actor:killer:_dodge",true).set("world:_O2",0.25d));
        for(String state:new String[]{"active","dormant","spore","none"}){
            Spec spec=scenario("fallow-jade-outer-gate-"+state,"FALLOW","JADE").set("actor:killer:_jadefactor",2);
            if(state.equals("dormant"))spec.set("actor:killer:active",false);
            if(state.equals("spore"))version(spec,1).set("actor:killer:_sporeversion",1);
            if(state.equals("none"))spec.set("actor:killer:_jadefactor",1);result.add(spec);
        }
        result.add(scenario("fallow-magenta-pays-protection","FALLOW","MAGENTA"));
        result.add(scenario("fallow-magenta-failed-protection-then-delay","FALLOW","MAGENTA").set("actor:killer:_energy",0.25d));
        result.add(scenario("fallow-altruist-rose","FALLOW","ROSE").set("actor:victim:_altruist",true));
        result.add(scenario("fallow-blue-partial-shield","FALLOW","BLUE").set("world:_O2",0.375d));
        for(boolean plant:new boolean[]{false,true})for(boolean enhanced:new boolean[]{false,true})result.add(scenario("fallow-red-gate-"+plant+"-"+enhanced,"FALLOW","RED").set("actor:victim:_isaplant",plant).set("actor:victim:_isenhanced",enhanced).set("actor:killer:_createlavender",1));
        result.add(version(scenario("fallow-one-bypasses-red-protection","FALLOW","RED"),1).set("actor:victim:_isaplant",true).set("actor:killer:_createlavender",1).set("actor:killer:_sporeversion",1));
        for(String peer:new String[]{"BROWN","VISION","BARK","OLDBARK"})for(boolean alive:new boolean[]{false,true})result.add(scenario("fallow-dormant-"+peer+"-alive-"+alive,"FALLOW",peer).set("actor:killer:active",false).set("actor:killer:alive",alive));
        for(boolean infectious:new boolean[]{false,true})result.add(scenario("fallow-two-dormant-infectious-"+infectious,"FALLOW","BROWN").set("actor:victim:_isinfectious",infectious).set("actor:killer:active",false));
        for(int variant:new int[]{1,2,3})result.add(version(scenario("fallow-dormant-bypasses-retained-shield-"+variant,"FALLOW","VISION"),variant).set("actor:killer:active",false).set("actor:killer:_lavender",10000));
        Spec dormant=scenario("fallow-dormant-mixed-current-colors","FALLOW","BROWN").set("actor:killer:active",false).set("actor:killer:_segColor",phenotype("BROWN","VISION","EYE","GREEN"));dormant.recipient.colors("BROWN","VISION","EYE","GREEN");result.add(dormant);
        Spec bark=scenario("fallow-dormant-mixed-bark-colors","FALLOW","BARK").set("actor:killer:active",false).set("actor:killer:_segColor",phenotype("BARK","OLDBARK","GREEN"));bark.recipient.colors("BARK","OLDBARK","GREEN");result.add(bark);
        for(String trait:new String[]{"_isaplant","_isaconsumer","_iscoral","_isafungus"})result.add(scenario("fallow-mint-attacker-"+trait,"FALLOW","MINT").set("actor:victim:"+trait,true));
        result.add(scenario("fallow-mint-pure-antiviral-protection","FALLOW","MINT").set("actor:victim:_antiviral",1).set("actor:killer:_isaplant",false));
        for(boolean enhanced:new boolean[]{false,true})result.add(scenario("fallow-lavender-refill-"+enhanced,"FALLOW","LAVENDER").set("actor:killer:_isenhanced",enhanced));
        result.add(scenario("fallow-lavender-source-zero-global","FALLOW","LAVENDER").setting("LAVENDER_ENERGY_CONSUMPTION",0d));
        for(double oxygen:new double[]{0d,0.0625d,0.125d,0.25d,Double.longBitsToDouble(0xfff8000000000042L)})result.add(version(scenario("fallow-independent-payments-"+bits(oxygen),"FALLOW","WHITE"),4).set("actor:killer:_isreproductive",true).set("actor:killer:_healing",1).set("world:_O2",oxygen));
        for(String weapon:new String[]{"PLAGUE","FALLOW"})for(double energy:new double[]{0d,Math.nextDown(Utils.tol),Utils.tol,Double.NaN})result.add(scenario("common-peer-death-"+weapon+"-"+bits(energy),weapon,"CORAL").set("actor:killer:_energy",energy));
        Spec selected=version(scenario("fallow-selected-second-segment","FALLOW","WHITE"),2).set("actor:victim:_m",new double[]{1d,9d});selected.parent.colors("FALLOW","FALLOW");selected.actions.set(1,new Action("touchEffects2.FALLOW","killer,1,0"));result.add(selected);
        result.add(scenario("plague-unsaved-viral-birth","PLAGUE","GREEN").set("actor:killer:_savedGeneticCode","~"));
        result.add(foreign(scenario("plague-unsaved-foreign-birth","PLAGUE","GREEN")).set("actor:killer:_savedGeneticCode","~"));
        result.add(blocked(scenario("plague-unsaved-blocked-viral-birth","PLAGUE","GREEN")).set("actor:killer:_savedGeneticCode","~"));
        result.add(blocked(foreign(scenario("plague-unsaved-blocked-foreign-birth","PLAGUE","GREEN"))).set("actor:killer:_savedGeneticCode","~"));
        Spec unsavedMutation=scenario("plague-unsaved-full-mutation-birth","PLAGUE","GREEN").set("actor:killer:_savedGeneticCode","~");unsavedMutation.parent.rate=10000;result.add(unsavedMutation);
        Spec foreignMutation=foreign(scenario("plague-unsaved-foreign-full-mutation-birth","PLAGUE","GREEN")).set("actor:killer:_savedGeneticCode","~");foreignMutation.infected.rate=10000;result.add(foreignMutation);
        for(int next:new int[]{Integer.MAX_VALUE,1,-1})result.add(scenario("plague-unsaved-newborn-id-"+next,"PLAGUE","GREEN").set("actor:killer:_savedGeneticCode","~").set("world:NEXT_ID",next));
        for(int variant:new int[]{1,6})result.add(plagueVersion(scenario("plague-unsaved-spore-consumer-version-"+variant,"PLAGUE","SPORE"),2).set("actor:killer:_savedGeneticCode","~").set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",true).set("actor:killer:_sporeversion",variant));
        Spec unsavedShared=scenario("plague-unsaved-shared-gene-edit-and-real-tree-rebuild","PLAGUE","GREEN").set("actor:killer:_savedGeneticCode","~");unsavedShared.add(tree().set("genome:g0:gene0._length",12.5d).set("genome:g0:_cladeID","plague-unsaved-edit"));result.add(unsavedShared);
        result.add(scenario("plague-unsaved-total-children-wrap","PLAGUE","GREEN").set("actor:killer:_savedGeneticCode","~").set("actor:killer:_nTotalChildren",Integer.MAX_VALUE));
        result.add(scenario("plague-unsaved-birth-post-payment-zero-oxygen","PLAGUE","GREEN").set("actor:killer:_savedGeneticCode","~").set("world:_O2",0d));
        return result;
    }
    private static String capture()throws Exception{
        contact=Organism.class.getDeclaredMethod("touchEffects2",Organism.class,int.class,int.class);contact.setAccessible(true);
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){
            Capture capture=new Capture(spec);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index));output.append(capture.finish());
        }return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.PlagueFallowOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        Files.writeString(directory.resolve("plague_fallow.tsv"),capture(),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("PLAGUE_FALLOW_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls+" newborns="+newborns);
    }
}

