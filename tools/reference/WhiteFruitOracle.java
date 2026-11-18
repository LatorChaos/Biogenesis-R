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

/** Observe the WHITE and FRUIT touchEffects2 branches on real placed organisms */
public final class WhiteFruitOracle {
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
            require(fields(Organism.class).length==153,"Unexpected organism schema");
            require(fields(Gene.class).length==40,"Unexpected gene schema");
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
            require(action.operation.startsWith("touchEffects2."),"Original touchEffects2 branch label required");
            String branch=action.operation.substring("touchEffects2.".length());require(List.of("WHITE","FRUIT").contains(branch),"Unsupported WHITE/FRUIT branch");
            String[] arguments=action.argument.split(",");require(arguments.length==3&&arguments[0].equals("killer"),"Distinct peer and two indices required");
            int segment=Integer.parseInt(arguments[1]),peerSegment=Integer.parseInt(arguments[2]);
            Organism peer=actors.get(arguments[0]);require(peer!=victim,"WHITE/FRUIT oracle excludes self contacts");
            require(victim._segColor[segment].equals(color(branch)),"Exact actor ARGB must select the labelled contact branch");
            Method method=Organism.class.getDeclaredMethod("touchEffects2",Organism.class,int.class,int.class);method.setAccessible(true);
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
            for(String name:new String[]{"_isaplant","_isaconsumer","_isafungus","_isauburn","_isenhanced","_isgray","_isblond","_isfrozen","_isinjured","_isinfectious","_transfersenergy","_dodge","_hasdodged","_candodge","_altruist","_isakiller","_issilver","_isonlyc4"})values.put("actor:"+role+":"+name,false);
            for(String name:new String[]{"_plagueversion","_sporeversion","_fallowversion","_skyversion","_nTotalKills","_nTotalInfected","_indigo","_antiviral","_lavender","_jadefactor","_healing","_reproducelate"})values.put("actor:"+role+":"+name,0);
            values.put("actor:"+role+":_color",Color.MAGENTA);values.put("actor:"+role+":_framesColor",0);values.put("actor:"+role+":_photosynthesis",10d);
        }
        double[] actorPhoto=new double[segments],peerPhoto=new double[peerSegments];for(int i=0;i<segments;i++)actorPhoto[i]=i+0.25;for(int i=0;i<peerSegments;i++)peerPhoto[i]=i+0.25;
        values.put("actor:victim:_mphoto",actorPhoto);values.put("actor:killer:_mphoto",peerPhoto);return values;
    }
    private static Action action(Spec spec){return new Action("touchEffects2."+spec.weapon,"killer,"+(spec.genes.length*spec.symmetry-1)+",0");}
    private static Spec base(String name,String weapon,String peer){
        require(List.of("WHITE","FRUIT").contains(weapon),"Only WHITE and FRUIT actor branches are allowed");
        return new Spec(name,weapon,peer).setting("WHITE_ENERGY_CONSUMPTION",0.125d).setting("VIRUS_ENERGY_CONSUMPTION",0.25d)
            .setting("BLUE_ENERGY_CONSUMPTION",0.375d).setting("DODGE_ENERGY_CONSUMPTION",0.625d).setting("SKY_ENERGY_CONSUMPTION",0.75d)
            .setting("MINT_ENERGY_CONSUMPTION",1.25d).setting("LAVENDER_ENERGY_CONSUMPTION",2d).setting("INDIGO_ENERGY_CONSUMPTION",0.5d)
            .set("actor:victim:_isaplant",true).set("actor:victim:_isinfectious",true)
            .set("actor:victim:_sporeversion",weapon.equals("FRUIT")?1:0)
            .set("actor:killer:_isaplant",true).set("actor:killer:_infectedGeneticCode","~").set("actor:killer:_savedGeneticCode","g4");
    }
    private static Spec scenario(String name,String weapon,String peer){Spec spec=base(name,weapon,peer);return spec.add(action(spec));}
    private static Spec virus(String name,String peer){return scenario(name,"WHITE",peer).set("actor:victim:_isaplant",false);}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(String weapon:new String[]{"WHITE","FRUIT"}){
            for(String peer:COLORS)result.add(scenario("palette-"+weapon+"-"+peer,weapon,peer));
            for(String peer:INTERNAL)result.add(scenario("palette-"+weapon+"-"+peer,weapon,peer));
            for(String peer:new String[]{"UNKNOWN","TRANSLUCENT_MAGENTA"})result.add(scenario("palette-"+weapon+"-"+peer,weapon,peer));
            for(String ref:new String[]{"g0","g3","g1","~"})result.add(scenario("identity-"+weapon+"-"+ref,weapon,"GREEN").set("actor:killer:_infectedGeneticCode",ref));
            Spec shared=scenario("shared-primary-"+weapon,weapon,"GREEN");shared.sharedPeer=true;result.add(shared);
            for(String role:new String[]{"victim","killer"})result.add(scenario("infection-counter-wrap-"+weapon+"-"+role,weapon,"GREEN").set("actor:"+role+":_nTotalInfected",Integer.MAX_VALUE));
            result.add(scenario("infection-clears-peer-saved-"+weapon,weapon,"GREEN").set("actor:killer:_infectedGeneticCode","g3").set("actor:killer:_savedGeneticCode","g4"));
        }
        for(String peer:new String[]{"WHITE","SILVER","PLAGUE"}){
            result.add(scenario("white-consumer-"+peer,"WHITE",peer).set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",true));
            result.add(virus("virus-consumer-"+peer,peer).set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",true));
            result.add(scenario("white-nonhost-indigo-"+peer,"WHITE",peer).set("actor:killer:_isaplant",false).set("actor:killer:_indigo",1));
            result.add(scenario("white-nonhost-fungus-indigo-"+peer,"WHITE",peer).set("actor:killer:_isaplant",false).set("actor:killer:_isafungus",true).set("actor:killer:_indigo",1));
        }
        for(String flag:new String[]{"_isaconsumer","_isafungus"})result.add(virus("virus-indigo-exception-"+flag,"INDIGO").set("actor:victim:_indigo",1).set("actor:victim:"+flag,true));
        result.add(virus("virus-indigo-own-indigo-block","INDIGO").set("actor:victim:_indigo",1));
        result.add(scenario("white-indigo-already-infected-antiviral","WHITE","INDIGO").set("actor:killer:_infectedGeneticCode","g0").set("actor:killer:_antiviral",1).set("actor:victim:_plagueversion",1));
        result.add(scenario("white-indigo-antiviral-without-plague","WHITE","INDIGO").set("actor:killer:_antiviral",1));
        result.add(scenario("white-indigo-zero-divisor","WHITE","INDIGO").setting("INDIGO_ENERGY_CONSUMPTION",0d));
        for(int experience:new int[]{-1,1,Integer.MIN_VALUE,Integer.MAX_VALUE})result.add(scenario("white-silver-experience-"+experience,"WHITE","SILVER").set("actor:killer:_nTotalInfected",experience));
        result.add(scenario("white-silver-enhanced-bypass","WHITE","SILVER").set("actor:killer:_nTotalInfected",1).set("actor:killer:_isenhanced",true));
        for(int kills:new int[]{0,1,-1})result.add(scenario("white-silver-fungus-kills-"+kills,"WHITE","SILVER").set("actor:killer:_isaplant",false).set("actor:killer:_isafungus",true).set("actor:killer:_nTotalKills",kills));
        for(String flag:new String[]{"_isinfectious","_isaconsumer","_isafungus"})result.add(scenario("white-plague-full-threshold-"+flag,"WHITE","PLAGUE").set("actor:killer:"+flag,true));
        for(int version:new int[]{-1,0,1,2,3,4})result.add(scenario("white-spore-version-"+version,"WHITE","SPORE").set("actor:killer:_sporeversion",version));
        for(String flag:new String[]{"_isaconsumer","_isafungus"})result.add(scenario("white-spore-positive-threshold-"+flag,"WHITE","SPORE").set("actor:killer:_sporeversion",3).set("actor:killer:"+flag,true));
        result.add(virus("virus-spore-negation","SPORE").set("actor:killer:_sporeversion",3).set("actor:victim:_reproduceEnergy",Integer.MIN_VALUE));
        for(String weapon:new String[]{"WHITE","FRUIT"}){
            for(int frames:new int[]{8,9}){
                result.add(scenario("dodge-old-timer-"+weapon+"-"+frames,weapon,"GREEN").set("actor:killer:_dodge",true).set("actor:killer:_isonlyc4",true).set("actor:killer:_framesColor",frames));
                if(weapon.equals("WHITE")){
                    result.add(virus("virus-dodge-auburn-"+frames,"GREEN").set("actor:killer:_dodge",true).set("actor:killer:_isonlyc4",true).set("actor:killer:_isauburn",true).set("actor:killer:_framesColor",frames));
                    result.add(scenario("white-dodge-auburn-"+frames,"WHITE","GREEN").set("actor:killer:_dodge",true).set("actor:killer:_isonlyc4",true).set("actor:killer:_isauburn",true).set("actor:killer:_framesColor",frames));
                }
            }
            for(String peer:new String[]{"GREEN","DARKGREEN","SPORE","MAGENTA"})result.add(scenario("dodge-group-"+weapon+"-"+peer,weapon,peer).set("actor:killer:_sporeversion",3).set("actor:killer:_dodge",true));
            result.add(scenario("dodge-partial-then-infect-"+weapon,weapon,"GREEN").set("actor:killer:_dodge",true).set("actor:killer:_energy",0.125d));
            for(String peer:new String[]{"GREEN","JADE","DARKJADE"})result.add(scenario("jade-defence-"+weapon+"-"+peer,weapon,peer).set("actor:killer:_jadefactor",2));
            result.add(scenario("jade-after-partial-dodge-"+weapon,weapon,"JADE").set("actor:killer:_jadefactor",2).set("actor:killer:_dodge",true).set("world:_O2",0.3125d));
            result.add(scenario("rose-altruist-"+weapon,weapon,"ROSE").set("actor:victim:_altruist",true));
            result.add(scenario("nonplant-default-"+weapon,weapon,"UNKNOWN").set("actor:killer:_isaplant",false));
        }
        result.add(virus("virus-dodge-nonauburn-old-timer","GREEN").set("actor:killer:_dodge",true).set("actor:killer:_isonlyc4",true).set("actor:killer:_framesColor",8));
        result.add(scenario("white-jade-neutralizes-plague","WHITE","JADE").set("actor:killer:_jadefactor",2).set("actor:killer:_antiviral",1).set("actor:victim:_plagueversion",1));
        result.add(scenario("white-green-nonplant-host","WHITE","GREEN").set("actor:killer:_isaplant",false));
        result.add(scenario("fruit-green-nonplant-host","FRUIT","GREEN").set("actor:killer:_isaplant",false));
        result.add(scenario("white-darkgreen-nonhost","WHITE","DARKGREEN").set("actor:killer:_isaplant",false));
        result.add(scenario("white-blue-enhanced-drain","WHITE","BLUE").set("actor:killer:_isenhanced",true));
        result.add(scenario("white-blue-enhanced-death","WHITE","BLUE").set("actor:killer:_isenhanced",true).set("actor:victim:_energy",1d));
        result.add(scenario("white-blue-partial-defence","WHITE","BLUE").set("actor:killer:_energy",0.125d));
        result.add(scenario("white-blue-shared-infection","WHITE","BLUE").set("actor:killer:_infectedGeneticCode","g0"));
        for(double length:new double[]{-1d,-0d,Double.POSITIVE_INFINITY,Double.longBitsToDouble(0xfff8000000000042L)})result.add(scenario("white-blue-raw-length-"+bits(length),"WHITE","BLUE").set("actor:killer:_isenhanced",true).set("actor:killer:_m",new double[]{length}));
        result.add(scenario("white-lightblue-enhanced-bypass","WHITE","LIGHT_BLUE").set("actor:victim:_isenhanced",true));
        result.add(virus("virus-lightblue-skips-shield-cost","LIGHT_BLUE"));
        result.add(scenario("white-lightblue-partial-defence","WHITE","LIGHT_BLUE").set("actor:killer:_energy",0.125d));
        for(String peer:new String[]{"SKY","DEEPSKY"}){
            result.add(virus("virus-skyversion2-"+peer,peer).set("actor:killer:_skyversion",2));
            result.add(scenario("white-sky-frozen-"+peer,"WHITE",peer).set("actor:victim:_isfrozen",true));
            result.add(scenario("white-sky-healing-"+peer,"WHITE",peer).set("actor:victim:_isfrozen",true).set("actor:victim:_healing",1));
            result.add(scenario("white-sky-enhanced-frozen-bypass-"+peer,"WHITE",peer).set("actor:victim:_isfrozen",true).set("actor:victim:_isenhanced",true));
            result.add(scenario("white-sky-killer-block-"+peer,"WHITE",peer).set("actor:victim:_isfrozen",true).set("actor:victim:_isenhanced",true).set("actor:victim:_isakiller",true));
            result.add(scenario("white-sky-antiviral-block-"+peer,"WHITE",peer).set("actor:victim:_isfrozen",true).set("actor:victim:_isenhanced",true).set("actor:killer:_antiviral",1));
            result.add(scenario("white-sky-consumer-v2-"+peer,"WHITE",peer).set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",true).set("actor:killer:_skyversion",2));
            result.add(scenario("white-sky-already-infected-"+peer,"WHITE",peer).set("actor:killer:_infectedGeneticCode","g0"));
        }
        result.add(scenario("white-sky-partial-shield","WHITE","SKY").set("actor:killer:_energy",0.25d));
        result.add(scenario("white-deepsky-enhanced-infects","WHITE","DEEPSKY").set("actor:victim:_isenhanced",true));
        for(String peer:new String[]{"OLIVE","DARKOLIVE","VIOLET","ICE","DEADBARK"}){
            result.add(scenario("white-rigid-enhanced-"+peer,"WHITE",peer).set("actor:victim:_isenhanced",true));
            result.add(scenario("white-rigid-killer-"+peer,"WHITE",peer).set("actor:victim:_isenhanced",true).set("actor:victim:_isakiller",true));
            result.add(virus("virus-rigid-"+peer,peer));
        }
        result.add(scenario("white-violet-consumer","WHITE","VIOLET").set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",true));
        result.add(scenario("white-ice-consumer-antiviral","WHITE","ICE").set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",true).set("actor:killer:_antiviral",1));
        for(String peer:new String[]{"VIOLET","LIGHTBROWN"}){
            result.add(virus("virus-auburn-fungus-"+peer,peer).set("actor:victim:_isauburn",true).set("actor:killer:_isaplant",false).set("actor:killer:_isafungus",true));
            result.add(virus("virus-auburn-silver-fungus-"+peer,peer).set("actor:victim:_isauburn",true).set("actor:killer:_isaplant",false).set("actor:killer:_isafungus",true).set("actor:killer:_issilver",true));
        }
        for(String weapon:new String[]{"WHITE","FRUIT"}){
            result.add(scenario("lavender-enhanced-"+weapon,weapon,"LAVENDER").set("actor:killer:_isenhanced",true));
            result.add(scenario("lavender-foreign-infection-"+weapon,weapon,"LAVENDER").set("actor:killer:_infectedGeneticCode","g3"));
            result.add(scenario("lavender-owned-infection-"+weapon,weapon,"LAVENDER").set("actor:killer:_infectedGeneticCode","g0"));
            result.add(scenario("lavender-saved-only-"+weapon,weapon,"LAVENDER").set("actor:killer:_infectedGeneticCode","~").set("actor:killer:_savedGeneticCode","g4"));
            result.add(scenario("lavender-failed-payment-refill-"+weapon,weapon,"LAVENDER").set("world:_O2",0d).set("actor:killer:_infectedGeneticCode","g3"));
            result.add(scenario("lavender-partial-payment-"+weapon,weapon,"LAVENDER").set("world:_O2",0.0625d).set("actor:killer:_isenhanced",true));
            result.add(scenario("lavender-kills-actor-"+weapon,weapon,"LAVENDER").set("actor:victim:_energy",0.01d));
            result.add(scenario("lavender-at-cap-"+weapon,weapon,"LAVENDER").set("actor:killer:_lavender",Utils.DEF_LAVENDER_SHIELD));
            result.add(scenario("lavender-wrap-"+weapon,weapon,"LAVENDER").setting("LAVENDER_SHIELD",Integer.MAX_VALUE).set("actor:killer:_isenhanced",true).set("actor:killer:_lavender",Integer.MAX_VALUE-10));
            result.add(scenario("lavender-zero-divisor-"+weapon,weapon,"LAVENDER").setting("LAVENDER_ENERGY_CONSUMPTION",0d));
        }
        result.add(scenario("white-lavender-plague-refill","WHITE","LAVENDER").set("actor:victim:_plagueversion",1));
        result.add(scenario("white-lavender-fallow-refill","WHITE","LAVENDER").set("actor:victim:_fallowversion",1));
        result.add(scenario("white-pink-modified","WHITE","PINK").set("genome:g1:_modifiespink",true));
        result.add(scenario("white-pink-late-boundary","WHITE","PINK").set("actor:killer:_timeToReproduce",10).set("actor:killer:_timeToReproduceMax",10));
        result.add(scenario("white-pink-overdue","WHITE","PINK").set("actor:killer:_timeToReproduce",11).set("actor:killer:_timeToReproduceMax",10));
        result.add(scenario("white-pink-overdue-delayed","WHITE","PINK").set("actor:killer:_timeToReproduce",11).set("actor:killer:_timeToReproduceMax",10).set("actor:killer:_reproducelate",1));
        result.add(virus("virus-auburn-pink","PINK").set("actor:victim:_isauburn",true));
        for(boolean plant:new boolean[]{true,false}){
            for(int cream:new int[]{1,2,3})result.add(scenario("white-cream-enhanced-"+plant+"-"+cream,"WHITE","CREAM").set("actor:victim:_isaplant",plant).set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",plant).set("actor:killer:_isenhanced",true).set("genome:g1:_modifiescream",cream));
        }
        result.add(virus("virus-cream-plant","CREAM"));
        result.add(scenario("white-cream-indigo-block","WHITE","CREAM").set("actor:victim:_isenhanced",true).set("actor:killer:_indigo",1));
        result.add(scenario("white-cream-killer-block","WHITE","CREAM").set("actor:victim:_isenhanced",true).set("actor:victim:_isakiller",true));
        result.add(virus("virus-spike-enhanced-host","SPIKE").set("actor:killer:_isaplant",false).set("actor:killer:_isenhanced",true));
        result.add(scenario("white-spikepoint-enhanced","WHITE","SPIKEPOINT").set("actor:victim:_isenhanced",true));
        result.add(scenario("white-spikepoint-antiviral-block","WHITE","SPIKEPOINT").set("actor:victim:_isenhanced",true).set("actor:killer:_antiviral",1));
        result.add(scenario("white-spikepoint-enhanced-host-bypass","WHITE","SPIKEPOINT").set("actor:victim:_isenhanced",true).set("actor:killer:_antiviral",1).set("actor:killer:_isenhanced",true));
        result.add(virus("virus-coral-blond-block","CORAL").set("actor:victim:_isblond",true));
        result.add(virus("virus-coral-consumer-bypass","CORAL").set("actor:victim:_isblond",true).set("actor:victim:_isaconsumer",true));
        result.add(virus("virus-fallow-enhanced","FALLOW").set("actor:victim:_isenhanced",true));
        result.add(scenario("white-fallow-plant-block","WHITE","FALLOW").set("actor:victim:_isenhanced",true));
        result.add(scenario("white-bark-inactive","WHITE","BARK").set("actor:killer:active",false));
        Spec bark=scenario("white-bark-second-segment","WHITE","BARK").set("actor:killer:_mphoto",new double[]{1d,4d});bark.peerGenes=new String[]{"GREEN","GREEN"};bark.actions.set(0,new Action("touchEffects2.WHITE","killer,1,1"));result.add(bark);
        for(int version:new int[]{-1,0,2,3})result.add(scenario("fruit-actor-spore-gate-"+version,"FRUIT","GREEN").set("actor:victim:_sporeversion",version));
        result.add(scenario("fruit-dormant-host-block","FRUIT","GREEN").set("actor:killer:_sporeversion",1));
        result.add(scenario("fruit-infected-dormant-host","FRUIT","GREEN").set("actor:killer:_sporeversion",1).set("actor:killer:_infectedGeneticCode","g3"));
        result.add(scenario("fruit-antiviral-breaks-body","FRUIT","GREEN").set("actor:killer:_antiviral",1).set("actor:victim:_segColor",phenotype("FRUIT","DARKFIRE","UNKNOWN","TRANSLUCENT_MAGENTA","GREEN")).colors("FRUIT","DARKFIRE","UNKNOWN","TRANSLUCENT_MAGENTA","GREEN"));
        Spec antiviral=result.get(result.size()-1);antiviral.actions.set(0,new Action("touchEffects2.FRUIT","killer,0,0"));
        for(String flag:new String[]{"_canmove","_canreact","_useframemovement","_useextraeffects"})antiviral.set("actor:victim:"+flag,true);
        antiviral.set("actor:victim:_updateEffects",7).set("actor:victim:_framesColor",8);
        int channel=0;
        for(Field field:fields(Organism.class))if(field.getName().startsWith("_seg")&&field.getName().endsWith("Reaction")){
            int[] values=new int[antiviral.genes.length];for(int i=0;i<values.length;i++)values[i]=channel*10+i;
            antiviral.set("actor:victim:"+field.getName(),values);channel++;
        }
        require(channel==36,"Unexpected reaction channel schema");
        result.add(scenario("fruit-antiviral-partial-before-jade","FRUIT","JADE").set("actor:killer:_antiviral",1).set("actor:killer:_jadefactor",2).set("actor:killer:_energy",0.0625d));
        result.add(scenario("fruit-antiviral-partial-before-infection","FRUIT","GREEN").set("actor:killer:_antiviral",1).set("actor:killer:_energy",0.0625d));
        result.add(scenario("fruit-dodge-before-antiviral","FRUIT","GREEN").set("actor:killer:_dodge",true).set("actor:killer:_antiviral",1));
        for(int version:new int[]{-1,0,1,2,3,4})result.add(scenario("fruit-target-spore-"+version,"FRUIT","SPORE").set("actor:killer:_sporeversion",version));
        result.add(scenario("fruit-infected-spore-one","FRUIT","SPORE").set("actor:killer:_sporeversion",1).set("actor:killer:_infectedGeneticCode","g3"));
        result.add(scenario("fruit-spore-consumer-keeps-positive","FRUIT","SPORE").set("actor:killer:_sporeversion",3).set("actor:killer:_isaconsumer",true));
        result.add(scenario("fruit-spore-negates-minimum","FRUIT","SPORE").set("actor:killer:_sporeversion",3).set("actor:killer:_reproduceEnergy",Integer.MIN_VALUE).set("actor:victim:_sporetime",0));
        for(int kills:new int[]{-1,0,1})result.add(scenario("fruit-silver-consumer-kills-"+kills,"FRUIT","SILVER").set("actor:killer:_isaconsumer",true).set("actor:killer:_nTotalKills",kills));
        result.add(scenario("fruit-silver-fungus-kill-block","FRUIT","SILVER").set("actor:killer:_isafungus",true).set("actor:killer:_nTotalKills",1));
        result.add(scenario("fruit-silver-enhanced-block","FRUIT","SILVER").set("actor:killer:_isenhanced",true));
        result.add(scenario("fruit-sky-version-two-shield","FRUIT","SKY").set("actor:killer:_skyversion",2));
        for(String peer:new String[]{"SKY","DEEPSKY"}){
            for(String variant:new String[]{"weak","enhanced","enhanced-blond","host-blond"}){
                Spec s=scenario("fruit-rigid-"+peer+"-"+variant,"FRUIT",peer).set("actor:killer:_skyversion",2).set("actor:killer:_energy",0.25d);
                if(!variant.equals("weak"))s.set("actor:victim:_isenhanced",true);
                if(variant.equals("enhanced-blond"))s.set("actor:victim:_isblond",true);
                if(variant.equals("host-blond"))s.set("actor:victim:_isblond",true).set("actor:killer:_isblond",true);
                result.add(s);
            }
        }
        for(int late:new int[]{0,1,-1})result.add(scenario("fruit-threshold-late-"+late,"FRUIT","GREEN").set("actor:killer:_reproducelate",late).set("actor:killer:_reproduceEnergy",123).set("actor:victim:_sporetime",7));
        for(int time:new int[]{Integer.MIN_VALUE,-1,0,1,Integer.MAX_VALUE})result.add(scenario("fruit-threshold-sporetime-"+time,"FRUIT","GREEN").set("actor:victim:_sporetime",time));
        for(String flag:new String[]{"_isinfectious","_isaconsumer","_isafungus"})result.add(scenario("fruit-plague-full-threshold-"+flag,"FRUIT","PLAGUE").set("actor:killer:"+flag,true));
        result.add(scenario("fruit-indigo-owned-infection-donates","FRUIT","INDIGO").set("actor:killer:_infectedGeneticCode","g0"));
        for(double energy:new double[]{-1d,-0d,0d,Math.nextDown(0.0000002d),0.0000002d,Math.nextUp(0.0000002d),Double.POSITIVE_INFINITY,Double.longBitsToDouble(0xfff8000000000042L)})result.add(scenario("fruit-donation-energy-"+bits(energy),"FRUIT","INDIGO").set("actor:victim:_energy",energy));
        for(boolean virus:new boolean[]{false,true}){
            double cost=virus?0.25d:0.125d;
            for(double oxygen:new double[]{0d,cost/2,cost,-0d,-1d,Double.POSITIVE_INFINITY,Double.longBitsToDouble(0xfff8000000000042L)})result.add(scenario("white-payment-oxygen-"+virus+"-"+bits(oxygen),"WHITE","GREEN").set("actor:victim:_isaplant",!virus).set("world:_O2",oxygen));
            for(double energy:new double[]{0d,cost/2,cost,-1d,Double.POSITIVE_INFINITY,Double.longBitsToDouble(0x7ff8000000000042L)})result.add(scenario("white-payment-energy-"+virus+"-"+bits(energy),"WHITE","GREEN").set("actor:victim:_isaplant",!virus).set("actor:victim:_energy",energy));
        }
        for(String weapon:new String[]{"WHITE","FRUIT"}){
            for(double energy:new double[]{Math.nextDown(Utils.tol),Utils.tol,Double.longBitsToDouble(0xfff8000000000042L),-1d})result.add(scenario("common-peer-death-"+weapon+"-"+bits(energy),weapon,"BROWN").set("actor:killer:_energy",energy));
            result.add(scenario("already-dead-peer-"+weapon,weapon,"BROWN").set("actor:killer:alive",false).set("actor:killer:_energy",0d));
            Spec retained=scenario("statistics-live-genome-"+weapon,weapon,"BROWN").set("actor:killer:_energy",0d);retained.add(action(retained).set("genome:g1:_cladeID","dead-retained"));result.add(retained);
        }
        Spec repeat=scenario("white-infect-once-reference-stability","WHITE","GREEN");repeat.add(action(repeat));result.add(repeat);
        require(result.stream().map(spec->spec.name).distinct().count()==result.size(),"Case names must be unique");
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.WhiteFruitOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("white_fruit.tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("WHITE_FRUIT_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
