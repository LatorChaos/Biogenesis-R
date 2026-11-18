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

/** Observe the three special touchEffects1 branches on real placed organisms */
public final class SpecialOracle {
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
        boolean passive,modifiedSky,clonePeer,sharedPeer,arithmeticDiagnostic;
        final Map<String,Object> settings=new LinkedHashMap<>(),controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        Spec(String name,String weapon,String target){this.name=name;this.weapon=weapon;this.target=target;genes=new String[]{weapon.equals("SPIKEPOINT")?"SPIKE":weapon,weapon.equals("SPIKEPOINT")?"SPIKE":weapon};}
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
            row("C",spec.name,spec.arithmeticDiagnostic?"arithmetic-diagnostic":"canonical",827364);
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
            control("setup","actor:victim:_segColor",repeatedColor(spec.weapon,victim._segments));
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
            if(target.equals("rng:seed")){Utils.random=new Random((Long)value);row("I",stage,target,"long",value);return;}
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
        void action(int index,Action action,boolean diagnostic)throws Exception{
            String stage="step-"+index;
            for(Map.Entry<String,Object> input:action.controls.entrySet())control(stage,input.getKey(),input.getValue());
            Organism victim=actors.get("victim");row("K",stage,action.operation,"victim",action.argument);
            try{invoke(action,victim,stage);}
            catch(java.lang.reflect.InvocationTargetException failure){
                if(!diagnostic||!(failure.getCause() instanceof ArithmeticException))throw failure;
                Throwable cause=failure.getCause();row("X",stage,cause.getClass().getName(),cause.getMessage());
                calls++;snapshot("failed-"+stage);return;
            }
            require(!diagnostic,"Expected arithmetic diagnostic did not throw");
            calls++;snapshot(stage);
        }
        void invoke(Action action,Organism victim,String stage)throws Exception{
            require(action.operation.startsWith("touchEffects1."),"Original touchEffects1 branch label required");
            String branch=action.operation.substring("touchEffects1.".length());require(List.of("SPIKEPOINT","OCHRE","SILVER").contains(branch),"Unsupported special branch");
            String[] arguments=action.argument.split(",");require(arguments.length==3&&arguments[0].equals("killer"),"Distinct peer and two indices required");
            int segment=Integer.parseInt(arguments[1]),peerSegment=Integer.parseInt(arguments[2]);
            Organism peer=actors.get(arguments[0]);require(peer!=victim,"Special oracle excludes self contacts");
            require(victim._segColor[segment].equals(color(branch)),"Exact actor ARGB must select the labelled special branch");
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
            for(String name:new String[]{"_isaplant","_isaconsumer","_isafungus","_isauburn","_isenhanced","_isgray","_isblond","_isfrozen","_isinjured","_isinfectious","_transfersenergy","_dodge","_hasdodged","_candodge","_altruist","_iscoral","_issilver","_isonlyc4","_useframemovement"})values.put("actor:"+role+":"+name,false);
            for(String name:new String[]{"_plagueversion","_sporeversion","_fallowversion","_skyversion","_nTotalKills","_nTotalInfected","_indigo","_lavender","_createlavender","_jadefactor","_antiviral"})values.put("actor:"+role+":"+name,0);
            values.put("actor:"+role+":_color",Color.MAGENTA);values.put("actor:"+role+":_framesColor",0);values.put("actor:"+role+":_photosynthesis",10d);
        }
        double[] actorPhoto=new double[segments],peerPhoto=new double[peerSegments];for(int i=0;i<segments;i++)actorPhoto[i]=i+0.25;for(int i=0;i<peerSegments;i++)peerPhoto[i]=i+0.25;
        values.put("actor:victim:_mphoto",actorPhoto);values.put("actor:killer:_mphoto",peerPhoto);return values;
    }
    private static Action action(Spec spec){return new Action("touchEffects1."+spec.weapon,"killer,"+(spec.genes.length*spec.symmetry-1)+",0");}
    private static Spec base(String name,String weapon,String peer){
        require(List.of("SPIKEPOINT","OCHRE","SILVER").contains(weapon),"Only special actor branches are allowed");
        return new Spec(name,weapon,peer).setting("MOSQUITO_ENERGY_CONSUMPTION",0.125d).setting("SPIKE_ENERGY_CONSUMPTION",0.25d)
            .setting("OCHRE_ENERGY_CONSUMPTION",0.5d).setting("SILVER_ENERGY_CONSUMPTION",0.375d).setting("EXPERIENCE_ENERGY_CONSUMPTION",0.625d)
            .setting("BLUE_ENERGY_CONSUMPTION",0.75d).setting("DODGE_ENERGY_CONSUMPTION",0.875d).setting("SKY_ENERGY_CONSUMPTION",1d);
    }
    private static Spec scenario(String name,String weapon,String peer){Spec spec=base(name,weapon,peer);return spec.add(action(spec));}
    private static Spec enhanced(Spec spec){return spec.set("actor:victim:_isenhanced",true);}
    private static Spec killer(Spec spec){return spec.set("actor:victim:_isaconsumer",true).set("actor:victim:_nTotalKills",4);}
    private static Spec infectious(Spec spec){return spec.set("actor:victim:_nTotalInfected",10).set("actor:killer:_nTotalInfected",0);}
    private static Spec seed(Spec spec,long seed){spec.controls.put("rng:seed",seed);return spec;}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        List<String> palette=new ArrayList<>(Arrays.asList(COLORS));palette.addAll(Arrays.asList(INTERNAL));palette.addAll(List.of("UNKNOWN","TRANSLUCENT_MAGENTA"));
        require(palette.size()==66,"Palette inventory drift");
        for(String peer:palette)result.add(enhanced(scenario("palette-SPIKEPOINT-"+peer,"SPIKEPOINT",peer)));
        for(String peer:new String[]{"OCHRE","BLUE","RED","MAROON","CREAM","CORAL","SKY","DEEPSKY","BROWN","OLIVE","GREEN","UNKNOWN"})result.add(seed(enhanced(scenario("baseline-OCHRE-"+peer,"OCHRE",peer)),0));
        for(String peer:new String[]{"SILVER","WHITE","PLAGUE","FRUIT","INDIGO","GRASS","GREEN","DARKGREEN","RED","FIRE","ORANGE","PINK","SPIKE","SPIKEPOINT","LAVENDER","MINT","MAGENTA","ROSE","BLUE","CREAM","MAROON","CORAL","OCHRE","SKY","OLIVE","OLDBARK","FALLOW","BARK","BROWN","UNKNOWN"})result.add(enhanced(scenario("baseline-SILVER-"+peer,"SILVER",peer)));

        for(String trait:new String[]{"_isaplant","_isaconsumer","_isafungus"})for(String peer:new String[]{"GREEN","GRASS","MAROON"})result.add(scenario("spike-ordinary-"+trait+"-"+peer,"SPIKEPOINT",peer).set("actor:victim:"+trait,true));
        for(String trait:new String[]{"_isaconsumer","_isafungus","_iscoral"})result.add(scenario("spike-unclassified-attacker-target-"+trait,"SPIKEPOINT","GREEN").set("actor:killer:"+trait,true));
        result.add(scenario("spike-plant-cannot-hurt-unclassified-spike","SPIKEPOINT","SPIKE").set("actor:victim:_isaplant",true));
        result.add(scenario("spike-plant-hurts-plant-spike","SPIKEPOINT","SPIKE").set("actor:victim:_isaplant",true).set("actor:killer:_isaplant",true));
        result.add(scenario("spike-consumer-hurts-enhanced-spike","SPIKEPOINT","SPIKE").set("actor:victim:_isaconsumer",true).set("actor:killer:_isenhanced",true));
        result.add(enhanced(scenario("spike-enhanced-hurts-plant-ochre","SPIKEPOINT","OCHRE").set("actor:killer:_isaplant",true)));
        result.add(enhanced(scenario("spike-enhanced-red-is-protected","SPIKEPOINT","RED").set("actor:killer:_isenhanced",true)));
        result.add(enhanced(scenario("spike-coral-uses-ordinary-cost-despite-enhancement","SPIKEPOINT","CORAL").set("actor:killer:_iscoral",true)));
        for(String trait:new String[]{"_isaplant","_isaconsumer","_isafungus","_isauburn"})result.add(enhanced(scenario("spike-white-eligible-"+trait,"SPIKEPOINT","WHITE").set("actor:killer:"+trait,true)));
        for(int version:new int[]{-1,1})result.add(enhanced(scenario("spike-white-plague-version-"+version,"SPIKEPOINT","WHITE").set("actor:killer:_plagueversion",version)));
        result.add(enhanced(scenario("spike-altruist-mint","SPIKEPOINT","MINT").set("actor:victim:_altruist",true)));
        for(boolean enhancedPeer:new boolean[]{false,true})result.add(scenario("spike-plant-brown-enhanced-peer-"+enhancedPeer,"SPIKEPOINT","BROWN").set("actor:victim:_isaplant",true).set("actor:killer:_isenhanced",enhancedPeer));
        result.add(scenario("spike-plant-leaves-corpse-untouched","SPIKEPOINT","BROWN").set("actor:victim:_isaplant",true).set("actor:killer:alive",false));
        for(String trait:new String[]{"_isaplant","_isaconsumer","_isafungus","_transfersenergy"})result.add(enhanced(scenario("spike-yellow-classified-"+trait,"SPIKEPOINT","YELLOW").set("actor:killer:"+trait,true)));
        for(boolean infectious:new boolean[]{false,true})result.add(enhanced(scenario("spike-plague-infectious-"+infectious,"SPIKEPOINT","PLAGUE").set("actor:killer:_isinfectious",infectious)));
        for(int version:new int[]{1,2})result.add(enhanced(scenario("spike-fruit-version-"+version,"SPIKEPOINT","FRUIT").set("actor:killer:_sporeversion",version)));
        result.add(enhanced(scenario("spike-fruit-antiviral-blocks-infection","SPIKEPOINT","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_antiviral",1)));
        result.add(enhanced(scenario("spike-fruit-shared-infection-identity","SPIKEPOINT","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_infectedGeneticCode","g1")));
        Spec fruitClone=enhanced(scenario("spike-fruit-cloned-infection-identity","SPIKEPOINT","FRUIT").set("actor:killer:_sporeversion",2));fruitClone.clonePeer=true;result.add(fruitClone);
        result.add(scenario("spike-ordinary-plant-fruit","SPIKEPOINT","FRUIT").set("actor:victim:_isaplant",true));
        result.add(scenario("spike-ordinary-consumer-fruit-is-ignored","SPIKEPOINT","FRUIT").set("actor:victim:_isaconsumer",true));
        for(int kills:new int[]{1,2,3})result.add(enhanced(scenario("spike-silver-kill-tint-"+kills,"SPIKEPOINT","SILVER").set("actor:victim:_nTotalKills",2).set("actor:killer:_nTotalKills",kills).set("actor:killer:_isaconsumer",true)));
        result.add(scenario("spike-ordinary-plant-fallow","SPIKEPOINT","FALLOW").set("actor:victim:_isaplant",true));
        result.add(enhanced(scenario("spike-blue-reflects-on-enhanced-attacker","SPIKEPOINT","BLUE").set("actor:killer:_isenhanced",true)));
        result.add(scenario("spike-blue-does-not-reflect-on-ordinary-consumer","SPIKEPOINT","BLUE").set("actor:victim:_isaconsumer",true).set("actor:killer:_isenhanced",true));
        result.add(enhanced(scenario("spike-blue-mutual-exhaustion-order","SPIKEPOINT","BLUE").set("actor:killer:_isenhanced",true).set("actor:victim:_energy",0.25d).set("actor:killer:_energy",0.75d)));
        result.add(enhanced(scenario("spike-sky-partial-shield-payment","SPIKEPOINT","SKY").set("world:_O2",0.5d)));
        for(boolean active:new boolean[]{false,true})result.add(scenario("spike-bark-active-"+active,"SPIKEPOINT","BARK").set("actor:killer:active",active).set("actor:killer:_growthRatio",1));
        for(String kind:new String[]{"enhanced","plant","consumer"}){
            Spec spec=scenario("spike-dodge-"+kind,"SPIKEPOINT","GREEN").set("actor:killer:_dodge",true);
            spec.set("actor:victim:"+(kind.equals("enhanced")?"_isenhanced":kind.equals("plant")?"_isaplant":"_isaconsumer"),true);result.add(spec);
        }
        result.add(enhanced(scenario("spike-dodge-partial-then-attack","SPIKEPOINT","GREEN").set("actor:killer:_dodge",true).set("world:_O2",0.4375d)));
        result.add(enhanced(scenario("spike-dodge-shortfall-keeps-oxygen-for-attacker","SPIKEPOINT","GREEN").set("actor:killer:_dodge",true).set("actor:killer:_energy",0.25d)));

        for(String peer:new String[]{"GREEN","BROWN","OLIVE"})for(long seed:new long[]{0,4096})result.add(seed(enhanced(scenario("ochre-rng-"+peer+"-"+seed,"OCHRE",peer)),seed));
        result.add(seed(enhanced(scenario("ochre-dead-brown-consumes-no-rng","OCHRE","BROWN").set("actor:killer:alive",false)),0));
        for(boolean movement:new boolean[]{false,true})result.add(seed(enhanced(scenario("ochre-damping-movement-"+movement,"OCHRE","CREAM").set("actor:victim:_useframemovement",movement).set("actor:victim:dx",3d).set("actor:victim:dy",-2d).set("actor:victim:dtheta",1d)),0));
        for(double velocity:new double[]{0d,-0d,1e-9d,1e-7d,Double.longBitsToDouble(0xfff8000000000042L)})result.add(seed(scenario("ochre-damping-value-"+bits(velocity),"OCHRE","CORAL").set("actor:victim:dx",velocity).set("actor:victim:dy",velocity).set("actor:victim:dtheta",velocity),0));
        result.add(seed(scenario("ochre-integer-center-wrap","OCHRE","CREAM").set("actor:victim:_centerX",Integer.MAX_VALUE).set("actor:victim:_centerY",Integer.MIN_VALUE).set("actor:killer:_centerX",Integer.MIN_VALUE).set("actor:killer:_centerY",Integer.MAX_VALUE).set("actor:victim:_dCenterX",-999d).set("actor:killer:_dCenterX",999d).set("actor:victim:_m",new double[]{1d,1d}).set("actor:killer:_mass",1d),0));
        for(String peer:new String[]{"SKY","DEEPSKY"})for(boolean plant:new boolean[]{false,true})result.add(seed(enhanced(scenario("ochre-sky-one-"+peer+"-plant-"+plant,"OCHRE",peer).set("actor:victim:_isaplant",plant).set("actor:killer:_skyversion",1).set("actor:killer:dx",3d).set("actor:killer:dy",4d)),0));
        for(String peer:new String[]{"OCHRE","MAROON"})result.add(seed(enhanced(scenario("ochre-enhanced-peer-"+peer,"OCHRE",peer).set("actor:killer:_isenhanced",true)),0));
        result.add(seed(enhanced(scenario("ochre-red-create-lavender-blocks-damage","OCHRE","RED").set("actor:killer:_createlavender",1)),0));
        for(int shield:new int[]{185,186})result.add(seed(enhanced(scenario("ochre-shield-threshold-"+shield,"OCHRE","MAROON").set("actor:killer:_lavender",shield)),0));
        result.add(seed(enhanced(scenario("ochre-blue-partial-payment-then-shield","OCHRE","BLUE").set("world:_O2",0.375d).set("actor:killer:_lavender",186)),0));
        result.add(seed(enhanced(scenario("ochre-blue-shortfall-damage","OCHRE","BLUE").set("actor:killer:_energy",0.5d)),0));
        for(double mass:new double[]{0d,-1d,Double.POSITIVE_INFINITY,Double.longBitsToDouble(0xfff8000000000042L)})result.add(seed(enhanced(scenario("ochre-controlled-mass-"+bits(mass),"OCHRE","GREEN").set("actor:killer:_mass",mass).set("actor:killer:_centerX",3500).set("actor:killer:_centerY",3500)),0));
        for(double cap:new double[]{0d,-1d})result.add(seed(enhanced(scenario("ochre-hidden-velocity-cap-"+bits(cap),"OCHRE","GREEN").setting("MAX_VEL",cap)),0));
        result.add(seed(enhanced(scenario("ochre-nonplant-hundredth-cost","OCHRE","CORAL")),0));
        result.add(seed(enhanced(scenario("ochre-plant-full-cost","OCHRE","CORAL").set("actor:victim:_isaplant",true)),0));

        for(String peer:new String[]{"SILVER","WHITE","PLAGUE","GRASS","GREEN","DARKGREEN","RED","FIRE","ORANGE","PINK","SPIKE","MAGENTA","BLUE","CREAM","MAROON","BARK","UNKNOWN"})result.add(infectious(scenario("silver-infection-group-"+peer,"SILVER",peer).set("actor:killer:_isaplant",true)));
        result.add(infectious(scenario("silver-infects-experienced-fungus","SILVER","SILVER").set("actor:killer:_isafungus",true).set("actor:killer:_nTotalKills",1)));
        result.add(infectious(scenario("silver-does-not-infect-inexperienced-fungus","SILVER","SILVER").set("actor:killer:_isafungus",true)));
        result.add(infectious(scenario("silver-pink-modifier-permits-infection","SILVER","PINK").set("genome:g1:_modifiespink",true)));
        result.add(infectious(scenario("silver-shared-infection-skips-payment","SILVER","RED").set("actor:killer:_infectedGeneticCode","g0")));
        result.add(infectious(scenario("silver-cloned-infection-remains-distinct","SILVER","RED").set("actor:killer:_infectedGeneticCode","g3")));
        Spec shared=infectious(scenario("silver-shared-primary-genotype","SILVER","RED"));shared.sharedPeer=true;result.add(shared);
        result.add(scenario("silver-infection-tie-does-not-pay","SILVER","RED").set("actor:victim:_nTotalInfected",1).set("actor:killer:_nTotalInfected",1));
        result.add(scenario("silver-infection-increment-wraps-before-indigo-test","SILVER","SILVER").set("actor:victim:_nTotalInfected",Integer.MAX_VALUE).set("actor:killer:_nTotalInfected",Integer.MIN_VALUE).set("actor:killer:_isaplant",true).set("actor:killer:_indigo",1));
        for(String peer:new String[]{"SILVER","WHITE","PLAGUE","INDIGO"})result.add(infectious(scenario("silver-indigo-"+peer,"SILVER",peer).set("actor:killer:_indigo",1)));
        result.add(infectious(scenario("silver-indigo-own-indigo-blocks","SILVER","INDIGO").set("actor:victim:_indigo",1)));
        for(String trait:new String[]{"_isaplant","_isaconsumer","_isafungus"})result.add(infectious(scenario("silver-indigo-own-override-"+trait,"SILVER","INDIGO").set("actor:victim:_indigo",1).set("actor:victim:"+trait,true)));
        result.add(infectious(scenario("silver-indigo-partial-cost","SILVER","INDIGO").set("world:_O2",0.125d)));
        for(String peer:new String[]{"UNKNOWN","WHITE","CREAM"})result.add(scenario("silver-fungus-assignment-gate-"+peer,"SILVER",peer).set("actor:victim:_nTotalKills",1).set("actor:victim:_isafungus",true));
        result.add(enhanced(scenario("silver-enhancement-still-evaluates-fungus-assignment","SILVER","UNKNOWN").set("actor:victim:_nTotalKills",1).set("actor:victim:_isafungus",true)));
        for(int[] pair:new int[][]{{1,2},{2,1},{Integer.MAX_VALUE,0},{Integer.MIN_VALUE,Integer.MIN_VALUE},{Integer.MAX_VALUE-11,-13},{0,-13},{-13,0}})result.add(enhanced(scenario("silver-integer-quotient-"+pair[0]+"-"+pair[1],"SILVER","RED").set("actor:victim:_nTotalKills",pair[0]).set("actor:killer:_nTotalKills",pair[1])));
        for(String trait:new String[]{"_isaconsumer","_isafungus"})result.add(killer(scenario("silver-experienced-recipient-tint-"+trait,"SILVER","SILVER").set("actor:killer:"+trait,true).set("actor:killer:_nTotalKills",1)));
        result.add(killer(scenario("silver-enhanced-recipient-tint","SILVER","SILVER").set("actor:killer:_isenhanced",true)));
        for(String peer:new String[]{"WHITE","PLAGUE"})for(String trait:new String[]{"_isaplant","_isaconsumer","_isafungus","_isauburn"})result.add(killer(scenario("silver-white-drain-"+peer+"-"+trait,"SILVER",peer).set("actor:killer:"+trait,true)));
        result.add(killer(scenario("silver-white-plague-version-drain","SILVER","WHITE").set("actor:killer:_plagueversion",1)));
        for(String peer:new String[]{"SPIKE","SPIKEPOINT"})for(int kills:new int[]{3,4,5})result.add(killer(scenario("silver-spike-kill-priority-"+peer+"-"+kills,"SILVER",peer).set("actor:killer:_nTotalKills",kills)));
        result.add(killer(scenario("silver-spike-enhanced-recipient","SILVER","SPIKE").set("actor:killer:_isenhanced",true)));
        result.add(killer(scenario("silver-spikepoint-enhanced-recipient","SILVER","SPIKEPOINT").set("actor:killer:_isenhanced",true).set("actor:killer:_nTotalKills",3)));
        result.add(enhanced(scenario("silver-fruit-two-ignores-antiviral","SILVER","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_antiviral",99)));
        result.add(enhanced(scenario("silver-fruit-existing-code-prevents-reinfection","SILVER","FRUIT").set("actor:killer:_sporeversion",2).set("actor:victim:_infectedGeneticCode","g1")));
        for(int version:new int[]{1,3})result.add(enhanced(scenario("silver-fruit-other-version-"+version,"SILVER","FRUIT").set("actor:killer:_sporeversion",version)));
        for(String peer:new String[]{"LAVENDER","MINT","MAGENTA","ROSE"})result.add(killer(infectious(scenario("silver-altruist-"+peer,"SILVER",peer).set("actor:victim:_altruist",true).set("actor:killer:_isaplant",true))));
        for(String peer:new String[]{"GRASS","JADE","DARKJADE","GREEN"})result.add(infectious(scenario("silver-jade-defense-"+peer,"SILVER",peer).set("actor:killer:_jadefactor",2)));
        for(boolean plant:new boolean[]{false,true})for(boolean auburn:new boolean[]{false,true})for(int frames:new int[]{8,9})result.add(infectious(scenario("silver-c4-dodge-"+plant+"-"+auburn+"-"+frames,"SILVER","C4").set("actor:victim:_isaplant",plant).set("actor:killer:_isonlyc4",true).set("actor:killer:_isauburn",auburn).set("actor:killer:_framesColor",frames).set("actor:killer:_dodge",true)));
        result.add(killer(infectious(scenario("silver-two-dodge-payments","SILVER","GREEN").set("actor:killer:_dodge",true))));
        result.add(killer(infectious(scenario("silver-first-dodge-partial-then-infection-and-attack","SILVER","GRASS").set("actor:killer:_dodge",true).set("world:_O2",0.4375d))));
        result.add(killer(infectious(scenario("silver-first-infection-cost-blocks-later-attack-payment","SILVER","RED").set("world:_O2",0.375d))));
        result.add(killer(infectious(scenario("silver-blue-infection-precedes-enhanced-reflection","SILVER","BLUE").set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true))));
        result.add(killer(scenario("silver-blue-mutual-death-order","SILVER","BLUE").set("actor:killer:_isenhanced",true).set("actor:victim:_energy",0.25d).set("actor:killer:_energy",0.75d)));
        result.add(killer(scenario("silver-blue-failed-shield-leaves-partial-cost","SILVER","BLUE").set("world:_O2",0.375d)));
        for(boolean plant:new boolean[]{false,true})result.add(infectious(scenario("silver-bark-attacker-plant-"+plant,"SILVER","BARK").set("actor:victim:_isaplant",plant)));

        for(String weapon:new String[]{"SPIKEPOINT","OCHRE","SILVER"}){
            String target=weapon.equals("SILVER")?"RED":"GREEN";
            double cost=weapon.equals("SPIKEPOINT")?0.125d:weapon.equals("OCHRE")?0.005d:0.625d;
            for(String state:new String[]{"partial-oxygen","empty-oxygen","negative-oxygen","nan-oxygen","infinite-oxygen","short-energy","exact-energy","nan-energy","infinite-energy","negative-target","nan-target","infinite-target"}){
                Spec spec=seed(enhanced(scenario("payment-"+weapon+"-"+state,weapon,target)),0);
                switch(state){
                    case "partial-oxygen":spec.set("world:_O2",cost/2);break;case "empty-oxygen":spec.set("world:_O2",0d);break;
                    case "negative-oxygen":spec.set("world:_O2",-0.125d);break;case "nan-oxygen":spec.set("world:_O2",Double.longBitsToDouble(0xfff8000000000042L));break;
                    case "infinite-oxygen":spec.set("world:_O2",Double.POSITIVE_INFINITY);break;case "short-energy":spec.set("actor:victim:_energy",cost/2);break;
                    case "exact-energy":spec.set("actor:victim:_energy",cost);break;case "nan-energy":spec.set("actor:victim:_energy",Double.longBitsToDouble(0x7ff8000000000042L));break;
                    case "infinite-energy":spec.set("actor:victim:_energy",Double.POSITIVE_INFINITY);break;case "negative-target":spec.set("actor:killer:_energy",-1d);break;
                    case "nan-target":spec.set("actor:killer:_energy",Double.longBitsToDouble(0xfff8000000000042L));break;case "infinite-target":spec.set("actor:killer:_energy",Double.POSITIVE_INFINITY);break;
                }result.add(spec);
            }
            String ignored=weapon.equals("SPIKEPOINT")?"CREAM":weapon.equals("OCHRE")?"OLIVE":"BROWN";
            for(double energy:new double[]{Math.nextDown(Utils.tol),Utils.tol,Double.NaN})result.add(scenario("common-death-"+weapon+"-"+bits(energy),weapon,ignored).set("actor:killer:_energy",energy));
            result.add(scenario("common-death-counter-wrap-"+weapon,weapon,ignored).set("actor:killer:_energy",0d).set("actor:victim:_nTotalKills",Integer.MAX_VALUE));
            for(double length:new double[]{-1d,-0d,Double.POSITIVE_INFINITY,Double.longBitsToDouble(0xfff8000000000042L)})result.add(seed(enhanced(scenario("controlled-length-"+weapon+"-"+bits(length),weapon,target).set("actor:victim:_m",new double[]{length,length})),0));
        }
        Spec segmentZero=enhanced(scenario("spike-selected-segment-zero","SPIKEPOINT","GREEN").set("actor:victim:_m",new double[]{1d,9d}));segmentZero.actions.set(0,new Action("touchEffects1.SPIKEPOINT","killer,0,0"));result.add(segmentZero);
        Spec peerSegment=killer(scenario("silver-peer-blue-segment-one","SILVER","BLUE").set("actor:killer:_isenhanced",true).set("actor:killer:_m",new double[]{1d,16d}));peerSegment.peerGenes=new String[]{"GREEN","GREEN"};peerSegment.actions.set(0,new Action("touchEffects1.SILVER","killer,1,1"));result.add(peerSegment);
        Spec repeated=infectious(scenario("silver-repeat-uses-shared-code-identity","SILVER","CREAM").set("actor:killer:_isaplant",true));repeated.add(action(repeated));result.add(repeated);
        result.add(killer(scenario("silver-preserves-world-clock-input","SILVER","BROWN").set("world:nFrames",Integer.MAX_VALUE)));
        return result;
    }
    private static List<Spec> diagnostics(){
        Spec spec=enhanced(scenario("division-by-wrapped-zero-after-payment","SILVER","RED").set("actor:victim:_nTotalKills",1).set("actor:killer:_nTotalKills",-12));
        spec.arithmeticDiagnostic=true;return List.of(spec);
    }
    private static String capture(boolean diagnostic)throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:diagnostic?diagnostics():canonical()){
            Capture capture=new Capture(spec);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index),spec.arithmeticDiagnostic);
            output.append(capture.finish());
        }return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length<1||args.length>2)throw new IllegalArgumentException("Usage: biogenesis.SpecialOracle NEW_OUTPUT_DIRECTORY [diagnostic]");
        boolean diagnostic=args.length==2;require(!diagnostic||args[1].equals("diagnostic"),"Unknown oracle mode");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        Files.writeString(directory.resolve(diagnostic?"special_contacts_invalid.tsv":"special_contacts.tsv"),capture(diagnostic),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("SPECIAL_ORACLE "+(diagnostic?"EXPECTED_ARITHMETIC_FAILURE":"PASS")+" cases="+cases+" snapshots="+snapshots+" calls="+calls);
        if(diagnostic)System.exit(20);
    }
}
