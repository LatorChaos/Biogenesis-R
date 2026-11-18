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

/** Observe the five completed energy-draining touchEffects1 branches on real placed organisms */
public final class DrainOracle {
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
        final String name,weapon,target;
        String[] genes,peerGenes={"GREEN"};
        double length=12;
        double[] lengths;
        int symmetry=1,mirror,spore=1;
        boolean passive,modifiedSky,clonePeer,sharedPeer;
        final Map<String,Object> settings=new LinkedHashMap<>(),controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        Spec(String name,String weapon,String target){this.name=name;this.weapon=weapon;this.target=target;genes=new String[]{weapon,weapon};}
        Spec add(Action action){actions.add(action);return this;}
        Spec set(String name,Object value){controls.put(name,value);return this;}
        Spec setting(String name,Object value){settings.put(name,value);return this;}
        Spec colors(String...names){genes=names;return this;}
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
            genomes.put("g1",genome(new Spec("actor","GREEN","GREEN").colors(spec.peerGenes)));genomes.put("g2",genome(new Spec("peer","GREEN","GREEN").colors("GREEN")));
            genomes.put("g3",(GeneticCode)genomes.get(spec.clonePeer?"g1":"g0").clone());genomes.put("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,"canonical",827364);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                references.put(entry.getValue(),entry.getKey());
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g3")?(spec.clonePeer?"clone:g1":"clone:g0"):entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            Organism peer=paste("peer","g2",1000,1000);visible._selectedOrganism=peer;
            row("I","setup","visible:selected","String","peer");world.addOrganism(peer,null);
            Organism victim=paste("victim","g0",Utils.WORLD_WIDTH>1000000?Utils.WORLD_WIDTH/2:3500,Utils.WORLD_HEIGHT>1000000?Utils.WORLD_HEIGHT/2:3500);world.addOrganism(victim,null);
            Organism killer=paste("killer",spec.sharedPeer?"g0":"g1",2000,2000);world.addOrganism(killer,null);
            control("setup","actor:victim:_infectedGeneticCode","g3");control("setup","actor:victim:_savedGeneticCode","g4");
            for(Map.Entry<String,Object> entry:prepared(victim._segments,killer._segments).entrySet())control("setup",entry.getKey(),entry.getValue());
            control("setup","actor:killer:_segColor",repeatedColor(spec.target,killer._segments));
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
            require(world._organisms.size()==3,"Unexpected member creation requires another observation schema");row("P",stage);
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
            if(owner instanceof Organism actor&&field.getType().isArray())require(Array.getLength(value)==actor._segments,"Explicit array input length must match actual segments");
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
            require(action.operation.startsWith("touchEffects1."),"Original touchEffects1 branch label required");
            String branch=action.operation.substring("touchEffects1.".length());require(List.of("ORANGE","FIRE","RED","PINK","MAROON").contains(branch),"Unsupported drain branch");
            String[] arguments=action.argument.split(",");require(arguments.length==3&&arguments[0].equals("killer"),"Distinct peer and two indices required");
            int segment=Integer.parseInt(arguments[1]),peerSegment=Integer.parseInt(arguments[2]);
            Organism peer=actors.get(arguments[0]);require(peer!=victim,"Drain oracle excludes self contacts");
            require(victim._segColor[segment].equals(color(branch)),"Exact actor ARGB must select the labelled drain branch");
            Method method=Organism.class.getDeclaredMethod("touchEffects1",Organism.class,int.class,int.class);method.setAccessible(true);
            method.invoke(victim,peer,segment,peerSegment);
        }
        String finish(){row("E");cases++;return rows.toString();}
    }
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static Random copyRandom(Random random)throws Exception{ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception{
        switch(name){case "UNKNOWN":return new Color(0x12345678,true);case "TRANSLUCENT_MAGENTA":return new Color(0x12ff00ff,true);case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static Color[] phenotype(String...names)throws Exception{Color[] values=new Color[names.length];for(int i=0;i<names.length;i++)values[i]=color(names[i]);return values;}
    private static Color[] repeatedColor(String name,int count)throws Exception{Color[] values=new Color[count];Arrays.fill(values,color(name));return values;}
    private static GeneticCode genome(Spec spec)throws Exception{
        List<Gene> genes=new ArrayList<>();for(int i=0;i<spec.genes.length;i++){
            Gene gene=new Gene();gene.setLength(spec.lengths==null?spec.length:spec.lengths[i]);gene.setTheta(i*0.25);gene.setBranch(i==0?0:-1);gene.setColor(color(spec.genes[i]));genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,spec.symmetry,spec.mirror,0,0,1,1,spec.spore,1,false,false,false,false,false,false,false,false,spec.passive,false,false,false,false,spec.modifiedSky,false);code._generation=41;code._cladeID="root";return code;
    }
    private static Map<String,Object> prepared(int segments,int peerSegments){
        Map<String,Object> values=new LinkedHashMap<>();
        for(String role:new String[]{"victim","killer"}){
            values.put("actor:"+role+":_energy",1000d);values.put("actor:"+role+":alive",true);values.put("actor:"+role+":active",true);
            for(String name:new String[]{"_isaplant","_isaconsumer","_isafungus","_isauburn","_isenhanced","_isgray","_isblond","_isfrozen","_isinjured","_isinfectious","_transfersenergy","_dodge","_hasdodged","_candodge","_altruist"})values.put("actor:"+role+":"+name,false);
            for(String name:new String[]{"_plagueversion","_sporeversion","_fallowversion","_skyversion","_nTotalKills","_nTotalInfected","_indigo"})values.put("actor:"+role+":"+name,0);
            values.put("actor:"+role+":_color",Color.MAGENTA);values.put("actor:"+role+":_framesColor",0);values.put("actor:"+role+":_photosynthesis",10d);
        }
        double[] actorPhoto=new double[segments],peerPhoto=new double[peerSegments];for(int i=0;i<segments;i++)actorPhoto[i]=i+0.25;for(int i=0;i<peerSegments;i++)peerPhoto[i]=i+0.25;
        values.put("actor:victim:_mphoto",actorPhoto);values.put("actor:killer:_mphoto",peerPhoto);return values;
    }
    private static Action action(Spec spec){return new Action("touchEffects1."+spec.weapon,"killer,"+(spec.genes.length*spec.symmetry-1)+",0");}
    private static Spec base(String name,String weapon,String peer){
        require(List.of("ORANGE","FIRE","RED","PINK","MAROON").contains(weapon),"Only completed actor branches are allowed");
        return new Spec(name,weapon,peer).setting("ORANGE_ENERGY_CONSUMPTION",0.125d).setting("FIRE_ENERGY_CONSUMPTION",0.25d)
            .setting("RED_ENERGY_CONSUMPTION",0.5d).setting("PINK_ENERGY_CONSUMPTION",1d).setting("MAROON_ENERGY_CONSUMPTION",2d)
            .setting("BLUE_ENERGY_CONSUMPTION",0.375d).setting("DODGE_ENERGY_CONSUMPTION",0.625d).setting("SKY_ENERGY_CONSUMPTION",0.75d);
    }
    private static Spec scenario(String name,String weapon,String peer){Spec spec=base(name,weapon,peer);return spec.add(action(spec));}
    private static String edible(String weapon){return weapon.equals("FIRE")||weapon.equals("RED")?"ORANGE":weapon.equals("PINK")?"WHITE":"GREEN";}
    private static String ignored(String weapon){return weapon.equals("FIRE")?"OLIVE":weapon.equals("RED")||weapon.equals("PINK")?"GREEN":"RED";}
    private static double cost(String weapon){switch(weapon){case "ORANGE":return 0.125d;case "FIRE":return 0.25d;case "RED":return 0.5d;case "PINK":return 1d;case "MAROON":return 2d;default:throw new IllegalArgumentException("Unknown weapon input");}}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(String peer:COLORS)result.add(scenario("palette-ORANGE-"+peer,"ORANGE",peer));
        for(String peer:INTERNAL)result.add(scenario("palette-ORANGE-"+peer,"ORANGE",peer));
        for(String peer:new String[]{"UNKNOWN","TRANSLUCENT_MAGENTA"})result.add(scenario("palette-ORANGE-"+peer,"ORANGE",peer));
        for(String peer:new String[]{"WHITE","GRASS","GREEN","YELLOW","RED","FIRE","ORANGE","SPIKE","SILVER","PINK","MAROON","CREAM","FALLOW","PLAGUE","BLUE","LIGHT_BLUE","SKY","DEEPSKY","VIOLET","FRUIT","ICE","BARK","OLDBARK","LIGHTBROWN","BROWN","GREENBROWN","MINT","UNKNOWN"})result.add(scenario("baseline-FIRE-"+peer,"FIRE",peer));
        for(String peer:new String[]{"RED","FIRE","ORANGE","SILVER","SPIKE","SPIKEPOINT","GRAY","CREAM","LILAC","BARK","UNKNOWN"})result.add(scenario("baseline-RED-"+peer,"RED",peer));
        for(String peer:new String[]{"PINK","WHITE","FRUIT","CREAM","GREENBROWN","DEADBARK","DARKFIRE","CORAL","BROWN","PLAGUE","OLDBARK","BARK","UNKNOWN"})result.add(scenario("baseline-PINK-"+peer,"PINK",peer));
        for(String peer:new String[]{"WHITE","GRASS","GREEN","BLUE","PLAGUE","FRUIT","BROKEN","OLDBARK","BARK","CYAN","SILVER","SPIKEPOINT","MINT","SKY","GREENBROWN","UNKNOWN"})result.add(scenario("baseline-MAROON-"+peer,"MAROON",peer));
        for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus","_isauburn"})result.add(scenario("orange-white-"+flag,"ORANGE","WHITE").set("actor:killer:"+flag,true));
        for(int version:new int[]{-1,1})result.add(scenario("orange-white-plague-"+version,"ORANGE","WHITE").set("actor:killer:_plagueversion",version));
        result.add(scenario("orange-violet-classified","ORANGE","VIOLET").set("actor:killer:_isaplant",true));
        for(String flag:new String[]{"_isaconsumer","_isafungus","_isaplant","_transfersenergy"})result.add(scenario("orange-yellow-"+flag,"ORANGE","YELLOW").set("actor:killer:"+flag,true));
        for(String weapon:new String[]{"ORANGE","FIRE","RED"}){
            result.add(scenario("silver-enhanced-"+weapon,weapon,"SILVER").set("actor:killer:_isenhanced",true));
            for(String flag:new String[]{"_isaconsumer","_isafungus"})result.add(scenario("silver-positive-kill-"+weapon+"-"+flag,weapon,"SILVER").set("actor:killer:"+flag,true).set("actor:killer:_nTotalKills",1));
            result.add(scenario("silver-negative-kill-"+weapon,weapon,"SILVER").set("actor:killer:_isaconsumer",true).set("actor:killer:_nTotalKills",Integer.MIN_VALUE));
        }
        result.add(scenario("orange-coral-consumer","ORANGE","CORAL").set("actor:killer:_isaconsumer",true));
        result.add(scenario("orange-plague-infectious","ORANGE","PLAGUE").set("actor:killer:_isinfectious",true));
        for(String flag:new String[]{"_isaplant","_isenhanced"})result.add(scenario("orange-spike-"+flag,"ORANGE","SPIKE").set("actor:killer:"+flag,true));
        for(int version:new int[]{2,3})result.add(scenario("orange-fallow-ordinary-enhanced-"+version,"ORANGE","FALLOW").set("actor:victim:_isenhanced",true).set("actor:killer:_fallowversion",version));
        result.add(scenario("orange-fallow-plant-weak","ORANGE","FALLOW").set("actor:killer:_isaplant",true));
        result.add(scenario("orange-fallow-plant-enhanced","ORANGE","FALLOW").set("actor:killer:_isaplant",true).set("actor:victim:_isenhanced",true).set("actor:killer:_fallowversion",3));
        for(int version:new int[]{3,4})result.add(scenario("orange-fallow-infectious-"+version,"ORANGE","FALLOW").set("actor:killer:_isinfectious",true).set("actor:killer:_fallowversion",version));
        for(String weapon:new String[]{"ORANGE","FIRE","MAROON"}){
            result.add(scenario("blue-enhanced-retaliation-"+weapon,weapon,"BLUE").set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true));
            result.add(scenario("blue-failed-shield-energy-"+weapon,weapon,"BLUE").set("actor:killer:_isaplant",true).set("actor:killer:_energy",0.25d));
            result.add(scenario("blue-failed-shield-oxygen-"+weapon,weapon,"BLUE").set("actor:killer:_isaplant",true).set("world:_O2",0.25d));
            result.add(scenario("blue-both-deaths-wrap-"+weapon,weapon,"BLUE").set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true)
                .set("actor:victim:_energy",0.25d).set("actor:killer:_energy",0d).set("actor:victim:_nTotalKills",Integer.MAX_VALUE)
                .set("actor:killer:_nTotalKills",Integer.MAX_VALUE).setting("BLUE_ENERGY_CONSUMPTION",0d).set("world:_O2",0.25d));
            for(String target:new String[]{"GREEN","GRASS"})result.add(scenario("dodge-success-"+weapon+"-"+target,weapon,target).set("actor:killer:_dodge",true));
            result.add(scenario("dodge-partial-before-attack-"+weapon,weapon,"GRASS").set("actor:killer:_dodge",true).set("actor:killer:_energy",0.25d).set("world:_O2",0.25d));
            result.add(scenario("dodge-already-marked-"+weapon,weapon,"GRASS").set("actor:killer:_dodge",true).set("actor:killer:_hasdodged",true));
            result.add(scenario("healer-altruist-"+weapon,weapon,"MINT").set("actor:victim:_altruist",true).set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true));
            result.add(scenario("fruit-infection-"+weapon,weapon,"FRUIT").set("actor:killer:_sporeversion",2).set("actor:killer:_nTotalInfected",Integer.MAX_VALUE));
            result.add(scenario("fruit-shared-infection-"+weapon,weapon,"FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_infectedGeneticCode","g1"));
        }
        for(String weapon:new String[]{"ORANGE","FIRE"}){
            result.add(scenario("lightblue-failed-shield-"+weapon,weapon,"LIGHT_BLUE").set("actor:killer:_energy",0.25d));
            result.add(scenario("sky-partial-shield-"+weapon,weapon,"SKY").set("world:_O2",0.25d));
            result.add(scenario("bark-inactive-"+weapon,weapon,"BARK").set("actor:killer:active",false));
        }
        result.add(scenario("orange-fruit-enhanced","ORANGE","FRUIT").set("actor:killer:_sporeversion",2).set("actor:killer:_isenhanced",true));
        Spec equalCode=scenario("orange-fruit-value-equal-distinct-code","ORANGE","FRUIT").set("actor:killer:_sporeversion",2);equalCode.clonePeer=true;result.add(equalCode);
        Spec sharedCode=scenario("orange-fruit-shared-primary-code","ORANGE","FRUIT").set("actor:killer:_sporeversion",2);sharedCode.sharedPeer=true;result.add(sharedCode);
        result.add(scenario("orange-ice-own-sky","ORANGE","ICE").set("actor:victim:_skyversion",1));
        result.add(scenario("orange-ice-own-enhanced","ORANGE","ICE").set("actor:victim:_isenhanced",true));
        for(String target:new String[]{"WHITE","GRASS","GREEN","YELLOW","SILVER","VIOLET","GREENBROWN","MINT","UNKNOWN"}){
            Spec spec=scenario("fire-consumer-full-"+target,"FIRE",target).set("actor:killer:_isaconsumer",true);
            if(target.equals("GREENBROWN"))spec.set("actor:victim:_isenhanced",true);if(target.equals("GRASS"))spec.set("actor:killer:_dodge",true);result.add(spec);
        }
        result.add(scenario("fire-own-sky-frozen-plant","FIRE","GREEN").set("actor:victim:_skyversion",1).set("actor:killer:_isfrozen",true).set("actor:killer:_isaplant",true));
        result.add(scenario("fire-yellow-transfer","FIRE","YELLOW").set("actor:killer:_transfersenergy",true));
        result.add(scenario("fire-spike-weak-plant","FIRE","SPIKE").set("actor:killer:_isaplant",true));
        result.add(scenario("fire-spike-full-enhanced-consumer","FIRE","SPIKE").set("actor:killer:_isenhanced",true).set("actor:killer:_isaconsumer",true));
        result.add(scenario("fire-pink-modified-gray","FIRE","PINK").set("actor:killer:_isgray",true).set("genome:g1:_modifiespink",true));
        result.add(scenario("fire-pink-modified-without-gray","FIRE","PINK").set("genome:g1:_modifiespink",true));
        result.add(scenario("fire-cream-modified-enhanced","FIRE","CREAM").set("actor:killer:_isenhanced",true).set("genome:g1:_modifiescream",2));
        for(boolean full:new boolean[]{false,true})result.add(scenario("fire-fallow-infectious-four-"+full,"FIRE","FALLOW").set("actor:killer:_isinfectious",true).set("actor:killer:_fallowversion",4).set("actor:killer:_isaconsumer",full));
        result.add(scenario("fire-fallow-plant-weak","FIRE","FALLOW").set("actor:killer:_isaplant",true));
        for(boolean full:new boolean[]{false,true})result.add(scenario("fire-plague-infectious-"+full,"FIRE","PLAGUE").set("actor:killer:_isinfectious",true).set("actor:killer:_isaconsumer",full));
        for(String target:new String[]{"SKY","DEEPSKY"})for(int version:new int[]{1,2})result.add(scenario("fire-enhanced-sky-gate-"+target+"-"+version,"FIRE",target).set("actor:victim:_isenhanced",true).set("actor:killer:_skyversion",version));
        result.add(scenario("fire-violet-fungus","FIRE","VIOLET").set("actor:killer:_isafungus",true));
        result.add(scenario("fire-violet-plant","FIRE","VIOLET").set("actor:killer:_isaplant",true));
        result.add(scenario("fire-violet-enhanced-peer-blocks","FIRE","VIOLET").set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true));
        result.add(scenario("fire-violet-infectious-peer-bypasses","FIRE","VIOLET").set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true).set("actor:killer:_isinfectious",true));
        result.add(scenario("fire-violet-enhanced-actor-blocks","FIRE","VIOLET").set("actor:killer:_isaplant",true).set("actor:victim:_isenhanced",true));
        result.add(scenario("fire-ice-plant","FIRE","ICE").set("actor:victim:_skyversion",1).set("actor:killer:_isaplant",true));
        result.add(scenario("fire-ice-ordinary","FIRE","ICE").set("actor:victim:_skyversion",1));
        result.add(scenario("fire-ice-enhanced-without-sky","FIRE","ICE").set("actor:victim:_isenhanced",true));
        for(boolean alive:new boolean[]{false,true})for(boolean enhanced:new boolean[]{false,true})result.add(scenario("fire-brown-"+alive+"-"+enhanced,"FIRE","BROWN").set("actor:victim:_isenhanced",true).set("actor:killer:alive",alive).set("actor:killer:_isenhanced",enhanced));
        result.add(scenario("fire-greenbrown-enhanced-weak","FIRE","GREENBROWN").set("actor:victim:_isenhanced",true));
        result.add(scenario("maroon-enhanced-blue-actor-skips-retaliation","MAROON","BLUE").set("actor:victim:_isenhanced",true).set("actor:killer:_isenhanced",true).set("actor:killer:_isaplant",true));
        result.add(scenario("red-spike-enhanced-peer","RED","SPIKE").set("actor:killer:_isenhanced",true));
        result.add(scenario("red-spikepoint-both-enhanced","RED","SPIKEPOINT").set("actor:killer:_isenhanced",true).set("actor:victim:_isenhanced",true));
        result.add(scenario("red-spikepoint-peer-only","RED","SPIKEPOINT").set("actor:killer:_isenhanced",true));
        for(boolean enhanced:new boolean[]{false,true})result.add(scenario("red-gray-consumer-"+enhanced,"RED","GRAY").set("actor:victim:_isenhanced",true).set("actor:killer:_isaconsumer",true).set("actor:killer:_isenhanced",enhanced));
        result.add(scenario("red-cream-enhanced-ordinary","RED","CREAM").set("actor:victim:_isenhanced",true));
        result.add(scenario("red-cream-enhanced-modified","RED","CREAM").set("actor:victim:_isenhanced",true).set("actor:killer:_isenhanced",true).set("genome:g1:_modifiescream",2));
        result.add(scenario("red-lilac-enhanced-fungus","RED","LILAC").set("actor:victim:_isenhanced",true).set("actor:killer:_isafungus",true));
        for(boolean own:new boolean[]{false,true})for(boolean peer:new boolean[]{false,true})if(own||peer)result.add(scenario("pink-modifier-pair-"+own+"-"+peer,"PINK","PINK").set("genome:g0:_modifiespink",own).set("genome:g1:_modifiespink",peer));
        result.add(scenario("pink-fruit-weak","PINK","FRUIT").set("actor:killer:_sporeversion",2));
        result.add(scenario("pink-fruit-indigo-blocks","PINK","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_indigo",1));
        result.add(scenario("pink-fruit-negative-indigo-blocks","PINK","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_indigo",-1));
        result.add(scenario("pink-fruit-modified-overrides-indigo","PINK","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_indigo",1).set("genome:g0:_modifiespink",true));
        result.add(scenario("pink-fruit-blond-overrides-indigo","PINK","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_indigo",1).set("actor:victim:_isblond",true));
        result.add(scenario("pink-fruit-already-infected","PINK","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_infectedGeneticCode","g1"));
        result.add(scenario("pink-cream-enhanced-modified","PINK","CREAM").set("actor:killer:_isenhanced",true).set("genome:g1:_modifiescream",2));
        result.add(scenario("pink-cream-inactive-modified","PINK","CREAM").set("actor:killer:active",false).set("genome:g1:_modifiescream",2));
        result.add(scenario("pink-cream-active-modified","PINK","CREAM").set("genome:g1:_modifiescream",2));
        result.add(scenario("pink-injury-modified","PINK","GREENBROWN").set("genome:g0:_modifiespink",true));
        result.add(scenario("pink-injury-blond-plant","PINK","GREENBROWN").set("actor:killer:_isblond",true).set("actor:killer:_isaplant",true));
        result.add(scenario("pink-injury-blond-nonplant","PINK","GREENBROWN").set("actor:killer:_isblond",true));
        result.add(scenario("pink-injury-frozen","PINK","GREENBROWN").set("actor:killer:_isfrozen",true));
        result.add(scenario("pink-injury-inactive","PINK","GREENBROWN").set("actor:killer:active",false));
        result.add(scenario("pink-deadbark-blond","PINK","DEADBARK").set("actor:killer:_isblond",true));
        result.add(scenario("pink-deadbark-frozen-own-sky","PINK","DEADBARK").set("actor:killer:_isfrozen",true).set("actor:victim:_skyversion",1));
        result.add(scenario("pink-deadbark-frozen-without-sky","PINK","DEADBARK").set("actor:killer:_isfrozen",true));
        for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus"})result.add(scenario("pink-darkfire-classified-"+flag,"PINK","DARKFIRE").set("actor:killer:"+flag,true));
        result.add(scenario("pink-darkfire-modified-classified","PINK","DARKFIRE").set("actor:killer:_isaplant",true).set("genome:g0:_modifiespink",true));
        result.add(scenario("pink-darkfire-inactive-classified","PINK","DARKFIRE").set("actor:killer:_isaplant",true).set("actor:killer:active",false));
        for(boolean consumer:new boolean[]{false,true})result.add(scenario("pink-coral-modified-"+consumer,"PINK","CORAL").set("genome:g0:_modifiespink",true).set("actor:killer:_isaconsumer",consumer));
        result.add(scenario("pink-brown-gray-protection","PINK","BROWN").set("actor:victim:_isgray",true));
        result.add(scenario("pink-brown-gray-modified","PINK","BROWN").set("actor:victim:_isgray",true).set("genome:g0:_modifiespink",true));
        result.add(scenario("pink-brown-enhanced-peer","PINK","BROWN").set("actor:killer:_isenhanced",true));
        result.add(scenario("pink-brown-inactive-live-peer","PINK","BROWN").set("actor:killer:active",false));
        for(int age:new int[]{1023,1024})result.add(scenario("pink-old-corpse-shift-boundary-"+age,"PINK","BROWN").set("actor:killer:alive",false).set("actor:victim:_fallowversion",1).set("actor:killer:_age",age).set("actor:killer:_max_age",2));
        result.add(scenario("pink-old-corpse-wrapped-max-age","PINK","BROWN").set("actor:killer:alive",false).set("actor:victim:_fallowversion",1).set("actor:killer:_age",0).set("actor:killer:_max_age",Integer.MAX_VALUE));
        result.add(scenario("pink-old-corpse-without-fallow","PINK","BROWN").set("actor:killer:alive",false).set("actor:killer:_age",Integer.MAX_VALUE).set("actor:killer:_max_age",2));
        result.add(scenario("pink-plague-infectious","PINK","PLAGUE").set("actor:killer:_isinfectious",true));
        Spec pinkPrime=scenario("pink-oldbark-prime-then-drain","PINK","OLDBARK").set("actor:victim:_isenhanced",true).set("actor:killer:_isenhanced",true);pinkPrime.add(action(pinkPrime));result.add(pinkPrime);
        for(boolean modified:new boolean[]{false,true})result.add(scenario("pink-oldbark-active-enhanced-positive-timer-"+modified,"PINK","OLDBARK").set("actor:victim:_isenhanced",true).set("actor:killer:_isenhanced",true).set("actor:killer:_framesColor",5).set("genome:g0:_modifiespink",modified));
        result.add(scenario("pink-oldbark-active-enhanced-blond","PINK","OLDBARK").set("actor:victim:_isenhanced",true).set("actor:killer:_isenhanced",true).set("actor:killer:_framesColor",5).set("actor:killer:_isblond",true));
        result.add(scenario("pink-oldbark-ordinary-peer","PINK","OLDBARK").set("actor:victim:_isenhanced",true));
        result.add(scenario("pink-oldbark-modified-ordinary-peer","PINK","OLDBARK").set("actor:victim:_isenhanced",true).set("genome:g0:_modifiespink",true));
        result.add(scenario("pink-oldbark-inactive-enhanced-peer","PINK","OLDBARK").set("actor:victim:_isenhanced",true).set("actor:killer:_isenhanced",true).set("actor:killer:active",false));
        result.add(scenario("maroon-plant-white","MAROON","WHITE").set("actor:killer:_isaplant",true));
        for(boolean infectious:new boolean[]{false,true})result.add(scenario("maroon-plant-plague-"+infectious,"MAROON","PLAGUE").set("actor:killer:_isaplant",true).set("actor:killer:_isinfectious",infectious));
        result.add(scenario("maroon-inactive-broken","MAROON","BROKEN").set("actor:killer:active",false));
        result.add(scenario("maroon-plant-broken","MAROON","BROKEN").set("actor:killer:_isaplant",true));
        Spec maroonPrime=scenario("maroon-oldbark-prime-then-drain","MAROON","OLDBARK").set("actor:killer:_isenhanced",true);maroonPrime.add(action(maroonPrime));result.add(maroonPrime);
        result.add(scenario("maroon-oldbark-positive-timer","MAROON","OLDBARK").set("actor:killer:_isenhanced",true).set("actor:killer:_framesColor",5));
        for(boolean enhanced:new boolean[]{false,true})result.add(scenario("maroon-oldbark-inactive-"+enhanced,"MAROON","OLDBARK").set("actor:killer:active",false).set("actor:killer:_isenhanced",enhanced));
        for(boolean enhanced:new boolean[]{false,true})result.add(scenario("maroon-bark-inactive-"+enhanced,"MAROON","BARK").set("actor:killer:active",false).set("actor:killer:_isenhanced",enhanced));
        for(String target:new String[]{"CYAN","MINT","SKY","GREENBROWN"})result.add(scenario("maroon-enhanced-plant-"+target,"MAROON",target).set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true));
        result.add(scenario("maroon-enhanced-plant-dodges","MAROON","CYAN").set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true).set("actor:killer:_dodge",true));
        result.add(scenario("maroon-enhanced-plant-sky-fails","MAROON","SKY").set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true).set("actor:killer:_energy",0.25d));
        result.add(scenario("maroon-plant-without-enhancement","MAROON","CYAN").set("actor:killer:_isaplant",true));
        result.add(scenario("maroon-enhanced-without-plant","MAROON","CYAN").set("actor:victim:_isenhanced",true));
        result.add(scenario("maroon-enhanced-plant-silver","MAROON","SILVER").set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true));
        result.add(scenario("maroon-enhanced-plant-silver-enhanced-peer","MAROON","SILVER").set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true));
        result.add(scenario("maroon-enhanced-plant-silver-killer","MAROON","SILVER").set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true).set("actor:killer:_isaconsumer",true).set("actor:killer:_nTotalKills",1));
        result.add(scenario("maroon-enhanced-plant-spikepoint","MAROON","SPIKEPOINT").set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true));
        result.add(scenario("maroon-enhanced-plant-spikepoint-enhanced-peer","MAROON","SPIKEPOINT").set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true));
        for(String weapon:new String[]{"ORANGE","FIRE","RED","PINK","MAROON"}){
            String target=edible(weapon);double consumption=cost(weapon);
            result.add(scenario("zero-cost-and-byproduct-"+weapon,weapon,target).setting(weapon+"_ENERGY_CONSUMPTION",0d).setting("ORGANIC_SUBS_PRODUCED",0d));
            result.add(scenario("full-byproduct-"+weapon,weapon,target).setting("ORGANIC_SUBS_PRODUCED",1d));
            result.add(scenario("zero-organic-yield-"+weapon,weapon,target).setting("ORGANIC_OBTAINED_ENERGY",0d));
            result.add(scenario("payment-partial-oxygen-"+weapon,weapon,target).set("world:_O2",consumption/2d));
            result.add(scenario("payment-exact-oxygen-"+weapon,weapon,target).set("world:_O2",consumption));
            result.add(scenario("payment-empty-oxygen-"+weapon,weapon,target).set("world:_O2",0d));
            result.add(scenario("payment-negative-oxygen-"+weapon,weapon,target).set("world:_O2",-0.125d));
            result.add(scenario("payment-nan-oxygen-"+weapon,weapon,target).set("world:_O2",Double.longBitsToDouble(0xfff8000000000042L)));
            result.add(scenario("payment-infinite-oxygen-"+weapon,weapon,target).set("world:_O2",Double.POSITIVE_INFINITY));
            result.add(scenario("payment-insufficient-energy-"+weapon,weapon,target).set("actor:victim:_energy",consumption/2d));
            result.add(scenario("payment-exact-energy-"+weapon,weapon,target).set("actor:victim:_energy",consumption));
            result.add(scenario("payment-negative-energy-"+weapon,weapon,target).set("actor:victim:_energy",-1d));
            result.add(scenario("payment-nan-energy-"+weapon,weapon,target).set("actor:victim:_energy",Double.longBitsToDouble(0x7ff8000000000042L)));
            result.add(scenario("payment-infinite-energy-"+weapon,weapon,target).set("actor:victim:_energy",Double.POSITIVE_INFINITY));
            result.add(scenario("target-nan-energy-"+weapon,weapon,target).set("actor:killer:_energy",Double.longBitsToDouble(0xfff8000000000042L)));
            result.add(scenario("target-infinite-energy-"+weapon,weapon,target).set("actor:killer:_energy",Double.POSITIVE_INFINITY));
            result.add(scenario("target-negative-energy-"+weapon,weapon,target).set("actor:killer:_energy",-1d));
            for(double energy:new double[]{Math.nextDown(Utils.tol),Utils.tol})result.add(scenario("ignored-target-death-boundary-"+weapon+"-"+bits(energy),weapon,ignored(weapon)).set("actor:killer:_energy",energy));
            for(double mass:new double[]{-1d,-0d,Double.POSITIVE_INFINITY,Double.longBitsToDouble(0xfff8000000000042L)})result.add(scenario("controlled-length-"+weapon+"-"+bits(mass),weapon,target).set("actor:victim:_m",new double[]{mass,mass}));
            Spec zero=scenario("constructed-zero-length-"+weapon,weapon,target);zero.length=0;result.add(zero);
        }
        Spec selected=scenario("orange-selected-segment-zero","ORANGE","GREEN").set("actor:victim:_m",new double[]{1d,9d});selected.actions.set(0,new Action("touchEffects1.ORANGE","killer,0,0"));result.add(selected);
        Spec shieldSegment=scenario("orange-peer-shield-segment-one","ORANGE","BLUE").set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true).set("actor:victim:_m",new double[]{1d,9d}).set("actor:killer:_m",new double[]{1d,16d});shieldSegment.peerGenes=new String[]{"GREEN","GREEN"};shieldSegment.actions.set(0,new Action("touchEffects1.ORANGE","killer,1,1"));result.add(shieldSegment);
        Spec barkSegment=scenario("red-bark-photo-segment-one","RED","BARK");barkSegment.peerGenes=new String[]{"GREEN","GREEN"};barkSegment.actions.set(0,new Action("touchEffects1.RED","killer,1,1"));result.add(barkSegment);
        Spec retained=scenario("red-death-statistics-retains-live-code","RED","ORANGE").set("actor:killer:_energy",0d);retained.add(action(retained).set("genome:g1:_cladeID","dead-retained"));result.add(retained);
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.DrainOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("draining.tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("DRAIN_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
