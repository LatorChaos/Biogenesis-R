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

/** Observe original pre-contact support, friendly effects, and shield helpers on real placed organisms */
public final class ContactSupportOracle {
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
            if(action.operation.equals("pretouchEffects")||action.operation.equals("touchfriendEffects")){
                String[] args=action.argument.split(",");require(args.length==3,"Invalid contact arguments");
                Method method=Organism.class.getDeclaredMethod(action.operation,Organism.class,int.class,int.class);method.setAccessible(true);
                method.invoke(victim,actors.get(args[0]),Integer.parseInt(args[1]),Integer.parseInt(args[2]));
            }else{
                require(action.argument.equals("~"),"Unexpected helper argument");
                Method method=Organism.class.getDeclaredMethod(action.operation);method.setAccessible(true);method.invoke(victim);
            }
        }

        String finish(){row("E");cases++;return rows.toString();}
    }
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static Random copyRandom(Random random)throws Exception{ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception{
        switch(name){case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
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
        values.put("actor:victim:_energy",20d);values.put("actor:killer:_energy",10d);
        values.put("actor:victim:_usefriendeffects",2);values.put("actor:victim:_transfersenergy",false);
        values.put("actor:victim:_segmentpretoucheffects",0);values.put("actor:victim:_forestphoto",0d);
        values.put("actor:victim:_colonyPhotosynthesis",0d);values.put("actor:victim:_isaplant",false);
        values.put("actor:victim:_isaconsumer",false);values.put("actor:victim:_isafungus",false);
        values.put("actor:victim:_altruist",false);values.put("actor:killer:_altruist",false);
        values.put("actor:victim:_lengthfriend",101);values.put("actor:killer:_lengthfriend",202);
        values.put("actor:victim:_thetafriend",303);values.put("actor:killer:_thetafriend",404);
        values.put("actor:victim:_color",Color.RED);values.put("actor:victim:_framesColor",0);
        values.put("actor:killer:_color",Color.BLUE);values.put("actor:killer:_framesColor",5);
        return values;
    }
    private static Action call(String operation,String actor){return new Action(operation,actor);}
    private static Spec support(String name,String...genes){return new Spec(name).colors(genes).add(call("touchfriendEffects","killer,0,0"));}
    private static Spec cooperating(Spec spec){return spec.set("actor:victim:_altruist",true).set("actor:killer:_altruist",true);}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(String method:new String[]{"virusneutralized","lavenderinhibition","lavendershield","weaklavendershield"}){
            Spec spec=new Spec(method+"-palette").colors("WHITE","PLAGUE","CORAL","GREEN").set("actor:victim:_gold",1);
            spec.add(call(method,"~"));result.add(spec);
        }
        for(String classification:new String[]{"plant","consumer","fungus","ordinary"}){
            Spec spec=new Spec("virus-"+classification).colors("WHITE","PLAGUE","CORAL").set("actor:victim:_gold",1).set("actor:victim:_updateEffects",0);
            if(!classification.equals("ordinary"))spec.set("actor:victim:_isa"+classification,true);
            spec.add(call("virusneutralized","~"));result.add(spec);
        }
        for(String method:new String[]{"lavenderinhibition","lavendershield","weaklavendershield"}){
            for(int shield:new int[]{Integer.MIN_VALUE,-1,0,184,185,186,2219,2220,2221,Integer.MAX_VALUE}){
                Spec spec=new Spec(method+"-"+shield).set("actor:victim:_lavender",shield).set("actor:victim:_framesColor",10);
                spec.add(call(method,"~")).add(call(method,"~").set("actor:victim:_lavender",10000).set("actor:victim:_framesColor",9));result.add(spec);
            }
        }
        int paletteIndex=0;
        for(Color color:palette()){
            Spec spec=new Spec("mimic-"+paletteIndex++).colors("DARK","GREEN","DARK").set("actor:victim:_segmentpretoucheffects",2).set("actor:killer:_segColor",new Color[]{color});
            int channel=0;
            for(Field field:fields(Organism.class))if(field.getName().startsWith("_seg")&&field.getName().endsWith("Reaction"))spec.set("actor:killer:"+field.getName(),new int[]{++channel-18});
            require(channel==36,"Reaction channel inventory drift");
            spec.add(call("pretouchEffects","killer,0,0"));
            spec.add(call("pretouchEffects","killer,2,0").set("actor:victim:_segColor",phenotype("DARK","GREEN","DARK")).set("actor:victim:_segmentpretoucheffects",2).set("genome:g0:_mimicall",true));result.add(spec);
        }
        for(String actor:new String[]{"FOREST","BARK","PURPLE","DARKGRAY","GREEN"}){
            for(String peer:new String[]{"FOREST","BARK","PURPLE","DARKGRAY","GREEN"}){
                Spec spec=new Spec("colony-"+actor+"-"+peer).colors(actor).set("actor:killer:_segColor",phenotype(peer)).set("actor:victim:_forestphoto",2.25d).set("actor:victim:_mphoto",new double[]{1d}).set("actor:killer:_mphoto",new double[]{1d}).set("actor:killer:_photosynthesis",1d).set("actor:victim:_isakiller",false).set("actor:killer:_isenhanced",false);
                spec.add(call("pretouchEffects","killer,0,0")).add(call("pretouchEffects","killer,0,0"));result.add(spec);
            }
        }
        for(String peer:new String[]{"FOREST","BARK","PURPLE","DARKGRAY","GREEN"}){
            Spec spec=new Spec("colony-no-own-photo-"+peer).colors("FOREST").set("actor:killer:_segColor",phenotype(peer)).set("actor:victim:_forestphoto",2d).set("actor:victim:_mphoto",new double[]{0d}).set("actor:killer:_mphoto",new double[]{1d}).set("actor:killer:_photosynthesis",1d);
            spec.add(call("pretouchEffects","killer,0,0")).add(call("pretouchEffects","killer,0,0").set("actor:killer:_mphoto",new double[]{0d}));result.add(spec);
        }
        for(String actor:new String[]{"FOREST","BARK","PURPLE","DARKGRAY","GREEN"}){
            Spec spec=new Spec("colony-no-peer-photo-"+actor).colors(actor).set("actor:victim:_forestphoto",3d).set("actor:victim:_mphoto",new double[]{1d}).set("actor:killer:_mphoto",new double[]{0d}).set("actor:killer:_photosynthesis",1d).set("actor:victim:_isaconsumer",true).set("actor:killer:_isenhanced",false);
            spec.add(call("pretouchEffects","killer,0,0")).add(call("pretouchEffects","killer,0,0").set("actor:killer:_isenhanced",true));result.add(spec);
        }
        for(boolean transfers:new boolean[]{false,true})for(boolean forest:new boolean[]{false,true}){
            Spec spec=new Spec("symbiosis-"+transfers+"-"+forest).colors("ROSE").set("actor:victim:_colonyPhotosynthesis",0.6d).set("actor:killer:_isaplant",true).set("actor:killer:_photosynthesis",2d).set("actor:killer:_methanotrophy",4d).set("actor:killer:_forestphoto",forest?3d:0d).set("actor:killer:_transfersenergy",transfers).set("world:_CH4",100d);
            spec.add(call("pretouchEffects","killer,0,0")).add(call("pretouchEffects","killer,0,0"));result.add(spec);
        }
        for(String actor:new String[]{"PURPLE","DARKGRAY"}){
            Spec spec=new Spec("corpse-methane-"+actor).colors(actor).set("actor:victim:_segmentpretoucheffects",2).set("actor:victim:_methanotrophy",4d).set("actor:victim:_isenhanced",true).set("actor:victim:_isakiller",false).set("actor:killer:_segColor",phenotype("BROWN")).set("actor:killer:alive",false).set("world:_CH4",3d);
            spec.add(call("pretouchEffects","killer,0,0")).add(call("pretouchEffects","killer,0,0").set("actor:victim:_isakiller",true));result.add(spec);
        }
        for(String relation:new String[]{"altruist","length","theta","parent","child","sibling","unrelated","peaceful"}){
            Spec spec=support("energy-"+relation,"ROSE").set("actor:victim:_transfersenergy",true).set("actor:victim:_growthRatio",1).set("actor:victim:_energy",80d).set("actor:killer:_energy",30d).set("actor:killer:active",false).set("actor:killer:_sporeversion",3).set("actor:killer:_reproduceEnergy",60).set("actor:killer:_timeToReproduce",99);
            if(relation.equals("altruist"))cooperating(spec);
            if(!relation.equals("altruist")&&!relation.equals("unrelated")&&!relation.equals("peaceful"))spec.set("genome:g0:_familial",true);
            if(relation.equals("length"))spec.set("actor:killer:_lengthfriend",101);
            if(relation.equals("theta"))spec.set("actor:killer:_thetafriend",303);
            if(relation.equals("parent"))spec.set("actor:victim:_parentID",2);
            if(relation.equals("child"))spec.set("actor:killer:_parentID",1);
            if(relation.equals("sibling"))spec.set("actor:victim:_parentID",91).set("actor:killer:_parentID",91);
            if(relation.equals("peaceful"))spec.set("genome:g0:_peaceful",true).set("actor:victim:_isaplant",true).set("actor:killer:_transfersenergy",true).set("actor:killer:active",true).set("actor:killer:_isaplant",false).set("actor:killer:_isaconsumer",false).set("actor:killer:_isafungus",false);
            result.add(spec);
        }
        for(String kind:new String[]{"MAGENTA","GREEN","BROWN"})for(boolean active:new boolean[]{false,true}){
            Spec spec=cooperating(support("heal-"+kind+"-"+active,"MAGENTA"));spec.peerGenes=new String[]{kind,"GREEN","GREEN","GREEN","GREEN","GREEN","GREEN"};
            spec.set("actor:killer:_segColor",phenotype(kind,"LIGHTBROWN","GREENBROWN","BROKEN","LIGHT_BLUE","ICE","DARKFIRE")).set("actor:killer:_isinjured",true).set("actor:killer:active",active).set("genome:g1:_modifiesspore",7).set("actor:killer:_useextraeffects",true);
            result.add(spec);
        }
        for(double oxygen:new double[]{0d,0.005d,0.0075d,0.01d,-0.005d}){
            Spec spec=cooperating(support("heal-payments-"+bits(oxygen),"MAGENTA"));spec.peerGenes=new String[]{"MAGENTA","GREEN","GREEN"};
            spec.set("actor:killer:_segColor",phenotype("MAGENTA","GREENBROWN","BROKEN")).set("actor:killer:_isinjured",true).set("world:_O2",oxygen);result.add(spec);
        }
        for(String touch:new String[]{"MINT","GREEN","BROWN"})for(String code:new String[]{"g0","g1","~"}){
            Spec spec=cooperating(support("mint-"+touch+"-"+code,"MINT")).set("actor:killer:_segColor",phenotype(touch)).set("actor:killer:_infectedGeneticCode",code).set("actor:killer:_savedGeneticCode","g4").set("actor:killer:_reproducelate",-1);
            result.add(spec);
        }
        for(int created:new int[]{0,19,20,1000,Integer.MAX_VALUE}){
            Spec spec=support("lavender-support-"+created,"LAVENDER").set("actor:killer:_segColor",phenotype("LAVENDER")).set("actor:victim:_createlavender",created).set("actor:victim:_lavender",10000).set("actor:killer:_lavender",9999).set("actor:killer:_timeToReproduce",100).set("actor:killer:_timeToReproduceMax",99).set("actor:victim:_timeToReproduce",1).set("actor:victim:_timeToReproduceMax",1);
            result.add(spec);
        }
        for(String actor:new String[]{"DARKGRAY","FOREST","GREEN"})for(double coefficient:new double[]{Double.NaN,-0d,-1d,Double.POSITIVE_INFINITY,Double.NEGATIVE_INFINITY}){
            Spec spec=new Spec("colony-raw-coefficient-"+actor+"-"+bits(coefficient)).colors(actor).set("actor:victim:_forestphoto",2d).set("actor:victim:_mphoto",new double[]{coefficient}).set("actor:killer:_mphoto",new double[]{1d}).set("actor:killer:_photosynthesis",1d).set("actor:killer:_isenhanced",false).set("actor:victim:_isakiller",false);
            spec.add(call("pretouchEffects","killer,0,0"));result.add(spec);
        }
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.ContactSupportOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("contact_support.tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("CONTACT_SUPPORT_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
