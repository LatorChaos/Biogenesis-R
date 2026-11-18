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

/** Observe four original disabling touchEffects2 branches on real placed organisms */
public final class DisablingOracle {
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
        boolean passive,modifiedSky,clonePeer,sharedPeer,peerPassive,peerModifiedSky;
        int peerSpore=1,peerSymmetry=1;
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
            Spec peerSpec=new Spec("actor","GREEN","GREEN").colors(spec.peerGenes);peerSpec.passive=spec.peerPassive;peerSpec.modifiedSky=spec.peerModifiedSky;peerSpec.spore=spec.peerSpore;peerSpec.symmetry=spec.peerSymmetry;
            genomes.put("g1",genome(peerSpec));genomes.put("g2",genome(new Spec("peer","GREEN","GREEN").colors("GREEN")));
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
            String branch=action.operation.substring("touchEffects2.".length());require(List.of("VIOLET","SKY","OLIVE","GRAY").contains(branch),"Unsupported disabling branch");
            String[] arguments=action.argument.split(",");require(arguments.length==3&&arguments[0].equals("killer"),"Distinct peer and two indices required");
            int segment=Integer.parseInt(arguments[1]),peerSegment=Integer.parseInt(arguments[2]);
            Organism peer=actors.get(arguments[0]);require(peer!=victim,"Disabling oracle excludes self contacts");
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
            for(String name:new String[]{"_isaplant","_isaconsumer","_isafungus","_isauburn","_isenhanced","_isgray","_isblond","_isfrozen","_isinjured","_isinfectious","_transfersenergy","_dodge","_hasdodged","_candodge","_altruist","_isakiller","_issilver","_isonlyc4","_iscoral","_allfrozen"})values.put("actor:"+role+":"+name,false);
            for(String name:new String[]{"_plagueversion","_sporeversion","_fallowversion","_skyversion","_nTotalKills","_nTotalInfected","_indigo","_antiviral","_lavender","_jadefactor","_healing","_reproducelate"})values.put("actor:"+role+":"+name,0);
            values.put("actor:"+role+":_canmove",true);values.put("actor:"+role+":_canreact",true);
            values.put("actor:"+role+":_updateEffects",7);values.put("actor:"+role+":_useextraeffects",false);values.put("actor:"+role+":_useframemovement",true);
            values.put("actor:"+role+":_color",Color.MAGENTA);values.put("actor:"+role+":_framesColor",0);values.put("actor:"+role+":_photosynthesis",10d);
        }
        double[] actorPhoto=new double[segments],peerPhoto=new double[peerSegments];for(int i=0;i<segments;i++)actorPhoto[i]=i+0.25;for(int i=0;i<peerSegments;i++)peerPhoto[i]=i+0.25;
        values.put("actor:victim:_mphoto",actorPhoto);values.put("actor:killer:_mphoto",peerPhoto);return values;
    }
    private static Action action(Spec spec){return new Action("touchEffects2."+spec.weapon,"killer,"+(spec.genes.length*spec.symmetry-1)+",0");}
    private static Spec base(String name,String weapon,String peer){
        require(List.of("VIOLET","SKY","OLIVE","GRAY").contains(weapon),"Only four disabling branches are allowed");
        return new Spec(name,weapon,peer).setting("VIOLET_ENERGY_CONSUMPTION",0.125d).setting("SKY_ENERGY_CONSUMPTION",0.25d)
            .setting("OLIVE_ENERGY_CONSUMPTION",0.375d).setting("GRAY_ENERGY_CONSUMPTION",0.5d)
            .setting("BLUE_ENERGY_CONSUMPTION",0.75d).setting("DODGE_ENERGY_CONSUMPTION",1d)
            .setting("MAGENTA_ENERGY_CONSUMPTION",1.25d).setting("FALLOW_ENERGY_CONSUMPTION",1.5d).setting("MINT_ENERGY_CONSUMPTION",1.75d)
            .setting("ORGANIC_OBTAINED_ENERGY",7d)
            .set("actor:victim:_isaplant",true).set("actor:killer:_isaplant",true)
            .set("actor:victim:_skyversion",weapon.equals("SKY")?2:0)
            .set("actor:killer:_infectedGeneticCode","g3").set("actor:killer:_savedGeneticCode","g4");
    }
    private static Spec scenario(String name,String weapon,String peer){Spec spec=base(name,weapon,peer);return spec.add(action(spec));}
    private static Spec peerBody(Spec spec,String...names){spec.peerGenes=names;return spec;}
    private static Spec peerPhenotype(Spec spec,String...names)throws Exception{return spec.set("actor:killer:_segColor",phenotype(names));}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(String peer:COLORS)result.add(scenario("palette-VIOLET-"+peer,"VIOLET",peer));
        for(String peer:INTERNAL)result.add(scenario("palette-VIOLET-"+peer,"VIOLET",peer));
        for(String peer:new String[]{"UNKNOWN","TRANSLUCENT_MAGENTA"})result.add(scenario("palette-VIOLET-"+peer,"VIOLET",peer));
        for(String peer:new String[]{"SKY","DEEPSKY","GREEN","BLUE","JADE","DARKJADE","MINT","LAVENDER","ROSE","MAGENTA","MAROON","BROKEN","CORAL","CREAM","FRUIT","DARKFIRE","BARK","FALLOW","SPIKE","OLIVE","SPIKEPOINT","BROWN","UNKNOWN","TRANSLUCENT_MAGENTA"})result.add(scenario("group-SKY-"+peer,"SKY",peer));
        for(String peer:new String[]{"OLIVE","DARKOLIVE","BLUE","OCHRE","SKY","DEEPSKY","LILAC","DARKLILAC","SPIKE","SPIKEPOINT","FALLOW","BARK","OLDBARK","UNKNOWN","TRANSLUCENT_MAGENTA"})result.add(scenario("group-OLIVE-"+peer,"OLIVE",peer));
        for(String peer:new String[]{"BROWN","SPIKEPOINT","FALLOW","PINK","OLDBARK","CORAL","MAROON","GREEN","JADE","DARKJADE","WHITE","BLUE","SKY","DEEPSKY","OCHRE","MINT","LAVENDER","ROSE","MAGENTA","RED","SILVER","SPIKE","OLIVE","DARKOLIVE","VIOLET","BARK","UNKNOWN","TRANSLUCENT_MAGENTA"})result.add(scenario("group-GRAY-"+peer,"GRAY",peer));
        for(String weapon:new String[]{"VIOLET","SKY","GRAY"}){
            for(boolean only:new boolean[]{false,true})for(boolean enhanced:new boolean[]{false,true})for(int frame:new int[]{8,9})result.add(scenario("dodged-prefix-"+weapon+"-"+only+"-"+enhanced+"-"+frame,weapon,"BROWN").set("actor:killer:_hasdodged",false).set("actor:killer:_isonlyc4",only).set("actor:killer:_isenhanced",enhanced).set("actor:killer:_framesColor",frame));
            result.add(scenario("dodged-already-"+weapon,weapon,"BROWN").set("actor:killer:_hasdodged",true));
        }
        for(String weapon:new String[]{"VIOLET","GRAY"})for(String peer:new String[]{"GREEN","JADE","MAGENTA"}){
            result.add(scenario("dodge-paid-"+weapon+"-"+peer,weapon,peer).set("actor:killer:_dodge",true));
            result.add(scenario("dodge-partial-"+weapon+"-"+peer,weapon,peer).set("actor:killer:_dodge",true).set("actor:killer:_energy",0.5d));
        }
        for(String peer:new String[]{"DARKFIRE","WHITE","VIOLET","CYAN","TEAL","DARKGRAY","PINK","UNKNOWN"})result.add(scenario("violet-nonplant-"+peer,"VIOLET",peer).set("actor:killer:_isaplant",false));
        result.add(scenario("violet-darkfire-sentinel","VIOLET","DARKFIRE").set("actor:killer:_mphoto",new double[]{-1d}));
        result.add(scenario("violet-darkfire-inactive","VIOLET","DARKFIRE").set("actor:killer:active",false));
        for(String role:new String[]{"victim","killer"})result.add(scenario("violet-coral-gate-"+role,"VIOLET","WHITE").set("actor:victim:_isaplant",false).set("actor:killer:_isaplant",false).set("actor:"+role+":_iscoral",true));
        for(String flag:new String[]{"_isaconsumer","_isafungus","_isauburn"})result.add(scenario("violet-white-eligible-"+flag,"VIOLET","WHITE").set("actor:victim:_isaplant",false).set("actor:victim:_isaconsumer",true).set("actor:killer:_isaplant",false).set("actor:killer:"+flag,true));
        result.add(scenario("violet-white-ineligible","VIOLET","WHITE").set("actor:victim:_isaplant",false).set("actor:victim:_isaconsumer",true).set("actor:killer:_isaplant",false));
        result.add(scenario("violet-white-plague","VIOLET","WHITE").set("actor:victim:_isaconsumer",true).set("actor:killer:_isaplant",false).set("actor:killer:_plagueversion",1));
        for(String peer:new String[]{"GREEN","SPRING","WINTER","JADE","DARKJADE"}){
            result.add(scenario("violet-plant-actor-ineligible-"+peer,"VIOLET",peer).set("actor:victim:_isaplant",false));
            result.add(scenario("violet-plant-actor-fungus-"+peer,"VIOLET",peer).set("actor:victim:_isaplant",false).set("actor:victim:_isafungus",true));
        }
        for(String peer:new String[]{"GREEN","SPRING","WINTER"}){
            Spec inactive=scenario("violet-inactive-all-plants-"+peer,"VIOLET",peer);peerBody(inactive,peer,"JADE","DARKGREEN","CYAN","BLUE","EYE");result.add(peerPhenotype(inactive,peer,"JADE","DARKGREEN","CYAN","BLUE","EYE").set("actor:killer:active",false));
        }
        for(String original:new String[]{"SPRING","JADE"}){
            Spec check=scenario("violet-checkplant-original-"+original,"VIOLET","DARKGREEN");peerBody(check,original,"DARKGREEN","JADE","SPRING");result.add(peerPhenotype(check,"DARKGREEN","DARKGREEN","JADE","SPRING").set("actor:killer:_jadefactor",7).set("actor:killer:_spin",Integer.MAX_VALUE));
        }
        Spec overflow=scenario("violet-checkplant-spin-wrap","VIOLET","SPRING");peerBody(overflow,"SPRING","SPRING","SPRING");result.add(peerPhenotype(overflow,"SPRING","SPRING","SPRING").set("genome:g1:gene1._length",214748364.7d).set("genome:g1:gene2._length",214748364.7d));
        Spec passive=scenario("violet-teal-passive","VIOLET","TEAL");passive.peerPassive=true;result.add(passive);
        for(String peer:new String[]{"JADE","DARKJADE"})result.add(scenario("violet-jade-free-"+peer,"VIOLET",peer).set("actor:victim:_energy",0d));
        for(String peer:new String[]{"AUBURN","YELLOW","BLOND","GOLD","SPORE","MINT","LAVENDER","MAGENTA","SKY"}){
            Spec restore=scenario("violet-restore-mixed-"+peer,"VIOLET",peer);peerBody(restore,peer,"EYE","ROSE","TEAL","BLOND","SPRING");result.add(peerPhenotype(restore,peer,"EYE","ROSE","TEAL","BLOND","SPRING").set("actor:killer:_isaplant",false).set("actor:killer:_gold",7).set("actor:killer:_skyversion",2).setting("SKY_ENERGY_CONSUMPTION",2000d));
        }
        for(String peer:new String[]{"ROSE","MAGENTA","MINT","LAVENDER"})result.add(scenario("violet-altruist-"+peer,"VIOLET",peer).set("actor:victim:_altruist",true));
        for(String peer:new String[]{"MAGENTA","CORAL","MAROON"}){
            result.add(scenario("violet-heal-paid-"+peer,"VIOLET",peer).set("actor:victim:_isaconsumer",true).set("actor:killer:_healing",2).set("actor:killer:_isenhanced",true));
            result.add(scenario("violet-heal-partial-"+peer,"VIOLET",peer).set("actor:victim:_isaconsumer",true).set("actor:killer:_healing",2).set("actor:killer:_isenhanced",true).set("actor:killer:_energy",0.5d));
        }
        for(String peer:new String[]{"OLIVE","DARKOLIVE"}){
            result.add(scenario("violet-olive-healed-"+peer,"VIOLET",peer).set("actor:killer:_healing",1));
            result.add(scenario("violet-olive-peer-empty-"+peer,"VIOLET",peer).set("actor:killer:_energy",0d).set("actor:killer:_isaplant",false));
            result.add(scenario("violet-olive-actor-empty-"+peer,"VIOLET",peer).set("actor:victim:_energy",0d).set("actor:victim:_isaplant",false));
        }
        result.add(scenario("violet-silver-experience-block","VIOLET","SILVER").set("actor:victim:_isaconsumer",true).set("actor:killer:_isaconsumer",true).set("actor:killer:_nTotalKills",1));
        result.add(scenario("violet-silver-experience-tie","VIOLET","SILVER").set("actor:victim:_isaconsumer",true).set("actor:killer:_isaconsumer",true));
        for(String peer:new String[]{"FIRE","CREAM","FRUIT","PLAGUE","LILAC","FALLOW"}){
            result.add(scenario("violet-consumer-"+peer,"VIOLET",peer).set("actor:victim:_isaconsumer",true));
            result.add(scenario("violet-fungus-"+peer,"VIOLET",peer).set("actor:victim:_isafungus",true));
        }
        result.add(scenario("violet-fire-enhanced-peer","VIOLET","FIRE").set("actor:victim:_isaconsumer",true).set("actor:killer:_isenhanced",true));
        result.add(scenario("violet-cream-infectious","VIOLET","CREAM").set("actor:victim:_isinfectious",true));
        result.add(scenario("violet-plague-infectious","VIOLET","PLAGUE").set("actor:victim:_isaconsumer",true).set("actor:killer:_isinfectious",true));
        result.add(scenario("violet-lilac-enhanced","VIOLET","LILAC").set("actor:victim:_isaconsumer",true).set("actor:victim:_isenhanced",true));
        result.add(scenario("violet-fallow-infectious-block","VIOLET","FALLOW").set("actor:victim:_isinfectious",true));
        result.add(scenario("violet-coral-gold","VIOLET","CORAL").set("actor:victim:_isaplant",false).set("actor:killer:_isaplant",false).set("actor:killer:_gold",1));
        result.add(scenario("violet-spike-ineligible","VIOLET","SPIKE").set("actor:killer:_isaplant",false));
        result.add(scenario("violet-spike-enhanced","VIOLET","SPIKE").set("actor:killer:_isaplant",false).set("actor:killer:_isenhanced",true));
        for(String weapon:new String[]{"VIOLET","SKY","GRAY"}){
            result.add(scenario("blue-retaliation-"+weapon,weapon,"BLUE").set("actor:killer:_isenhanced",true));
            result.add(scenario("blue-retaliation-actor-death-"+weapon,weapon,"BLUE").set("actor:killer:_isenhanced",true).set("actor:victim:_energy",0.5d));
            result.add(scenario("blue-shield-empty-"+weapon,weapon,"BLUE").set("actor:killer:_energy",0d));
            result.add(scenario("blue-shield-partial-"+weapon,weapon,"BLUE").set("actor:killer:_energy",0.5d));
        }
        for(int own:new int[]{1,2}){
            for(String peer:new String[]{"SKY","DEEPSKY","GREEN","BARK","FALLOW","CORAL","FRUIT","UNKNOWN"})result.add(scenario("sky-mode-"+own+"-"+peer,"SKY",peer).set("actor:victim:_skyversion",own).set("actor:killer:_skyversion",2));
            for(boolean move:new boolean[]{false,true})result.add(scenario("sky-same-mode-move-"+own+"-"+move,"SKY","GREEN").set("actor:victim:_skyversion",own).set("actor:killer:_skyversion",own).set("actor:killer:_canmove",move).set("actor:victim:_isaplant",false).set("actor:killer:_isaplant",false));
        }
        for(String peer:new String[]{"SKY","DEEPSKY"}){
            result.add(scenario("sky-shield-twelve-paid-"+peer,"SKY",peer).set("actor:killer:_skyversion",1));
            result.add(scenario("sky-shield-twelve-failed-"+peer,"SKY",peer).set("actor:killer:_skyversion",1).set("actor:killer:_energy",2d));
            result.add(scenario("sky-shield-actor-failed-"+peer,"SKY",peer).set("actor:killer:_skyversion",2).set("actor:victim:_energy",0d));
            result.add(scenario("sky-already-frozen-"+peer,"SKY",peer).set("actor:killer:_skyversion",2).set("actor:killer:_allfrozen",true).set("actor:killer:_energy",0d));
        }
        for(int frame:new int[]{0,1,8,9}){
            result.add(scenario("sky-dodge-frame-"+frame,"SKY","GREEN").set("actor:killer:_dodge",true).set("actor:killer:_hasdodged",true).set("actor:killer:_framesColor",frame));
            result.add(scenario("sky-clear-dodge-prefix-"+frame,"SKY","GREEN").set("actor:victim:_skyversion",1).set("actor:killer:_dodge",true).set("actor:killer:_canreact",false).set("actor:killer:_framesColor",frame));
        }
        Spec blueInactive=scenario("sky-blue-inactive-modifier10","SKY","BLUE");blueInactive.peerSpore=10;peerBody(blueInactive,"BLUE","BLUE","EYE");result.add(peerPhenotype(blueInactive,"BLUE","BLUE","EYE").set("actor:killer:active",false));
        Spec blueLow=scenario("sky-blue-inactive-modifier9","SKY","BLUE");blueLow.peerSpore=9;result.add(blueLow.set("actor:killer:active",false));
        result.add(scenario("sky-blue-inactive-plant-block","SKY","BLUE").set("actor:killer:active",false).set("actor:victim:_skyversion",1));
        for(String peer:new String[]{"JADE","DARKJADE"}){
            result.add(scenario("sky-jade-enhanced-"+peer,"SKY",peer).set("actor:victim:_isenhanced",true));
            result.add(scenario("sky-jade-dodge-"+peer,"SKY",peer).set("actor:killer:_dodge",true).set("actor:killer:_hasdodged",true));
        }
        result.add(scenario("sky-darkjade-healing-block","SKY","DARKJADE").set("actor:killer:_healing",1));
        result.add(scenario("sky-darkjade-inactive-healing","SKY","DARKJADE").set("actor:killer:_healing",1).set("actor:killer:active",false));
        for(String peer:new String[]{"MINT","MAGENTA","MAROON","CORAL","CREAM","FRUIT","DARKFIRE","UNKNOWN"}){
            result.add(scenario("sky-healing-plant-block-"+peer,"SKY",peer).set("actor:victim:_skyversion",1).set("actor:killer:_healing",1));
            result.add(scenario("sky-healing-enhanced-"+peer,"SKY",peer).set("actor:victim:_skyversion",1).set("actor:victim:_isenhanced",true).set("actor:killer:_healing",1).set("actor:victim:_isaconsumer",true));
        }
        for(String peer:new String[]{"MINT","LAVENDER","ROSE","MAGENTA"})result.add(scenario("sky-altruist-"+peer,"SKY",peer).set("actor:victim:_altruist",true));
        result.add(scenario("sky-magenta-payment-fails","SKY","MAGENTA").set("actor:killer:_energy",0.5d));
        for(String peer:new String[]{"CREAM","FRUIT","DARKFIRE"})for(int version:new int[]{1,5,6}){
            Spec inactive=scenario("sky-inactive-spore-"+peer+"-"+version,"SKY",peer);peerBody(inactive,peer,"FRUIT","DARKFIRE","BLUE","EYE");result.add(peerPhenotype(inactive,peer,"FRUIT","DARKFIRE","BLUE","EYE").set("actor:killer:active",false).set("actor:killer:_sporeversion",version).set("actor:victim:_skyversion",1));
        }
        result.add(scenario("sky-inactive-already-frozen","SKY","FRUIT").set("actor:killer:active",false).set("actor:killer:_sporeversion",1).set("actor:killer:_isfrozen",true));
        result.add(scenario("sky-fallow-infectious","SKY","FALLOW").set("actor:victim:_skyversion",1).set("actor:killer:_isaplant",false).set("actor:killer:_isinfectious",true));
        result.add(scenario("sky-fallow-ineligible","SKY","FALLOW").set("actor:victim:_skyversion",1).set("actor:killer:_isaplant",false));
        result.add(scenario("sky-spike-ineligible","SKY","SPIKE").set("actor:killer:_isaplant",false));
        result.add(scenario("sky-spike-enhanced","SKY","SPIKE").set("actor:killer:_isaplant",false).set("actor:killer:_isenhanced",true));
        for(String peer:new String[]{"OLIVE","DARKOLIVE"}){
            result.add(scenario("olive-nonplant-actor-"+peer,"OLIVE",peer).set("actor:victim:_isaplant",false));
            result.add(scenario("olive-infectious-enhanced-"+peer,"OLIVE",peer).set("actor:victim:_isenhanced",true).set("actor:killer:_isinfectious",true));
            result.add(scenario("olive-both-infectious-"+peer,"OLIVE",peer).set("actor:victim:_isinfectious",true).set("actor:killer:_isinfectious",true));
            result.add(scenario("olive-own-infectious-block-"+peer,"OLIVE",peer).set("actor:victim:_isinfectious",true));
            result.add(scenario("olive-peer-nonplant-"+peer,"OLIVE",peer).set("actor:killer:_isaplant",false));
            result.add(scenario("olive-peer-empty-"+peer,"OLIVE",peer).set("actor:killer:_energy",0d));
        }
        result.add(scenario("olive-blue-shield","OLIVE","BLUE").set("actor:killer:_isaplant",false));
        result.add(scenario("olive-blue-infectious-shield","OLIVE","BLUE").set("actor:victim:_isinfectious",true));
        result.add(scenario("olive-blue-enhanced-bypass","OLIVE","BLUE").set("actor:victim:_isenhanced",true).set("actor:killer:_isaplant",false));
        for(int modifier:new int[]{9,10}){
            Spec inactive=scenario("olive-blue-inactive-modifier"+modifier,"OLIVE","BLUE");inactive.peerSpore=modifier;peerBody(inactive,"BLUE","BLUE","EYE");result.add(peerPhenotype(inactive,"BLUE","BLUE","EYE").set("actor:killer:active",false).set("actor:victim:_isenhanced",true));
        }
        result.add(scenario("olive-blue-inactive-ineligible","OLIVE","BLUE").set("actor:killer:active",false));
        for(String flag:new String[]{"_iscoral","_isenhanced"})result.add(scenario("olive-blue-inactive-"+flag,"OLIVE","BLUE").set("actor:killer:active",false).set("actor:victim:"+flag,true));
        result.add(scenario("olive-ochre-ineligible","OLIVE","OCHRE").set("actor:killer:_isaplant",false));
        result.add(scenario("olive-ochre-enhanced","OLIVE","OCHRE").set("actor:killer:_isaplant",false).set("actor:victim:_isenhanced",true));
        for(String peer:new String[]{"SKY","DEEPSKY"}){
            Spec modified=scenario("olive-sky-modified-"+peer,"OLIVE",peer);modified.peerModifiedSky=true;result.add(modified.set("actor:killer:_isaplant",false));
            result.add(scenario("olive-sky-ordinary-"+peer,"OLIVE",peer).set("actor:killer:_isaplant",false));
        }
        result.add(scenario("olive-lilac-nonplant","OLIVE","LILAC").set("actor:killer:_isaplant",false));
        for(String flag:new String[]{"_isaconsumer","_isafungus","_isinfectious"})result.add(scenario("olive-fallow-ineligible-"+flag,"OLIVE","FALLOW").set("actor:victim:"+flag,true));
        result.add(scenario("olive-fallow-enhanced","OLIVE","FALLOW").set("actor:victim:_isaconsumer",true).set("actor:victim:_isenhanced",true));
        result.add(scenario("olive-fallow-nonplant-peer","OLIVE","FALLOW").set("actor:killer:_isaplant",false));
        for(String peer:new String[]{"BARK","OLDBARK"}){
            result.add(scenario("olive-bark-dead-"+peer,"OLIVE",peer).set("actor:victim:_isaplant",false));
            result.add(scenario("olive-bark-inactive-"+peer,"OLIVE",peer).set("actor:killer:active",false));
            result.add(scenario("olive-bark-consumer-"+peer,"OLIVE",peer).set("actor:killer:_isaconsumer",true));
        }
        for(String peer:new String[]{"FALLOW","PINK","OLDBARK","CORAL"}){
            result.add(scenario("gray-enhanced-"+peer,"GRAY",peer).set("actor:victim:_isenhanced",true));
            result.add(scenario("gray-enhanced-heal-paid-"+peer,"GRAY",peer).set("actor:victim:_isenhanced",true).set("actor:killer:_healing",1));
            result.add(scenario("gray-enhanced-heal-failed-"+peer,"GRAY",peer).set("actor:victim:_isenhanced",true).set("actor:killer:_healing",1).set("actor:killer:_energy",0.5d));
        }
        for(String peer:new String[]{"GREEN","MAROON","WHITE","OCHRE","MAGENTA","RED","SILVER","SPIKE","VIOLET","UNKNOWN"}){
            result.add(scenario("gray-survive-"+peer,"GRAY",peer).set("actor:killer:_healing",1));
            result.add(scenario("gray-inactive-"+peer,"GRAY",peer).set("actor:killer:_healing",1).set("actor:killer:active",false));
        }
        for(String peer:new String[]{"GREEN","JADE","DARKJADE"}){
            result.add(scenario("gray-gray-consumer-"+peer,"GRAY",peer).set("actor:killer:_healing",1).set("actor:killer:_isgray",true).set("actor:killer:_isaconsumer",true));
        }
        for(String peer:new String[]{"JADE","DARKJADE"}){
            result.add(scenario("gray-jade-healing-"+peer,"GRAY",peer).set("actor:killer:_healing",1));
            result.add(scenario("gray-jade-enhanced-"+peer,"GRAY",peer).set("actor:killer:_healing",1).set("actor:victim:_isenhanced",true));
        }
        result.add(scenario("gray-white-virus-heal-paid","GRAY","WHITE").set("actor:killer:_isaplant",false).set("actor:killer:_healing",1));
        result.add(scenario("gray-white-virus-heal-failed","GRAY","WHITE").set("actor:killer:_isaplant",false).set("actor:killer:_healing",1).set("actor:killer:_energy",0.5d));
        result.add(scenario("gray-white-enhanced-source","GRAY","WHITE").set("actor:killer:_isaplant",false).set("actor:killer:_healing",1).set("actor:victim:_isenhanced",true));
        result.add(scenario("gray-red-enhanced-immunity","GRAY","RED").set("actor:killer:_isenhanced",true));
        result.add(scenario("gray-silver-experience-block","GRAY","SILVER").set("actor:killer:_isaconsumer",true).set("actor:killer:_nTotalKills",1));
        result.add(scenario("gray-silver-experience-tie","GRAY","SILVER").set("actor:killer:_isaconsumer",true));
        result.add(scenario("gray-spike-nonplant-ineligible","GRAY","SPIKE").set("actor:killer:_isaplant",false));
        result.add(scenario("gray-spike-enhanced","GRAY","SPIKE").set("actor:killer:_isaplant",false).set("actor:killer:_isenhanced",true));
        for(String peer:new String[]{"MINT","LAVENDER","ROSE","MAGENTA"})result.add(scenario("gray-altruist-"+peer,"GRAY",peer).set("actor:victim:_altruist",true));
        result.add(scenario("gray-magenta-gray-bypass","GRAY","MAGENTA").set("actor:killer:_isgray",true));
        result.add(scenario("gray-maroon-defense","GRAY","MAROON").set("actor:victim:_isaconsumer",true).set("actor:killer:_healing",1));
        for(String peer:new String[]{"SKY","DEEPSKY"}){
            result.add(scenario("gray-sky-immune-"+peer,"GRAY",peer).set("actor:killer:_isaplant",false).set("actor:killer:_skyversion",1));
            result.add(scenario("gray-sky-failed-"+peer,"GRAY",peer).set("actor:killer:_energy",0d).set("actor:killer:_healing",1));
            result.add(scenario("gray-sky-consumer-"+peer,"GRAY",peer).set("actor:victim:_isaconsumer",true));
        }
        for(String peer:new String[]{"OLIVE","DARKOLIVE"}){
            result.add(scenario("gray-olive-healing-"+peer,"GRAY",peer).set("actor:killer:_healing",1));
            result.add(scenario("gray-olive-failed-"+peer,"GRAY",peer).set("actor:killer:_energy",0d));
            result.add(scenario("gray-olive-actor-empty-"+peer,"GRAY",peer).set("actor:victim:_energy",0d));
        }
        Spec survive=scenario("gray-survive-payment-context","GRAY","GREEN");peerBody(survive,"GREEN","SKY","FALLOW","EYE","MINT");result.add(peerPhenotype(survive,"GREEN","SKY","FALLOW","EYE","MINT").set("actor:killer:_healing",1).set("world:_O2",0.625d));
        result.add(scenario("gray-kill-counter-wrap","GRAY","GREEN").set("actor:victim:_nTotalKills",Integer.MAX_VALUE));
        for(String weapon:new String[]{"VIOLET","SKY","OLIVE","GRAY"}){
            double cost=weapon.equals("VIOLET")?0.125d:weapon.equals("SKY")?0.25d:weapon.equals("OLIVE")?0.375d:0.5d;
            String target=weapon.equals("OLIVE")?"LILAC":"GREEN";
            for(double energy:new double[]{0d,cost/2,cost,-0d,-1d,Double.POSITIVE_INFINITY,Double.longBitsToDouble(0x7ff8000000000042L)})result.add(scenario("payment-energy-"+weapon+"-"+bits(energy),weapon,target).set("actor:victim:_energy",energy));
            for(double oxygen:new double[]{0d,cost/2,cost,-0d,-1d,Double.POSITIVE_INFINITY,Double.longBitsToDouble(0xfff8000000000042L)})result.add(scenario("payment-oxygen-"+weapon+"-"+bits(oxygen),weapon,target).set("world:_O2",oxygen));
            result.add(scenario("payment-zero-cost-"+weapon,weapon,target).setting(weapon+"_ENERGY_CONSUMPTION",0d).set("world:_O2",0d));
            for(double energy:new double[]{Math.nextDown(Utils.tol),Utils.tol,-1d,Double.longBitsToDouble(0xfff8000000000042L)})result.add(scenario("common-peer-death-"+weapon+"-"+bits(energy),weapon,"BROWN").set("actor:killer:_energy",energy));
            result.add(scenario("common-peer-already-dead-"+weapon,weapon,"BROWN").set("actor:killer:alive",false).set("actor:killer:_energy",0d));
            Spec alias=scenario("statistics-live-genome-"+weapon,weapon,"BROWN").set("actor:killer:_energy",0d);alias.add(action(alias).set("genome:g1:_cladeID","retained-dead"));result.add(alias);
        }
        for(double oxygen:new double[]{0d,0.125d,0.375d,0.625d}){
            Spec mixed=scenario("sky-frozen-mixed-partial-"+bits(oxygen),"SKY","GREEN");peerBody(mixed,"GREEN","CYAN","TEAL","JADE","SKY","FALLOW","EYE","MAGENTA");result.add(peerPhenotype(mixed,"GREEN","CYAN","TEAL","JADE","SKY","FALLOW","EYE","MAGENTA").set("world:_O2",oxygen));
        }
        Spec modifiedViolet=scenario("violet-sky-modified-nonplant","VIOLET","SKY");modifiedViolet.peerModifiedSky=true;result.add(modifiedViolet.set("actor:killer:_isaplant",false).set("actor:killer:_skyversion",2).setting("SKY_ENERGY_CONSUMPTION",2000d));
        Spec survivor=scenario("gray-violet-survive-then-darkfire","GRAY","VIOLET");peerBody(survivor,"VIOLET","MAGENTA","EYE");result.add(peerPhenotype(survivor,"VIOLET","MAGENTA","EYE").set("actor:killer:_healing",1));
        Spec repeatGray=scenario("gray-repeated-contact-after-death","GRAY","GREEN");repeatGray.add(action(repeatGray));result.add(repeatGray);
        Spec repeatJade=scenario("violet-repeated-jade-degradation","VIOLET","JADE");repeatJade.add(action(repeatJade)).add(action(repeatJade));result.add(repeatJade);
        Spec repeatSky=scenario("sky-repeated-frozen-guard","SKY","GREEN");repeatSky.add(action(repeatSky));result.add(repeatSky);
        require(result.stream().map(spec->spec.name).distinct().count()==result.size(),"Case names must be unique");
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.DisablingOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("disabling.tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("DISABLING_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
