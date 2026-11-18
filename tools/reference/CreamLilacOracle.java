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

/** Observe original Cream and Lilac touchEffects1 branches on real placed organisms */
public final class CreamLilacOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static int cases,snapshots,calls;

    private static final class Action{
        final String operation,argument;
        final Map<String,Object> controls=new LinkedHashMap<>();
        Action(String operation,String argument){this.operation=operation;this.argument=argument;}
        Action set(String target,Object value){controls.put(target,value);return this;}
    }
    private static final class Spec{
        final String name;
        String[] genes={"GREEN"},peerGenes={"GREEN"};
        double length=100;
        double[] lengths;
        int symmetry=1,mirror,spore=1;
        boolean passive,modifiedSky;
        final Map<String,Object> settings=new LinkedHashMap<>(),controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        Spec(String name){this.name=name;}
        Spec add(Action action){actions.add(action);return this;}
        Spec set(String name,Object value){controls.put(name,value);return this;}
        Spec setting(String name,Object value){settings.put(name,value);return this;}
        Spec colors(String...names){genes=names;return this;}
        Spec huge(){settings.put("WORLD_WIDTH",1000000000);settings.put("WORLD_HEIGHT",1000000000);return this;}
    }
    private static final class Capture{
        final StringBuilder rows=new StringBuilder();
        final Map<String,GeneticCode> genomes=new LinkedHashMap<>();
        final Map<String,Organism> actors=new LinkedHashMap<>();
        final IdentityHashMap<GeneticCode,String> references=new IdentityHashMap<>();
        final World world;
        Capture(Spec spec)throws Exception{
            for(Field original:Utils.class.getDeclaredFields())if(original.getName().startsWith("DEF_"))Utils.class.getDeclaredField(original.getName().substring(4)).set(null,original.get(null));
            Utils.INITIAL_ORGANISMS=0;Utils.INITIAL_O2=1000;Utils.MIN_MUTATION_RATE=0;Utils.MAX_MUTATION_RATE=10000;Utils.META_MUTATION_RATE=0;
            for(String name:COLORS)Utils.class.getDeclaredField(name+"_PROB").setInt(null,name.equals("GREEN")?7:0);
            for(Map.Entry<String,Object> entry:spec.settings.entrySet())Utils.class.getDeclaredField(entry.getKey()).set(null,entry.getValue());
            Utils.random=new Random(827364);
            VisibleWorld visible=new VisibleWorld(null);world=new World(visible);
            world.addCO2(Utils.INITIAL_CO2);world.addO2(Utils.INITIAL_O2);world.addCH4(Utils.INITIAL_CH4);
            genomes.put("g0",genome(spec));
            genomes.put("g1",genome(new Spec("actor").colors(spec.peerGenes)));genomes.put("g2",genome(new Spec("peer")));
            genomes.put("g3",(GeneticCode)genomes.get("g0").clone());genomes.put("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,"canonical",827364);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                references.put(entry.getValue(),entry.getKey());
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g3")||entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            Organism peer=paste("peer","g2",1000,1000);visible._selectedOrganism=peer;
            row("I","setup","visible:selected","String","peer");world.addOrganism(peer,null);
            Organism victim=paste("victim","g0",Utils.WORLD_WIDTH>1000000?Utils.WORLD_WIDTH/2:3500,Utils.WORLD_HEIGHT>1000000?Utils.WORLD_HEIGHT/2:3500);world.addOrganism(victim,null);
            Organism killer=paste("killer","g1",2000,2000);world.addOrganism(killer,null);
            control("setup","actor:victim:_infectedGeneticCode","g3");control("setup","actor:victim:_savedGeneticCode","g4");
            for(Map.Entry<String,Object> entry:prepared(victim._segments).entrySet())control("setup",entry.getKey(),entry.getValue());
            for(Map.Entry<String,Object> entry:spec.controls.entrySet())control("setup",entry.getKey(),entry.getValue());
            snapshot("initial");
        }
        Organism paste(String role,String code,int x,int y)throws Exception{
            Organism actor=new Organism(world,genomes.get(code));require(actor.pasteOrganism(x,y),"Real actor paste failed");actors.put(role,actor);row("A",role,code,x,y);return actor;
        }
        void row(Object...values){for(int i=0;i<values.length;i++){if(i!=0)rows.append('\t');rows.append(values[i]);}rows.append('\n');}
        String scalar(Object value){
            if(value==null)return "~";
            if(value instanceof Double number)return bits(number);
            if(value instanceof Color color)return String.format(java.util.Locale.ROOT,"%08x",color.getRGB());
            if(value instanceof Organism actor){for(Map.Entry<String,Organism> entry:actors.entrySet())if(entry.getValue()==actor)return entry.getKey();throw new IllegalStateException("Unknown organism reference");}
            if(value instanceof Rectangle r)return r.x+","+r.y+","+r.width+","+r.height;
            if(value instanceof GeneticCode code){String reference=references.get(code);require(reference!=null,"Unknown genome reference");return reference;}
            if(value instanceof World||value instanceof VisibleWorld)return "present";
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
            row("P",stage);
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
            Field[] statsFields=fields(WorldStatistics.class);require(statsFields.length==52,"Unexpected statistics schema");
            for(Field field:statsFields)row("S",stage,field.getName(),field.getType().getSimpleName(),scalar(field.get(world.worldStatistics)));
            Method[] getters=Arrays.stream(WorldStatistics.class.getDeclaredMethods()).filter(method->Modifier.isPublic(method.getModifiers())&&method.getParameterCount()==0&&method.getName().startsWith("get")).sorted(Comparator.comparing(Method::getName)).toArray(Method[]::new);
            require(getters.length==49,"Unexpected statistics getter schema");for(Method method:getters)row("T",stage,method.getName(),method.getReturnType().getSimpleName(),scalar(method.invoke(world.worldStatistics)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet())genomeState(stage,entry.getKey(),entry.getValue());
            Random peek=copyRandom(Utils.random);row("R",stage,peek.nextLong(),peek.nextInt(10000),bits(peek.nextDouble()));snapshots++;
        }
        void control(String stage,String target,Object value)throws Exception{
            String[] path=target.split(":",3);Object owner;String name;
            if(target.equals("visible:selected")){world._visibleWorld._selectedOrganism=actors.get((String)value);row("I",stage,target,"String",value);return;}
            switch(path[0]){case "actor":owner=actors.get(path[1]);name=path[2];break;case "world":owner=world;name=path[1];break;case "statistics":owner=world.worldStatistics;name=path[1];break;case "genome":owner=genomes.get(path[1]);name=path[2];break;default:throw new IllegalArgumentException("Unknown control target");}
            if(name.startsWith("gene")){
                int dot=name.indexOf('.');int index=Integer.parseInt(name.substring(4,dot));String field=name.substring(dot+1);Gene gene=((GeneticCode)owner).getGene(index);
                if(field.equals("_length")){gene.setLength((Double)value);row("I",stage,target,"double",bits((Double)value));}
                else if(field.equals("_color")){gene.setColor((Color)value);row("I",stage,target,"Color",scalar(value));}
                else throw new IllegalArgumentException("Unsupported actual gene setter");return;
            }
            Field field=owner.getClass().getDeclaredField(name);field.setAccessible(true);
            if(field.getType()==GeneticCode.class){field.set(owner,value.equals("~")?null:genomes.get((String)value));row("I",stage,target,"GeneticCode",value);}
            else{field.set(owner,value);row("I",stage,target,field.getType().getSimpleName(),scalar(value));}
        }
        void action(int index,Action action)throws Exception{
            String stage="step-"+index;
            for(Map.Entry<String,Object> input:action.controls.entrySet())control(stage,input.getKey(),input.getValue());
            Organism victim=actors.get("victim");row("K",stage,action.operation,"victim",action.argument);
            invoke(action,victim,stage);
            calls++;snapshot(stage);
        }
        void invoke(Action action,Organism victim,String stage)throws Exception{
            String prefix="touchEffects1.";require(action.operation.startsWith(prefix),"Unknown contact operation");String branch=action.operation.substring(prefix.length());
            require(branch.equals("CREAM")||branch.equals("LILAC"),"Unsupported touchEffects1 branch");
            String[] args=action.argument.split(",");require(args.length==3&&args[0].equals("killer"),"Invalid contact arguments");
            int segment=Integer.parseInt(args[1]),peerSegment=Integer.parseInt(args[2]);
            require(victim._segColor[segment].getRGB()==color(branch).getRGB(),"Actor ARGB does not select requested branch");
            Method method=Organism.class.getDeclaredMethod("touchEffects1",Organism.class,int.class,int.class);method.setAccessible(true);
            method.invoke(victim,actors.get(args[0]),segment,peerSegment);
        }

        String finish(){row("E");cases++;return rows.toString();}
    }
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static Random copyRandom(Random random)throws Exception{ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception{
        switch(name){case "UNKNOWN":return new Color(0x12345678,true);case "TRANSLUCENT":return new Color(0x12ff00ff,true);case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static Color[] phenotype(String...names)throws Exception{Color[] values=new Color[names.length];for(int i=0;i<names.length;i++)values[i]=color(names[i]);return values;}
    private static Color[] palette()throws Exception{
        List<Color> values=new ArrayList<>();for(String name:COLORS)values.add(color(name));for(String name:INTERNAL)values.add(color(name));values.add(new Color(0x12345678,true));values.add(new Color(0x12ff00ff,true));return values.toArray(Color[]::new);
    }
    private static GeneticCode genome(Spec spec)throws Exception{
        List<Gene> genes=new ArrayList<>();for(int i=0;i<spec.genes.length;i++){
            Gene gene=new Gene();gene.setLength(spec.lengths==null?spec.length:spec.lengths[i]);gene.setTheta(i*0.25);gene.setBranch(0);gene.setColor(color(spec.genes[i]));genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,spec.symmetry,spec.mirror,0,0,1,1,spec.spore,1,false,false,false,false,false,false,false,false,spec.passive,false,false,false,false,spec.modifiedSky,false);code._generation=41;code._cladeID="root";return code;
    }
    private static Map<String,Object> prepared(int segments){
        Map<String,Object> values=new LinkedHashMap<>();
        values.put("actor:victim:_energy",20d);values.put("actor:killer:_energy",50d);
        for(String role:new String[]{"victim","killer"}){
            for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus","_isenhanced","_isinfectious","_isauburn","_iscoral","_altruist","_dodge","_hasdodged","_isinjured","_useextraeffects"})values.put("actor:"+role+":"+flag,false);
            values.put("actor:"+role+":active",true);values.put("actor:"+role+":_lavender",0);values.put("actor:"+role+":_createlavender",0);
            values.put("actor:"+role+":_plagueversion",0);values.put("actor:"+role+":_fallowversion",0);values.put("actor:"+role+":_sporeversion",0);
            values.put("actor:"+role+":_skyversion",1);values.put("actor:"+role+":_updateEffects",0);
        }
        values.put("actor:victim:_healing",0);values.put("actor:victim:_photosynthesis",3.25d);
        values.put("actor:victim:_color",Color.RED);values.put("actor:victim:_framesColor",4);
        double[] peerPhoto=new double[segments];Arrays.fill(peerPhoto,2d);
        values.put("actor:killer:_photosynthesis",10d);values.put("actor:killer:_mphoto",peerPhoto);
        values.put("actor:killer:_color",Color.BLUE);values.put("actor:killer:_framesColor",5);
        return values;
    }
    private static Action call(String branch){return new Action("touchEffects1."+branch,"killer,0,0");}
    private static Spec base(String name,String branch,String peer)throws Exception{
        return new Spec(name).colors(branch).set("actor:killer:_segColor",phenotype(peer)).set("genome:g0:_modifieslilac",branch.equals("LILAC"));
    }
    private static Spec one(String name,String branch,String peer)throws Exception{return base(name,branch,peer).add(call(branch));}
    private static Action isolated(String branch,String peer)throws Exception{return call(branch).set("actor:victim:_segColor",phenotype(branch)).set("actor:killer:_segColor",phenotype(peer)).set("actor:victim:_energy",20d).set("actor:killer:_energy",80d).set("actor:killer:_lavender",0);}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(String branch:new String[]{"CREAM","LILAC"}){
            int index=0;for(Color peer:palette())result.add(new Spec(branch.toLowerCase(java.util.Locale.ROOT)+"-palette-"+index++).colors(branch).set("genome:g0:_modifieslilac",branch.equals("LILAC")).set("actor:killer:_segColor",new Color[]{peer}).add(call(branch)));
        }
        for(int modification:new int[]{1,2,3}){
            Spec enhanced=base("cream-enhanced-modifier-"+modification,"CREAM","GREEN").set("genome:g0:_modifiescream",modification).set("actor:victim:_isenhanced",true);
            for(String peer:new String[]{"WHITE","FIRE","ORANGE","RED","MAROON","BLUE","DEEPSKY","FALLOW","OLDBARK","FRUIT","CREAM","PINK","CORAL","BROWN"})enhanced.add(isolated("CREAM",peer));result.add(enhanced);
            Spec inactive=base("cream-inactive-modifier-"+modification,"CREAM","GREEN").set("genome:g0:_modifiescream",modification).set("actor:victim:active",false);
            for(String peer:new String[]{"DEEPSKY","OLDBARK","PINK","BROWN"})inactive.add(isolated("CREAM",peer));result.add(inactive);
        }
        for(String classification:new String[]{"_isaplant","_isaconsumer","_isafungus","_plagueversion","_isauburn"}){
            Spec spec=one("cream-white-classified-"+classification,"CREAM","WHITE");if(classification.equals("_plagueversion"))spec.set("actor:killer:"+classification,1);else spec.set("actor:killer:"+classification,true);result.add(spec);
        }
        for(int lavender:new int[]{0,1,-1})result.add(one("cream-spike-plant-lavender-"+lavender,"CREAM","SPIKE").set("actor:killer:_isaplant",true).set("actor:killer:_createlavender",lavender));
        result.add(one("cream-spike-enhanced","CREAM","SPIKE").set("actor:killer:_isenhanced",true).set("actor:killer:_createlavender",1));
        for(int peerModification:new int[]{2,3})result.add(one("cream-specialized-peer-"+peerModification,"CREAM","CREAM").set("actor:victim:_isenhanced",true).set("genome:g0:_modifiescream",2).set("actor:killer:_isenhanced",true).set("genome:g1:_modifiescream",peerModification).set("actor:killer:_lavender",1000));
        for(int count:new int[]{0,1})for(String classification:new String[]{"ordinary","consumer","fungus"}){
            Spec spec=one("cream-silver-history-"+classification+"-"+count,"CREAM","SILVER").set("actor:killer:_nTotalKills",count);
            if(!classification.equals("ordinary"))spec.set("actor:killer:_isa"+classification,true);result.add(spec);
        }
        for(int created:new int[]{0,-1,1})result.add(one("cream-blue-specialization-gate-"+created,"CREAM","BLUE").set("actor:victim:_isenhanced",true).set("genome:g0:_modifiescream",3).set("actor:killer:_isaplant",true).set("actor:killer:_createlavender",created).setting("BLUE_ENERGY_CONSUMPTION",1d));
        result.add(one("cream-blue-specialization-nonplant","CREAM","BLUE").set("actor:victim:_isenhanced",true).set("genome:g0:_modifiescream",3).setting("BLUE_ENERGY_CONSUMPTION",1d));
        for(boolean active:new boolean[]{false,true})for(int version:new int[]{1,2})result.add(one("cream-sky-attacker-active-"+active+"-version-"+version,"CREAM","SKY").set("actor:victim:active",active).set("actor:killer:_skyversion",version));
        result.add(one("cream-sky-partial-payment","CREAM","SKY").setting("SKY_ENERGY_CONSUMPTION",1d).set("world:_O2",0.5d));
        for(int shield:new int[]{0,1,184,185,186,1000,Integer.MIN_VALUE,Integer.MAX_VALUE})result.add(one("cream-lavender-recheck-"+shield,"CREAM","GREEN").set("actor:killer:_lavender",shield));
        for(boolean enhanced:new boolean[]{false,true}){
            result.add(one("cream-lavender-wrapping-failed-payment-"+enhanced,"CREAM","LAVENDER").setting("LAVENDER_SHIELD",Integer.MAX_VALUE).set("actor:killer:_lavender",Integer.MAX_VALUE-50).set("actor:killer:_isenhanced",enhanced).set("world:_O2",0d));
        }
        for(double oxygen:new double[]{0d,0.1d,0.2d})result.add(one("cream-lavender-charge-"+bits(oxygen),"CREAM","LAVENDER").set("actor:victim:_energy",0.2d).set("world:_O2",oxygen));
        for(int version:new int[]{2,3})result.add(one("cream-fruit-shield-bypass-"+version,"CREAM","FRUIT").set("actor:killer:_sporeversion",version).set("actor:killer:_lavender",1000).set("actor:victim:_isenhanced",true).set("genome:g0:_modifiescream",2));
        for(String reaction:new String[]{"healing","plant","ordinary"}){
            Spec spec=base("cream-mint-retained-caches-"+reaction,"CREAM","MINT").set("actor:victim:_healing",reaction.equals("healing")?1:0).set("actor:victim:_isaplant",reaction.equals("plant"));
            spec.add(call("CREAM").set("world:_O2",0.003d)).add(call("CREAM").set("world:_O2",0.0075d));result.add(spec);
        }
        result.add(one("cream-mint-empty-oxygen","CREAM","MINT").set("actor:victim:_healing",1).set("world:_O2",0d));
        for(boolean alive:new boolean[]{false,true})result.add(one("cream-brown-inactive-"+alive,"CREAM","BROWN").set("actor:victim:active",false).set("actor:killer:alive",alive).set("world:_population",alive?3:2));
        for(String eligibility:new String[]{"none","consumer","fungus","plague","fallow","coral","coral-actor-plant","coral-actor-consumer","coral-actor-fungus"}){
            Spec spec=one("lilac-eligibility-"+eligibility,"LILAC","WHITE").set("genome:g0:_modifieslilac",false);
            if(eligibility.equals("consumer")||eligibility.equals("fungus"))spec.set("actor:killer:_isa"+eligibility,true);
            if(eligibility.equals("plague")||eligibility.equals("fallow"))spec.set("actor:killer:_"+eligibility+"version",1);
            if(eligibility.startsWith("coral"))spec.set("actor:killer:_iscoral",true);
            if(eligibility.contains("actor-"))spec.set("actor:victim:_isa"+eligibility.substring(eligibility.indexOf("actor-")+6),true);result.add(spec);
        }
        for(String peer:new String[]{"GRASS","GREEN"})result.add(one("lilac-grass-half-strength-"+peer,"LILAC",peer).set("actor:victim:_m",new double[]{1d}).set("actor:killer:_energy",100d));
        for(boolean actorEnhanced:new boolean[]{false,true})for(boolean peerEnhanced:new boolean[]{false,true})result.add(one("lilac-own-and-peer-recharge-"+actorEnhanced+"-"+peerEnhanced,"LILAC","LILAC").set("actor:victim:_isenhanced",actorEnhanced).set("actor:killer:_isenhanced",peerEnhanced).set("actor:killer:_energy",100d));
        for(boolean modified:new boolean[]{false,true})for(boolean enhanced:new boolean[]{false,true})result.add(one("lilac-spikepoint-specialization-"+modified+"-"+enhanced,"LILAC","SPIKEPOINT").set("genome:g0:_modifieslilac",modified).set("actor:killer:_isenhanced",enhanced).set("actor:killer:_isaplant",!enhanced));
        for(boolean modified:new boolean[]{false,true})result.add(one("lilac-brown-enhanced-"+modified,"LILAC","BROWN").set("genome:g0:_modifieslilac",modified).set("actor:killer:_isenhanced",true));
        result.add(one("lilac-red-enhanced-immunity","LILAC","RED").set("actor:killer:_isenhanced",true));
        for(double oxygen:new double[]{0d,0.5d,1d,2d})result.add(one("lilac-attack-before-damage-"+bits(oxygen),"LILAC","FIRE").setting("LILAC_ENERGY_CONSUMPTION",1d).set("world:_O2",oxygen));
        for(String branch:new String[]{"CREAM","LILAC"}){
            for(double oxygen:new double[]{0d,0.5d,1d})result.add(one(branch.toLowerCase(java.util.Locale.ROOT)+"-dodge-payment-"+bits(oxygen),branch,"GREEN").set("actor:killer:_dodge",true).setting("DODGE_ENERGY_CONSUMPTION",1d).set("world:_O2",oxygen));
            result.add(one(branch.toLowerCase(java.util.Locale.ROOT)+"-blue-partial-then-fallback",branch,"BLUE").setting("BLUE_ENERGY_CONSUMPTION",1d).set("world:_O2",0.5d));
            result.add(one(branch.toLowerCase(java.util.Locale.ROOT)+"-blue-double-death",branch,"BLUE").set("actor:victim:_energy",0.1d).set("actor:killer:_energy",0d).set("actor:killer:_isenhanced",true).set("world:_O2",0.1d));
            for(String peer:new String[]{"MINT","LAVENDER","MAGENTA","ROSE","TRANSLUCENT"})result.add(one(branch.toLowerCase(java.util.Locale.ROOT)+"-altruist-"+peer,branch,peer).set("actor:victim:_altruist",true));
            for(double length:new double[]{-1d,0d,Double.NaN,Double.POSITIVE_INFINITY})result.add(one(branch.toLowerCase(java.util.Locale.ROOT)+"-controlled-length-"+bits(length),branch,"UNKNOWN").set("actor:victim:_m",new double[]{length}));
            for(double energy:new double[]{Utils.tol,Utils.tol*0.5,-1d,Double.NaN,Double.POSITIVE_INFINITY}){
                Spec spec=one(branch.toLowerCase(java.util.Locale.ROOT)+"-common-death-"+bits(energy),branch,branch.equals("CREAM")?"SPIKEPOINT":"OLIVE").set("actor:killer:_energy",energy);
                if(energy==Utils.tol*0.5)spec.add(call(branch));result.add(spec);
            }
        }
        for(double byproduct:new double[]{0d,1d})result.add(one("cream-byproduct-"+bits(byproduct),"CREAM","UNKNOWN").setting("CREAM_ORGANIC_SUBS_PRODUCED",byproduct).set("world:_O2",0.1d));
        for(String branch:new String[]{"CREAM","LILAC"}){
            Spec spec=new Spec(branch.toLowerCase(java.util.Locale.ROOT)+"-nonzero-segment-indices").colors("GREEN",branch);spec.peerGenes=new String[]{"GREEN","GREEN"};
            spec.set("actor:killer:_mphoto",new double[]{1,2}).set("actor:killer:_segColor",phenotype("BARK","GRASS")).set("genome:g0:_modifieslilac",true).add(new Action("touchEffects1."+branch,"killer,1,1"));result.add(spec);
        }
        for(String peer:new String[]{"GRASS","GREEN"})result.add(one("lilac-ineligible-before-dodge-"+peer,"LILAC",peer).set("genome:g0:_modifieslilac",false).set("actor:killer:_dodge",true).setting("DODGE_ENERGY_CONSUMPTION",1d).set("world:_O2",0.5d));
        for(String classification:new String[]{"plant","enhanced"})result.add(one("lilac-spike-eligible-"+classification,"LILAC","SPIKE").set("actor:killer:"+(classification.equals("plant")?"_isaplant":"_isenhanced"),true));
        for(double oxygen:new double[]{0d,0.5d,1d,2d})result.add(one("cream-attack-before-transfer-"+bits(oxygen),"CREAM","UNKNOWN").setting("CREAM_ENERGY_CONSUMPTION",1d).set("world:_O2",oxygen));
        for(String branch:new String[]{"CREAM","LILAC"}){
            result.add(one(branch.toLowerCase(java.util.Locale.ROOT)+"-attack-insufficient-actor-energy",branch,"UNKNOWN").setting(branch+"_ENERGY_CONSUMPTION",1d).set("actor:victim:_energy",0d));
            Spec repeat=base(branch.toLowerCase(java.util.Locale.ROOT)+"-repeated-paid-dodge",branch,"GREEN").set("actor:killer:_dodge",true).setting("DODGE_ENERGY_CONSUMPTION",1d);
            repeat.add(call(branch).set("world:_O2",1d)).add(call(branch).set("world:_O2",1d));result.add(repeat);
        }
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.CreamLilacOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("cream_lilac.tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("CREAM_LILAC_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
