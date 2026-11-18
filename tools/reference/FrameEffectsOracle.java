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

/** Observe original active frame helpers on real placed organisms */
public final class FrameEffectsOracle {
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
        long seed=827364;
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
            Utils.random=new Random(spec.seed);
            VisibleWorld visible=new VisibleWorld(null);world=new World(visible);
            world.addCO2(Utils.INITIAL_CO2);world.addO2(Utils.INITIAL_O2);world.addCH4(Utils.INITIAL_CH4);
            genomes.put("g0",genome(spec));
            genomes.put("g1",genome(new Spec("actor").colors(spec.peerGenes)));genomes.put("g2",genome(new Spec("peer")));
            genomes.put("g3",(GeneticCode)genomes.get("g0").clone());genomes.put("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,"canonical",spec.seed);
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
            if(action.operation.equals("pretouchEffects")||action.operation.equals("touchfriendEffects")||action.operation.equals("touchHatch")){
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
        values.put("actor:victim:_energy",20d);values.put("actor:killer:_energy",10d);
        values.put("actor:victim:_methanotrophy",0d);values.put("actor:victim:_photosynthesis",0d);
        values.put("actor:victim:_forestphoto",0d);values.put("actor:victim:_leafphoto",0d);
        values.put("actor:victim:_colonyPhotosynthesis",0d);values.put("actor:victim:_lowersymmetric",0);
        values.put("actor:victim:_healing",0);values.put("actor:victim:_jadefactor",1);
        values.put("actor:victim:_useextraeffects",false);values.put("actor:victim:_age",8);
        values.put("actor:victim:_color",Color.RED);values.put("actor:victim:_framesColor",0);
        values.put("actor:victim:_maintenance",3.25d);values.put("actor:victim:_updateEffects",0);
        return values;
    }
    private static Action call(String operation){return new Action(operation,"~");}
    private static Spec scenario(String name,String operation,String...genes){return new Spec(name).colors(genes).add(call(operation));}
    private static Spec seed(Spec spec,long seed){spec.seed=seed;return spec;}
    private static double[] repeated(double value,int count){double[] values=new double[count];Arrays.fill(values,value);return values;}
    private static Color[] repeatedColor(String name,int count)throws Exception{Color[] values=new Color[count];Arrays.fill(values,color(name));return values;}
    private static String[] allNames(){List<String> names=new ArrayList<>(Arrays.asList(COLORS));names.addAll(Arrays.asList(INTERNAL));names.add("UNKNOWN");names.add("TRANSLUCENT");return names.toArray(String[]::new);}
    private static Spec touch(String name){
        return new Spec(name).colors("GREEN").set("actor:victim:_segColor",new Color[]{Color.BLUE}).set("actor:victim:_mphoto",new double[]{-2}).set("actor:victim:active",false).set("actor:victim:_sporeversion",-2).set("actor:victim:_energy",100d).set("actor:victim:_spin",0).set("actor:victim:_healing",1).set("actor:victim:_isinfectious",false).set("actor:victim:_iscoral",false).set("actor:killer:_isaplant",true).set("actor:killer:_isaconsumer",false).set("actor:killer:_isafungus",false).set("actor:killer:_isinfectious",false).set("actor:killer:active",true);
    }
    private static Action touchCall(){return new Action("touchHatch","killer,0,0");}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(double oxygen:new double[]{1d,0.375d,0d}){
            Spec spec=seed(scenario("cached-reverse-"+bits(oxygen),"segmentsFrameEffects","GREEN","GREEN"),263);
            spec.setting("TEAL_ENERGY_CONSUMPTION",0.25d).setting("CYAN_ENERGY_CONSUMPTION",0.25d).setting("MAX_VEL",100d);
            spec.set("actor:victim:_mphoto",new double[]{-3,-4}).set("actor:victim:_mass",1d).set("actor:victim:_I",1d);
            spec.set("actor:victim:x1",new int[]{0,0}).set("actor:victim:y1",new int[]{0,0}).set("actor:victim:x2",new int[]{1,2}).set("actor:victim:y2",new int[]{0,0});
            spec.set("actor:victim:dx",0d).set("actor:victim:dy",0d).set("actor:victim:dtheta",0d).set("actor:victim:_energy",1d).set("world:_O2",oxygen);result.add(spec);
        }
        result.add(seed(scenario("cached-energy-shortfall","segmentsFrameEffects","GREEN"),263).setting("CYAN_ENERGY_CONSUMPTION",1d).set("actor:victim:_mphoto",new double[]{-3}).set("actor:victim:_energy",0.5d));
        result.add(seed(scenario("cached-filter-and-random-failure","segmentsFrameEffects","GREEN","GREEN","GREEN","GREEN","GREEN","GREEN","GREEN","GREEN"),0).set("actor:victim:_mphoto",new double[]{-4,-3,-5,-2,0,-0d,Double.NaN,Double.NEGATIVE_INFINITY}));
        result.add(seed(scenario("cached-wrapped-endpoints-and-clamping","segmentsFrameEffects","CYAN"),263).set("actor:victim:_mphoto",new double[]{-3}).set("actor:victim:_mass",0.001d).set("actor:victim:_I",0.001d).set("actor:victim:x1",new int[]{Integer.MAX_VALUE}).set("actor:victim:x2",new int[]{Integer.MIN_VALUE}).set("actor:victim:y1",new int[]{Integer.MIN_VALUE}).set("actor:victim:y2",new int[]{Integer.MAX_VALUE}));
        Spec zero=seed(scenario("cached-real-zero-length","segmentsFrameEffects","CYAN"),263);zero.length=0;zero.set("actor:victim:_mphoto",new double[]{-3});result.add(zero);
        result.add(seed(scenario("cached-nonfinite-velocity","segmentsFrameEffects","CYAN"),263).set("actor:victim:_mphoto",new double[]{-3}).set("actor:victim:dx",Double.NaN).set("actor:victim:dy",Double.POSITIVE_INFINITY).set("actor:victim:dtheta",Double.NEGATIVE_INFINITY));
        for(int age:new int[]{7,8,9})result.add(scenario("leaf-methane-age-"+age,"leafExtraEffects","LEAF").set("actor:victim:_age",age).set("actor:victim:_methanotrophy",2d).set("actor:victim:_photosynthesis",4d).set("actor:victim:_leafphoto",1d).set("world:_CH4",1d).setting("DRAIN_SUBS_DIVISOR",1));
        Spec tints=new Spec("leaf-temporary-tint-palette").colors("LEAF").set("actor:victim:_photosynthesis",4d).set("actor:victim:_leafphoto",1d);
        for(String tint:new String[]{"BLOND","LIGHTBROWN","YELLOW","GREENBROWN","DARKLILAC","RED"})tints.add(call("leafExtraEffects").set("actor:victim:_color",color(tint)).set("actor:victim:_framesColor",1));
        tints.add(call("leafExtraEffects").set("actor:victim:_color",color("BLOND")).set("actor:victim:_framesColor",2));result.add(tints);
        for(double colony:new double[]{3d,0d,-1d,Double.NaN}){
            Spec spec=scenario("leaf-colony-low-carbon-failure-"+bits(colony),"leafExtraEffects","LEAF").set("actor:victim:_photosynthesis",4d).set("actor:victim:_leafphoto",1d).set("actor:victim:_colonyPhotosynthesis",colony).set("actor:victim:_lowersymmetric",10).set("world:_CO2",0d);result.add(spec);
        }
        result.add(scenario("leaf-low-carbon-success","leafExtraEffects","LEAF").set("actor:victim:_photosynthesis",4d).set("actor:victim:_leafphoto",1d).set("actor:victim:_colonyPhotosynthesis",2d).set("actor:victim:_lowersymmetric",1).set("world:_CO2",0.5d).setting("DRAIN_SUBS_DIVISOR",1));
        result.add(scenario("leaf-zero-amount-signed-zero","leafExtraEffects","LEAF").set("actor:victim:_energy",-0d).set("world:_O2",-0d).set("world:_CO2",1d));
        for(double amount:new double[]{-1d,Double.NaN,Double.POSITIVE_INFINITY,Double.NEGATIVE_INFINITY})result.add(scenario("leaf-raw-photo-"+bits(amount),"leafExtraEffects","LEAF").set("actor:victim:_photosynthesis",amount));
        for(double amount:new double[]{-1d,Double.NaN,Double.POSITIVE_INFINITY})result.add(scenario("leaf-raw-carbon-"+bits(amount),"leafExtraEffects","LEAF").set("actor:victim:_photosynthesis",1d).set("world:_CO2",amount));
        for(double amount:new double[]{-1d,Double.NaN,Double.POSITIVE_INFINITY})result.add(scenario("leaf-raw-methane-"+bits(amount),"leafExtraEffects","LEAF").set("actor:victim:_methanotrophy",1d).set("world:_CH4",amount));
        for(int bound:new int[]{1,2000000000})result.add(scenario("jade-darkjade-bound-"+bound,"jadeExtraEffects","JADE").set("actor:victim:_segColor",phenotype("DARKJADE")).set("actor:victim:_jadefactor",bound).set("actor:victim:_photosynthesis",10d).set("actor:victim:_mphoto",new double[]{2}));
        for(long time:new long[]{0,1,2,-1,Long.MIN_VALUE}){
            result.add(scenario("jade-summer-time-"+time,"jadeExtraEffects","SUMMER").set("actor:victim:_segColor",phenotype("DARKGREEN")).set("actor:victim:_summerinactivity",0).set("actor:victim:_photosynthesis",10d).set("actor:victim:_mphoto",new double[]{2}).set("statistics:time",time));
        }
        for(String original:new String[]{"FOREST","LEAF","LIME","GREEN","UNKNOWN"})for(boolean crowded:new boolean[]{false,true}){
            Spec spec=scenario("jade-original-"+original+"-crowded-"+crowded,"jadeExtraEffects",original).set("actor:victim:_segColor",phenotype("DARKGREEN")).set("actor:victim:_crowded",crowded).set("actor:victim:_photosynthesis",10d).set("actor:victim:_forestphoto",7d).set("actor:victim:_leafphoto",2d).set("actor:victim:_mphoto",new double[]{2});result.add(spec);
        }
        for(int frames:new int[]{1,2})result.add(scenario("jade-leaf-tinted-"+frames,"jadeExtraEffects","LEAF").set("actor:victim:_segColor",phenotype("DARKGREEN")).set("actor:victim:_photosynthesis",10d).set("actor:victim:_leafphoto",2d).set("actor:victim:_mphoto",new double[]{2}).set("actor:victim:_color",color("BLOND")).set("actor:victim:_framesColor",frames));
        result.add(seed(scenario("jade-restoration-alters-later-healing","jadeExtraEffects","MAGENTA","GREEN"),0).set("actor:victim:_segColor",phenotype("DARKFIRE","BROKEN")).set("actor:victim:_healing",Integer.MAX_VALUE));
        for(String operation:new String[]{"jadeExtraEffects","segmentsHealingEffects","segmentsUpdateEffects","plantsUpdateEffects"}){
            for(int symmetry:new int[]{1,3,8}){
                Spec spec=seed(scenario(operation+"-paid-darkfire-symmetry-"+symmetry,operation,"MAGENTA"),0);spec.symmetry=symmetry;spec.mirror=symmetry==3?1:0;
                spec.setting("HEALING",1).set("actor:victim:_healing",Integer.MAX_VALUE).set("actor:victim:_segColor",repeatedColor("DARKFIRE",symmetry)).set("actor:victim:_isfrozen",true).set("actor:victim:_allfrozen",true);result.add(spec);
            }
        }
        for(int threshold:new int[]{-1,0,1}){
            result.add(scenario("healing-unconditional-threshold-"+threshold,"segmentsHealingEffects","GREEN").setting("HEALING",1).set("actor:victim:_segColor",phenotype("BROKEN")).set("actor:victim:_healing",threshold));
        }
        for(double oxygen:new double[]{0d,0.005d,0.01d})result.add(scenario("healing-oxygen-"+bits(oxygen),"segmentsHealingEffects","GREEN","GREEN").setting("HEALING",1).set("actor:victim:_segColor",phenotype("ICE","BROKEN")).set("actor:victim:_healing",1).set("world:_O2",oxygen));
        result.add(scenario("healing-zero-cost-empty-gas","segmentsHealingEffects","GREEN").setting("HEALING",1).setting("MAGENTA_ENERGY_CONSUMPTION",0d).set("actor:victim:_segColor",phenotype("BROKEN")).set("actor:victim:_energy",0d).set("world:_O2",0d));
        result.add(scenario("healing-insufficient-energy","segmentsHealingEffects","GREEN").setting("HEALING",1).set("actor:victim:_segColor",phenotype("BROKEN")).set("actor:victim:_energy",0.005d));
        result.add(scenario("healing-nonfinite-energy-and-oxygen","segmentsHealingEffects","GREEN").setting("HEALING",1).set("actor:victim:_segColor",phenotype("BROKEN")).set("actor:victim:_energy",Double.NaN).set("world:_O2",Double.NaN));
        for(String operation:new String[]{"jadeExtraEffects","segmentsHealingEffects","segmentsExtraEffects","segmentsExtraPlantsEffects"}){
            String[] originals={"TEAL","CYAN","OLIVE","LILAC","SKY","FIRE","GREEN","GREEN","GREEN","GREEN","GREEN","UNKNOWN","TRANSLUCENT"};
            Spec spec=seed(scenario(operation+"-mixed-current-phenotype",operation,originals),33).set("actor:victim:_segColor",phenotype("TEAL","CYAN","DARKOLIVE","DARKLILAC","DEEPSKY","DARKFIRE","GREENBROWN","LIGHTBROWN","BROKEN","LIGHT_BLUE","ICE","UNKNOWN","TRANSLUCENT")).set("actor:victim:_healing",1).setting("HEALING",1).set("actor:victim:_mphoto",repeated(-4,originals.length));result.add(spec);
        }
        for(double gate:new double[]{-1d,-0d,0d,Double.NaN,Double.NEGATIVE_INFINITY}){
            result.add(seed(scenario("plant-extra-cache-gate-"+bits(gate),"segmentsExtraPlantsEffects","OLIVE","LILAC","SKY","FIRE","CYAN","TEAL"),33).set("actor:victim:_segColor",phenotype("DARKOLIVE","DARKLILAC","DEEPSKY","DARKFIRE","CYAN","TEAL")).set("actor:victim:_mphoto",repeated(gate,6)));
        }
        for(String operation:new String[]{"plantsUpdateEffects","segmentsUpdateEffects","darkfireRestoreEffects"}){
            String[] names=allNames();double[] coefficients=new double[names.length];for(int i=0;i<coefficients.length;i++)coefficients[i]=i+0.25d;
            result.add(scenario(operation+"-complete-color-classification",operation,names).set("actor:victim:_mphoto",coefficients).set("actor:victim:_useextraeffects",false).set("actor:victim:_gold",1).set("actor:victim:_isgray",false));
        }
        String[] costs={"TEAL","VIOLET","PINK","LAVENDER","FLOWER","EYE","OLIVE","SKY","BLUE","FALLOW","CORAL","ROSE","VISION","GOLD","DARK","DARKOLIVE","DEEPSKY","SPORE","AUBURN","MAROON","INDIGO"};
        for(String classification:new String[]{"ordinary","consumer","fungus"}){
            Spec spec=scenario("nonplant-maintenance-"+classification,"segmentsUpdateEffects",costs).set("actor:victim:_mphoto",repeated(0,costs.length)).set("actor:victim:_isaconsumer",classification.equals("consumer")).set("actor:victim:_isafungus",classification.equals("fungus")).set("actor:victim:_isgray",classification.equals("consumer")).set("actor:victim:_gold",1).set("actor:victim:_isakiller",classification.equals("fungus")).set("actor:victim:_skyversion",classification.equals("ordinary")?1:2).set("actor:victim:_haseyes",!classification.equals("ordinary"));result.add(spec);
        }
        for(boolean infectious:new boolean[]{false,true})for(int fallow:new int[]{3,4}){
            result.add(scenario("plant-maintenance-fallow-"+fallow+"-infectious-"+infectious,"plantsUpdateEffects",costs).set("actor:victim:_mphoto",repeated(0,costs.length)).set("actor:victim:_fallowversion",fallow).set("actor:victim:_isinfectious",infectious).set("actor:victim:_isakiller",infectious).set("actor:victim:_isonlyc4",infectious).set("actor:victim:_isgray",infectious).set("actor:victim:_haseyes",infectious));
        }
        for(long time:new long[]{0,1,2,-1})result.add(scenario("plants-summer-winter-time-"+time,"plantsUpdateEffects","SUMMER","WINTER","FOREST","LEAF","LIME","PURPLE").set("statistics:time",time).set("actor:victim:_summerinactivity",0).set("actor:victim:_crowded",time==1).set("actor:victim:_mphoto",new double[]{1,2,3,4,5,6}).set("actor:victim:_colonyPhotosynthesis",2d).set("actor:victim:_color",color("BLOND")).set("actor:victim:_framesColor",1).set("world:_CH4",2d));
        for(String original:new String[]{"SUMMER","FOREST","LEAF","LIME","GREEN"}){
            Spec spec=scenario("plants-darkgreen-"+original,"plantsUpdateEffects",original).set("actor:victim:_segColor",phenotype("DARKGREEN")).set("actor:victim:_mphoto",new double[]{2}).set("actor:victim:_crowded",true).set("actor:victim:_colonyPhotosynthesis",1d).set("actor:victim:_color",color("BLOND")).set("actor:victim:_framesColor",1).set("statistics:time",1L);result.add(spec);
        }
        result.add(scenario("plants-positive-leaf-underflow","plantsUpdateEffects","GREEN","LEAF").set("actor:victim:_mphoto",new double[]{1,Double.longBitsToDouble(1)}).set("actor:victim:_color",color("BLOND")).set("actor:victim:_framesColor",1));
        result.add(scenario("plants-positive-methane-underflow","plantsUpdateEffects","PURPLE").set("actor:victim:_mphoto",new double[]{Double.longBitsToDouble(1)}).set("actor:victim:_energy",-0d).set("world:_O2",-0d).set("world:_CH4",1d));
        for(double coefficient:new double[]{-1d,Double.NaN,Double.POSITIVE_INFINITY})result.add(scenario("plants-nonpositive-photo-"+bits(coefficient),"plantsUpdateEffects","GREEN").set("actor:victim:_mphoto",new double[]{coefficient}).set("actor:victim:_colonyPhotosynthesis",7d));
        result.add(scenario("plants-low-carbon-colony-failure","plantsUpdateEffects","GREEN","FOREST","LEAF").set("actor:victim:_mphoto",new double[]{1,2,3}).set("actor:victim:_colonyPhotosynthesis",7d).set("actor:victim:_lowersymmetric",10).set("world:_CO2",0d));
        for(int mirror:new int[]{0,1}){
            Spec spec=scenario("darkfire-symmetry3-integer-factor-"+mirror,"darkfireRestoreEffects","MAGENTA");spec.symmetry=3;spec.mirror=mirror;spec.length=12.75;spec.set("actor:victim:_isfrozen",true).set("actor:victim:_allfrozen",true).set("actor:victim:_isinjured",true).add(call("darkfireRestoreEffects"));result.add(spec);
            Spec huge=scenario("darkfire-valid-large-healing-wrap-"+mirror,"darkfireRestoreEffects","MAGENTA").huge();huge.symmetry=3;huge.mirror=mirror;huge.length=200000000;result.add(huge);
        }
        String[] touchInputs=("RED:1 GREEN:7 BLUE:9 CYAN:7 WHITE:7 GRAY:8 MAGENTA:7 PINK:5 CORAL:7 ORANGE:3 FOREST:7 SPRING:7 LEAF:7 SUMMER:7 LIME:7 BARK:9 VIOLET:9 TEAL:7 EYE:7 MAROON:3 OLIVE:9 MINT:7 CREAM:0 ROSE:7 DARK:7 OCHRE:9 SKY:9 LILAC:8 SILVER:7 FIRE:2 DARKGRAY:7 GOLD:7 BLOND:7 FLOWER:7 AUBURN:7 PLAGUE:7 SPIKE:8 INDIGO:7 LAVENDER:7 FALLOW:9 SPORE:7 JADE:7 C4:7 GRASS:7 PURPLE:7 YELLOW:7 WINTER:7 OLDBARK:9 DARKJADE:7 DARKGREEN:7 DARKFIRE:7 DARKLILAC:8 DEEPSKY:9 DARKOLIVE:9 SPIKEPOINT:8 FRUIT:0 VISION:7 ICE:9 LIGHT_BLUE:9 LIGHTBROWN:7 GREENBROWN:7 BROKEN:7 DEADBARK:9 BROWN:0 UNKNOWN:7 TRANSLUCENT:7").split(" ");
        require(touchInputs.length==allNames().length,"Touch palette input inventory drift");
        for(int i=0;i<touchInputs.length;i++){
            String[] input=touchInputs[i].split(":");require(input[0].equals(allNames()[i]),"Touch palette input order drift");
            Spec spec=touch("touch-palette-"+input[0]).set("actor:killer:_segColor",phenotype(input[0])).set("genome:g0:_adaptspore",-1);
            spec.add(touchCall()).add(touchCall().set("genome:g0:_adaptspore",Integer.parseInt(input[1])).set("actor:victim:_spin",input[0].equals("CREAM")||input[0].equals("BROWN")?-1:0));result.add(spec);
        }
        for(String color:new String[]{"SILVER","CORAL","UNKNOWN"})for(String classification:new String[]{"consumer-first","fungus-first","plant-infectious","plant-ordinary","active-ordinary","inactive-ordinary"}){
            Spec spec=touch("touch-precedence-"+color+"-"+classification).set("actor:killer:_segColor",phenotype(color));int adaptation;
            if(classification.equals("consumer-first")){spec.set("actor:killer:_isaconsumer",true).set("actor:killer:_isafungus",true);adaptation=color.equals("SILVER")?2:4;}
            else if(classification.equals("fungus-first")){spec.set("actor:killer:_isafungus",true);adaptation=5;}
            else if(classification.equals("plant-infectious")){spec.set("actor:killer:_isinfectious",true);adaptation=6;}
            else if(classification.equals("plant-ordinary")){adaptation=7;}
            else{spec.set("actor:killer:_isaplant",false).set("actor:killer:active",classification.equals("active-ordinary"));adaptation=0;}
            spec.set("genome:g0:_adaptspore",-1).add(touchCall()).add(touchCall().set("genome:g0:_adaptspore",adaptation));result.add(spec);
        }
        for(String gate:new String[]{"infectious","coral"})result.add(touch("touch-coral-actor-block-"+gate).set("actor:killer:_segColor",phenotype("CORAL")).set("actor:killer:_isaplant",false).set("genome:g0:_adaptspore",0).set("actor:victim:_is"+gate,true).add(touchCall()));
        for(String color:new String[]{"OLIVE","DARKOLIVE","SKY","DEEPSKY","VIOLET"})result.add(touch("touch-healing-block-"+color).set("actor:killer:_segColor",phenotype(color)).set("genome:g0:_adaptspore",9).set("actor:victim:_healing",0).add(touchCall()));
        for(String color:new String[]{"LILAC","DARKLILAC"})result.add(touch("touch-modified-lilac-block-"+color).set("actor:killer:_segColor",phenotype(color)).set("genome:g0:_adaptspore",8).set("genome:g1:_modifieslilac",true).add(touchCall()));
        for(String classification:new String[]{"consumer","fungus"})result.add(touch("touch-blue-classification-block-"+classification).set("actor:killer:_segColor",phenotype("BLUE")).set("genome:g0:_adaptspore",9).set("actor:killer:_isa"+classification,true).add(touchCall()));
        result.add(touch("touch-fruit-peer-genotype").set("actor:killer:_segColor",phenotype("FRUIT")).set("genome:g1:_modifiesspore",9).set("genome:g0:_adaptspore",0).add(touchCall()).add(touchCall().set("genome:g0:_adaptspore",6)));
        for(int adaptation:new int[]{-10,-9,Integer.MIN_VALUE,Integer.MAX_VALUE})result.add(touch("touch-raw-adaptation-"+adaptation).set("actor:killer:_segColor",phenotype("UNKNOWN")).set("actor:killer:_isaplant",adaptation==Integer.MAX_VALUE).set("genome:g0:_adaptspore",adaptation).add(touchCall()));
        for(double oxygen:new double[]{0d,0.005d,0.5d,1d})result.add(touch("touch-hatch-gas-"+bits(oxygen)).setting("SPORE_ENERGY_CONSUMPTION",1d).set("actor:killer:_segColor",phenotype("RED")).set("genome:g0:_adaptspore",1).set("world:_O2",oxygen).add(touchCall()));
        result.add(touch("touch-hatch-fallback-death").setting("SPORE_ENERGY_CONSUMPTION",1d).set("actor:killer:_segColor",phenotype("RED")).set("genome:g0:_adaptspore",1).set("actor:victim:_energy",0.005d).add(touchCall()));
        result.add(touch("touch-hatch-nan-payment").set("actor:killer:_segColor",phenotype("RED")).set("genome:g0:_adaptspore",1).set("actor:victim:_energy",Double.NaN).set("world:_O2",Double.NaN).add(touchCall()));
        result.add(touch("touch-hatch-zero-cost-signed-zero").setting("SPORE_ENERGY_CONSUMPTION",0d).set("actor:killer:_segColor",phenotype("RED")).set("genome:g0:_adaptspore",1).set("actor:victim:_energy",-0d).set("world:_O2",-0d).add(touchCall()));
        for(int modifier:new int[]{9,10}){
            Spec spec=touch("touch-hatch-eye-modifier-"+modifier).colors("EYE","GREEN").set("actor:victim:_segColor",phenotype("BLUE","BLUE")).set("actor:victim:_mphoto",new double[]{-2,-2}).set("genome:g0:_modifiesspore",modifier).set("actor:killer:_segColor",phenotype("RED")).set("genome:g0:_adaptspore",1).set("actor:victim:_age",modifier==9?0:Integer.MAX_VALUE).add(touchCall());result.add(spec);
        }
        for(String[] pair:new String[][]{{"OLIVE","DARKOLIVE"},{"LILAC","DARKLILAC"},{"SKY","DEEPSKY"}})result.add(seed(scenario("ability-confirmed-success-"+pair[0],"segmentsExtraPlantsEffects",pair[0]),32).set("actor:victim:_segColor",phenotype(pair[1])).set("actor:victim:_mphoto",new double[]{-1}));
        result.add(seed(scenario("natural-darkfire-confirmed-success","segmentsExtraEffects","FIRE"),32).set("actor:victim:_segColor",phenotype("DARKFIRE")));
        Spec naturalMirror=seed(scenario("natural-darkfire-mirrored-three","segmentsExtraEffects","FIRE"),32);naturalMirror.symmetry=3;naturalMirror.mirror=1;naturalMirror.set("actor:victim:_segColor",repeatedColor("DARKFIRE",3));result.add(naturalMirror);
        result.add(seed(scenario("paid-darkfire-confirmed-natural-success","segmentsHealingEffects","MAGENTA"),47).setting("HEALING",1).set("actor:victim:_healing",Integer.MAX_VALUE).set("actor:victim:_segColor",phenotype("DARKFIRE")));
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.FrameEffectsOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("frame_effects.tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("FRAME_EFFECTS_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
