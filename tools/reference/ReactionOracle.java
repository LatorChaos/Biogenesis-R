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

/** Observe original private reaction and brake methods on distinct real placed organisms */
public final class ReactionOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static final String[] REACTIONS=("red green blue plague scourge white gray silver default consumer plant magenta pink coral orange bark violet virus maroon olive mint cream spike fallow lightblue ochre sky lilac fire lightbrown greenbrown brown ice broken sick friend").split(" ");
    private static int cases,snapshots,calls;

    private static final class Action{
        final String operation,argument;
        final Map<String,Object> controls=new LinkedHashMap<>();
        Action(String operation,String argument){this.operation=operation;this.argument=argument;}
        Action set(String target,Object value){controls.put(target,value);return this;}
    }
    private static final class Spec{
        final String name;
        String[] genes={"TEAL","GREEN"};
        double length=12;
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
            genomes.put("g1",genome(new Spec("actor").colors("GREEN")));genomes.put("g2",genome(new Spec("peer").colors("GREEN")));
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
            String[] arguments=action.argument.split(",");require(arguments.length==3&&arguments[0].equals("killer"),"Distinct peer and two indices required");
            require(List.of("touchReaction","touchfriendReaction","touchBrake","touchfriendBrake").contains(action.operation),"Unknown actual operation");
            Organism peer=actors.get(arguments[0]);require(peer!=victim,"Reaction oracle excludes self contacts");
            Method method=Organism.class.getDeclaredMethod(action.operation,Organism.class,int.class,int.class);method.setAccessible(true);
            method.invoke(victim,peer,Integer.parseInt(arguments[1]),Integer.parseInt(arguments[2]));
        }
        String finish(){row("E");cases++;return rows.toString();}
    }
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static Random copyRandom(Random random)throws Exception{ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception{
        switch(name){case "UNKNOWN":return new Color(0x12345678,true);case "TRANSLUCENT_MAGENTA":return new Color(0x12ff00ff,true);case "TRANSLUCENT_EYE":return new Color(0x12004040,true);case "TRANSLUCENT_TEAL":return new Color(0x12008080,true);case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static Color[] phenotype(String...names)throws Exception{Color[] values=new Color[names.length];for(int i=0;i<names.length;i++)values[i]=color(names[i]);return values;}
    private static GeneticCode genome(Spec spec)throws Exception{
        List<Gene> genes=new ArrayList<>();for(int i=0;i<spec.genes.length;i++){
            Gene gene=new Gene();gene.setLength(spec.lengths==null?spec.length:spec.lengths[i]);gene.setTheta(i*0.25);gene.setBranch(i==0?0:-1);gene.setColor(color(spec.genes[i]));genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,spec.symmetry,spec.mirror,0,0,1,1,spec.spore,1,false,false,false,false,false,false,false,false,spec.passive,false,false,false,false,spec.modifiedSky,false);code._generation=41;code._cladeID="root";return code;
    }
    private static Map<String,Object> prepared(int segments){
        Map<String,Object> values=new LinkedHashMap<>();
        values.put("actor:victim:_energy",1000d);values.put("actor:killer:_energy",1000d);
        values.put("actor:victim:_candodge",false);
        for(String name:new String[]{"_isaplant","_isaconsumer","_isafungus","_isauburn","_isenhanced"})values.put("actor:killer:"+name,false);
        values.put("actor:killer:_plagueversion",0);values.put("actor:killer:_sporeversion",0);values.put("actor:killer:_nTotalKills",0);
        int[] x1=new int[segments],y1=new int[segments],x2=new int[segments],y2=new int[segments];
        for(int i=0;i<segments;i++){x1[i]=1+4*i;y1[i]=2+5*i;x2[i]=3+8*i;y2[i]=4+13*i;}
        values.put("actor:victim:x1",x1);values.put("actor:victim:y1",y1);values.put("actor:victim:x2",x2);values.put("actor:victim:y2",y2);
        for(String name:REACTIONS)values.put("actor:victim:_seg"+name+"Reaction",new int[segments]);
        return values;
    }
    private static Action action(Spec spec,String operation){
        return new Action(operation,"killer,"+(spec.genes.length*spec.symmetry-1)+",0")
            .set("rng:seed",0L).set("actor:victim:dx",0.75d).set("actor:victim:dy",-0.5d).set("actor:victim:dtheta",0.03125d);
    }
    private static Action vector(Spec spec,Action action,int[] values){
        require(values.length==36,"Reaction vector must have 36 channels");
        for(int channel=0;channel<REACTIONS.length;channel++){
            int[] current=new int[spec.genes.length*spec.symmetry];Arrays.fill(current,values[channel]);
            action.set("actor:victim:_seg"+REACTIONS[channel]+"Reaction",current);
        }
        return action;
    }
    private static int[] constant(int value){int[] values=new int[36];Arrays.fill(values,value);return values;}
    private static Spec base(String name,String peer)throws Exception{
        return new Spec(name).set("actor:killer:_segColor",phenotype(peer)).setting("TEAL_ENERGY_CONSUMPTION",0.25d).setting("MAX_VEL",1000000d).setting("MAX_ROT",1000000d);
    }
    private static Spec coded(Spec spec){
        for(int digit=0;digit<2;digit++){
            int[] values=new int[36];for(int i=0;i<36;i++)values[i]=digit==0?i%6:i/6;
            spec.add(vector(spec,action(spec,"touchReaction"),values));
        }
        spec.add(vector(spec,action(spec,"touchBrake"),constant(1)));return spec;
    }
    private static Spec pair(Spec spec,int mode){
        spec.add(vector(spec,action(spec,"touchReaction"),constant(mode)));
        spec.add(vector(spec,action(spec,"touchBrake"),constant(mode)));return spec;
    }
    private static Spec one(String channel,String peer)throws Exception{
        Spec spec=base("one-channel-"+channel,peer);int index=Arrays.asList(REACTIONS).indexOf(channel);require(index>=0,"Unknown supplied channel");
        int[] values=new int[36];values[index]=1;
        String prefix=channel.equals("friend")?"touchfriend":"touch";
        spec.add(vector(spec,action(spec,prefix+"Reaction"),values));spec.add(vector(spec,action(spec,prefix+"Brake"),values));return spec;
    }
    private static Spec payment(String name,double oxygen,double energy,int mode)throws Exception{
        return pair(base(name,"GREEN").setting("TEAL_ENERGY_CONSUMPTION",1d).set("world:_O2",oxygen).set("actor:victim:_energy",energy),mode);
    }
    private static long seedForInt(int value){
        for(long seed=0;seed<10000;seed++)if(new Random(seed).nextInt(10)==value)return seed;
        throw new IllegalStateException("Bounded seed search exhausted");
    }
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(String name:COLORS)result.add(coded(base("peer-palette-"+name,name)));
        for(String name:INTERNAL)result.add(coded(base("peer-palette-"+name,name)));
        for(String name:new String[]{"UNKNOWN","TRANSLUCENT_MAGENTA"})result.add(coded(base("peer-palette-"+name,name)));
        String[] peers=("RED GREEN BLUE WHITE WHITE WHITE GRAY SILVER GOLD GOLD GOLD MAGENTA PINK CORAL ORANGE BARK VIOLET WHITE MAROON OLIVE MINT CREAM SPIKEPOINT FALLOW LIGHT_BLUE OCHRE SKY LILAC FIRE LIGHTBROWN GREENBROWN BROWN ICE BROKEN ORANGE GREEN").split(" ");
        require(peers.length==36,"Representative peer inputs must cover all supplied reaction channels");
        for(int i=0;i<36;i++){
            String name=REACTIONS[i];Spec spec=one(name,peers[i]);
            switch(name){
                case "plague":spec.set("actor:killer:_isaplant",true).set("actor:killer:_plagueversion",1);break;
                case "scourge":case "consumer":spec.set("actor:killer:_isaconsumer",true);break;
                case "white":case "plant":spec.set("actor:killer:_isaplant",true);break;
                case "silver":spec.set("actor:killer:_isenhanced",true);break;
                case "sick":spec.set("actor:victim:_segColor",phenotype("TEAL","SPIKE"));break;
                default:break;
            }
            result.add(spec);
        }
        for(int kills:new int[]{Integer.MIN_VALUE,-1,0,1,Integer.MAX_VALUE})result.add(coded(base("silver-plant-consumer-kills-"+kills,"SILVER").set("actor:killer:_isaplant",true).set("actor:killer:_isaconsumer",true).set("actor:killer:_nTotalKills",kills)));
        result.add(coded(base("silver-enhanced-plant-precedence","SILVER").set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true)));
        result.add(coded(base("silver-fungus-kill","SILVER").set("actor:killer:_isafungus",true).set("actor:killer:_nTotalKills",1)));
        for(int version:new int[]{-1,0,1})result.add(coded(base("white-plant-consumer-plague-"+version,"WHITE").set("actor:killer:_isaplant",true).set("actor:killer:_isaconsumer",true).set("actor:killer:_plagueversion",version)));
        result.add(coded(base("white-auburn-scourge-input","WHITE").set("actor:killer:_isauburn",true)));
        result.add(coded(base("white-fungus-scourge-input","PLAGUE").set("actor:killer:_isafungus",true)));
        for(int version:new int[]{-2,0,1,2,3})result.add(coded(base("fruit-version-"+version,"FRUIT").set("actor:killer:_sporeversion",version).set("actor:killer:_isaplant",true).set("actor:killer:_isaconsumer",true)));
        result.add(coded(base("spike-eye-plant-enhanced-precedence","SPIKE").set("actor:victim:_segColor",phenotype("TEAL","EYE")).set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true)));
        result.add(coded(base("spike-plant-enhanced-precedence","SPIKE").set("actor:killer:_isaplant",true).set("actor:killer:_isenhanced",true)));
        result.add(coded(base("spike-enhanced-consumer-input","SPIKE").set("actor:killer:_isenhanced",true)));
        result.add(coded(base("default-consumer-plant-precedence","GOLD").set("actor:killer:_isaplant",true).set("actor:killer:_isaconsumer",true)));
        for(String own:new String[]{"SPIKE","DARKOLIVE","BROKEN","DARKFIRE","LIGHTBROWN","GREENBROWN","LIGHT_BLUE","DEADBARK","ICE"}){
            Spec spec=base("injured-self-"+own,"ORANGE").set("actor:victim:_segColor",phenotype("TEAL",own));
            int[] values=new int[36];values[34]=1;spec.add(vector(spec,action(spec,"touchReaction"),values));spec.add(vector(spec,action(spec,"touchBrake"),values));result.add(spec);
        }
        for(int mode:new int[]{0,1,2,3,4,5,-1,6,Integer.MIN_VALUE,Integer.MAX_VALUE}){
            Spec spec=base("mode-"+mode,"GREEN");
            for(String operation:new String[]{"touchReaction","touchfriendReaction","touchBrake","touchfriendBrake"})spec.add(vector(spec,action(spec,operation),constant(mode)));
            result.add(spec);
        }
        for(String method:new String[]{"touchfriendReaction","touchfriendBrake"}){
            Spec spec=base("friendly-eye-exclusion-"+method,"EYE");spec.add(vector(spec,action(spec,method),constant(1)));result.add(spec);
        }
        String[][] actuators={{"GREEN","GREEN"},{"CYAN","GREEN"},{"CYAN","EYE"},{"CYAN","TRANSLUCENT_EYE"},{"TRANSLUCENT_TEAL","GREEN"},{"TEAL","TEAL","GREEN"},{"TEAL","CYAN","EYE"},{"CYAN","TEAL","EYE"},{"TEAL","CYAN","GREEN"}};
        for(int index=0;index<actuators.length;index++){
            Spec spec=base("actuator-order-"+index,"GREEN").colors(actuators[index]);spec.lengths=new double[actuators[index].length];for(int i=0;i<spec.lengths.length;i++)spec.lengths[i]=3d+8d*i;result.add(pair(spec,2));
        }
        for(int value:new int[]{2,3}){
            Spec spec=base("dodge-boundary-"+value,"GREEN").set("actor:victim:_candodge",true);
            spec.add(vector(spec,action(spec,"touchReaction").set("rng:seed",seedForInt(value)),constant(2)));result.add(spec);
        }
        Spec falseGate=base("ordinary-false-gate-retains-payment","GREEN");falseGate.add(vector(falseGate,action(falseGate,"touchReaction").set("rng:seed",4096L),constant(2)));result.add(falseGate);
        result.add(payment("payment-zero-oxygen",0d,1000d,2));result.add(payment("payment-partial-oxygen",0.5d,1000d,2));result.add(payment("payment-exact-oxygen",1d,1000d,2));
        result.add(payment("payment-insufficient-energy",10d,0.5d,2));result.add(payment("payment-negative-energy",10d,-1d,2));result.add(payment("payment-negative-oxygen",-0.5d,1000d,2));
        result.add(payment("payment-nan-oxygen",Double.longBitsToDouble(0xfff8000000000042L),1000d,2));
        result.add(payment("payment-nan-energy",10d,Double.longBitsToDouble(0x7ff8000000000042L),2));result.add(payment("payment-infinite-energy",10d,Double.POSITIVE_INFINITY,2));
        result.add(payment("braking-partial-oxygen",0.5d,1000d,1));
        Spec wrap=base("mode3-wrapped-cached-offsets","GREEN").set("actor:victim:x1",new int[]{Integer.MAX_VALUE,10}).set("actor:victim:y1",new int[]{Integer.MIN_VALUE,20}).set("actor:victim:x2",new int[]{Integer.MIN_VALUE,Integer.MIN_VALUE}).set("actor:victim:y2",new int[]{Integer.MAX_VALUE,Integer.MAX_VALUE});result.add(pair(wrap,3));
        Spec reverseWrap=base("mode4-wrapped-cached-offsets","GREEN").set("actor:victim:x1",new int[]{Integer.MAX_VALUE,Integer.MIN_VALUE}).set("actor:victim:y1",new int[]{Integer.MIN_VALUE,Integer.MAX_VALUE}).set("actor:victim:x2",new int[]{Integer.MIN_VALUE,Integer.MAX_VALUE}).set("actor:victim:y2",new int[]{Integer.MAX_VALUE,Integer.MIN_VALUE});result.add(pair(reverseWrap,4));
        Spec centers=base("mode5-integer-centers","GREEN").set("actor:victim:_centerX",3501).set("actor:victim:_centerY",3499).set("actor:killer:_centerX",2001).set("actor:killer:_centerY",2002);result.add(pair(centers,5));
        Spec wrappedCenters=base("mode5-wrapped-integer-centers","GREEN").set("actor:victim:_centerX",Integer.MAX_VALUE).set("actor:victim:_centerY",Integer.MIN_VALUE).set("actor:killer:_centerX",Integer.MIN_VALUE).set("actor:killer:_centerY",Integer.MAX_VALUE);result.add(pair(wrappedCenters,5));
        Spec zero=base("constructed-zero-mass-inertia","GREEN").set("actor:victim:x1",new int[2]).set("actor:victim:y1",new int[2]).set("actor:victim:x2",new int[2]).set("actor:victim:y2",new int[2]);zero.length=0;result.add(pair(zero,2));
        result.add(pair(base("controlled-nan-mass","GREEN").set("actor:victim:_mass",Double.longBitsToDouble(0xfff8000000000042L)),2));
        result.add(pair(base("controlled-negative-zero-mass","GREEN").set("actor:victim:_mass",-0d),2));
        result.add(pair(base("controlled-negative-mass","GREEN").set("actor:victim:_mass",-1d),2));
        result.add(pair(base("controlled-zero-inertia","GREEN").set("actor:victim:_I",0d),2));
        result.add(pair(base("controlled-nan-inertia","GREEN").set("actor:victim:_I",Double.longBitsToDouble(0x7ff8000000000042L)),2));
        double[][] limits={{0d,0d},{-0d,-0d},{-2d,-0.5d},{0.125d,0.0625d}};
        for(int i=0;i<limits.length;i++)result.add(pair(base("persisted-limits-"+i,"GREEN").setting("MAX_VEL",limits[i][0]).setting("MAX_ROT",limits[i][1]),4));
        for(String operation:new String[]{"touchReaction","touchBrake"})for(int side:new int[]{-1,0,1}){
            double velocity=operation.equals("touchReaction")?0.0000005d:0.00000025d;velocity=side<0?Math.nextDown(velocity):side>0?Math.nextUp(velocity):velocity;
            Spec spec=base("braking-tolerance-"+operation+"-"+side,"GREEN").setting("RUBBING",1d);
            spec.add(vector(spec,action(spec,operation).set("actor:victim:dx",velocity).set("actor:victim:dy",-velocity).set("actor:victim:dtheta",-0d),constant(1)));result.add(spec);
        }
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.ReactionOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("reactions.tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("REACTION_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
