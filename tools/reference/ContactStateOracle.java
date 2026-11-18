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

/** Observe original contact-state helpers and restoration on real placed organisms */
public final class ContactStateOracle {
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
        String[] genes={"GREEN"};
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
            genomes.put("g1",genome(new Spec("actor")));genomes.put("g2",genome(new Spec("peer")));
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
            Organism actor=action.argument.equals("self")?victim:actors.get(action.argument);
            switch(action.operation){
                case "survive":victim.survive(actor);break;
                case "inhibited":victim.inhibited(actor);break;
                case "frozen":victim.frozen(actor);break;
                case "paralyzed":victim.paralyzed(actor);break;
                case "neutralized":victim.neutralized(actor);break;
                case "segmentsRestoreEffects":victim.segmentsRestoreEffects();break;
                default:throw new IllegalArgumentException("Unknown actual operation");
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
        values.put("actor:victim:_energy",20d);values.put("actor:killer:_energy",1000d);
        values.put("actor:victim:_healing",10);values.put("actor:victim:_antiviral",0);values.put("actor:victim:_gold",0);values.put("actor:victim:_sporeversion",1);
        for(String name:new String[]{"_isaplant","_isaconsumer","_isafungus","_iscoral","_isinjured","_isfrozen","_allfrozen","_useextraeffects","_isonlyc4"})values.put("actor:victim:"+name,false);
        for(String name:new String[]{"_canmove","_canreact","_useframemovement","_candodge","_dodge"})values.put("actor:victim:"+name,true);
        for(String name:new String[]{"_isaplant","_isaconsumer","_isafungus","_iscoral"})values.put("actor:killer:"+name,false);
        values.put("actor:victim:_updateEffects",0);values.put("actor:victim:_color",Color.MAGENTA);values.put("actor:victim:_framesColor",7);
        values.put("actor:killer:_color",Color.RED);values.put("actor:killer:_framesColor",5);
        double[] photos=new double[segments];for(int i=0;i<segments;i++)photos[i]=i+0.25;values.put("actor:victim:_mphoto",photos);return values;
    }
    private static Action call(String operation,String actor){return new Action(operation,actor);}
    private static Spec operation(String name,String operation,String...colors)throws Exception{
        Spec spec=new Spec(name);spec.genes=new String[colors.length];Arrays.fill(spec.genes,"GREEN");spec.set("actor:victim:_segColor",phenotype(colors));spec.add(call(operation,"killer"));return spec;
    }
    private static Spec restore(String name,String...genes){return new Spec(name).colors(genes).add(call("segmentsRestoreEffects","~"));}
    private static Spec costs(Spec spec){return spec.setting("SKY_ENERGY_CONSUMPTION",0.25d).setting("FALLOW_ENERGY_CONSUMPTION",0.25d).setting("MINT_ENERGY_CONSUMPTION",2.5d);}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(String method:new String[]{"survive","inhibited","frozen","paralyzed","neutralized","segmentsRestoreEffects"}){
            Spec spec=new Spec("all-current-colors-"+method);spec.genes=new String[66];Arrays.fill(spec.genes,"GREEN");
            spec.set("actor:victim:_segColor",palette()).set("actor:victim:_gold",1).setting("SKY_ENERGY_CONSUMPTION",0d).setting("FALLOW_ENERGY_CONSUMPTION",0d).setting("MINT_ENERGY_CONSUMPTION",0d);
            spec.add(call(method,method.equals("segmentsRestoreEffects")?"~":"killer"));result.add(spec);
        }
        result.add(operation("survive-unconditional-tint","survive","GREEN").set("actor:killer:_energy",-1d));
        Spec surviveSelf=operation("survive-self-full-argb","survive","MAGENTA","GREEN");surviveSelf.set("actor:victim:_segColor",new Color[]{Color.MAGENTA,new Color(0x12ff00ff,true)});surviveSelf.actions.set(0,call("survive","self"));result.add(surviveSelf);
        String[] paying={"inhibited","frozen","paralyzed","neutralized"};
        String[][] targets={{"SPORE","AUBURN","YELLOW"},{"TEAL","CYAN","SPRING"},{"TEAL","CYAN","TEAL"},{"WHITE","PLAGUE","FALLOW"}};
        for(int index=0;index<paying.length;index++){
            String method=paying[index];String[] current=targets[index];
            result.add(costs(operation(method+"-partial-oxygen",method,current)).set("world:_O2",0.375d).set("actor:killer:_energy",1d));
            result.add(costs(operation(method+"-empty-oxygen",method,current)).set("world:_O2",0d));
            result.add(costs(operation(method+"-energy-guard",method,current)).set("world:_O2",10d).set("actor:killer:_energy",0.125d));
            Spec self=costs(operation(method+"-self-payment-tint",method,current)).set("world:_O2",10d).set("actor:victim:_energy",2d);self.actions.set(0,call(method,"self"));result.add(self);
        }
        result.add(costs(operation("frozen-negative-oxygen-cascade","frozen","TEAL","CYAN","SPRING")).set("world:_O2",-0.125d));
        result.add(costs(operation("frozen-nan-oxygen","frozen","TEAL","CYAN")).set("world:_O2",Double.longBitsToDouble(0xfff8000000000042L)));
        result.add(costs(operation("frozen-infinite-oxygen","frozen","TEAL","CYAN")).set("world:_O2",Double.POSITIVE_INFINITY));
        result.add(costs(operation("frozen-negative-distinct-recipient-energy","frozen","TEAL")).set("actor:victim:_energy",-1d));
        result.add(costs(operation("frozen-nan-distinct-recipient-energy","frozen","TEAL")).set("actor:victim:_energy",Double.longBitsToDouble(0x7ff8000000000042L)));
        for(String state:new String[]{"negative","nan","infinite"}){
            Spec spec=costs(operation("self-payment-energy-"+state,"frozen","TEAL","CYAN"));spec.actions.set(0,call("frozen","self"));
            spec.set("actor:victim:_energy",state.equals("negative")?-1d:state.equals("nan")?Double.longBitsToDouble(0xfff8000000000042L):Double.POSITIVE_INFINITY);result.add(spec);
        }
        Spec zero=operation("zero-cost-negative-self-energy","inhibited","YELLOW").setting("FALLOW_ENERGY_CONSUMPTION",0d).set("actor:victim:_energy",-1d);zero.actions.set(0,call("inhibited","self"));result.add(zero);
        Spec spore=costs(operation("inhibition-spore-version-gate","inhibited","SPORE")).set("actor:victim:_sporeversion",0);spore.add(call("inhibited","killer").set("actor:victim:_sporeversion",1));result.add(spore);
        result.add(operation("inhibition-plant-damage","inhibited","SPORE","AUBURN","INDIGO","YELLOW","BLOND").set("actor:victim:_isaplant",true));
        for(boolean passive:new boolean[]{false,true}){
            for(boolean paid:new boolean[]{false,true}){
                Spec spec=costs(operation("teal-modifier-"+passive+"-"+paid,"frozen","TEAL")).set("world:_O2",paid?10d:0d).set("actor:victim:_useframemovement",false);spec.passive=passive;result.add(spec);
            }
        }
        for(boolean modified:new boolean[]{false,true}){
            for(boolean paid:new boolean[]{false,true}){
                Spec spec=costs(operation("sky-modifier-"+modified+"-"+paid,"frozen","SKY","DEEPSKY")).set("world:_O2",paid?10d:0d).set("actor:victim:_skyversion",2);spec.modifiedSky=modified;result.add(spec);
            }
        }
        result.add(operation("sky-plant-keeps-update-cache","frozen","SKY").set("actor:victim:_isaplant",true).set("actor:victim:_skyversion",1));
        result.add(operation("winter-damage-keeps-photo-update-cache","frozen","WINTER","GREENBROWN"));
        result.add(operation("bark-freezes-without-injury","frozen","BARK","OLDBARK"));
        for(String original:new String[]{"JADE","SPRING","GREEN"}){
            Spec spec=costs(new Spec("darkgreen-original-"+original)).colors(original).set("actor:victim:_segColor",phenotype("DARKGREEN")).set("world:_O2",0d).set("actor:victim:_jadefactor",9).add(call("frozen","killer"));result.add(spec);
        }
        Spec alphaJade=costs(new Spec("darkgreen-transparent-original-jade")).set("world:_O2",0d).set("actor:victim:_jadefactor",9);
        alphaJade.add(call("frozen","killer").set("genome:g0:gene0._color",new Color(0x1200a86b,true)).set("actor:victim:_segColor",phenotype("DARKGREEN")));result.add(alphaJade);
        Spec delays=costs(new Spec("frozen-spring-wrapping-jade-delay")).colors("SPRING","JADE").setting("DARKJADE_DELAY",Integer.MAX_VALUE).set("world:_O2",0d);delays.symmetry=3;delays.add(call("frozen","killer"));result.add(delays);
        Spec c4=operation("frozen-and-paralyzed-only-c4-dodge-retention","frozen","CYAN").set("actor:victim:_isonlyc4",true);
        c4.add(call("paralyzed","self").set("actor:victim:_segColor",phenotype("CYAN")).set("actor:victim:_candodge",true).set("actor:victim:_dodge",true));result.add(c4);
        Spec paral=operation("paralyzed-retains-full-freeze-caches","paralyzed","GREEN").set("actor:victim:_allfrozen",false).set("actor:victim:_skyversion",2).set("actor:victim:_jadefactor",99).set("actor:victim:_spin",7);result.add(paral);
        for(String kind:new String[]{"plant","consumer","fungus","ordinary-gold","ordinary-no-gold"}){
            Spec spec=operation("neutralized-coral-"+kind,"neutralized","CORAL").set("actor:victim:_gold",kind.equals("ordinary-no-gold")?0:1);
            if(kind.equals("plant"))spec.set("actor:victim:_isaplant",true);if(kind.equals("consumer"))spec.set("actor:victim:_isaconsumer",true);if(kind.equals("fungus"))spec.set("actor:victim:_isafungus",true);result.add(spec);
        }
        result.add(operation("neutralized-white-plague-plant","neutralized","WHITE","PLAGUE","FALLOW").set("actor:victim:_isaplant",true));
        result.add(operation("neutralized-white-plague-consumer","neutralized","WHITE","PLAGUE","FALLOW").set("actor:victim:_isaconsumer",true));
        result.add(operation("neutralized-white-plague-coral","neutralized","WHITE","PLAGUE","FALLOW").set("actor:victim:_iscoral",true));
        result.add(operation("neutralized-without-healing","neutralized","WHITE","PLAGUE").set("actor:victim:_healing",0));
        result.add(operation("neutralized-plague-version-suppresses-darkfire","neutralized","WHITE","PLAGUE").set("actor:victim:_plagueversion",1));
        result.add(operation("neutralized-negative-antiviral-protects","neutralized","WHITE","FALLOW").set("actor:victim:_antiviral",Integer.MIN_VALUE));
        Spec immunity=operation("neutralized-classified-actor-immunity","neutralized","WHITE","PLAGUE","FALLOW").set("actor:victim:_antiviral",1);
        String previous=null;for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus","_iscoral"}){
            Action action=call("neutralized","killer").set("actor:victim:_segColor",phenotype("WHITE","PLAGUE","FALLOW")).set("actor:killer:"+flag,true);
            if(previous!=null)action.set("actor:killer:"+previous,false);immunity.add(action);previous=flag;
        }result.add(immunity);
        Spec immuneSelf=operation("neutralized-self-fungus-overcomes-antiviral","neutralized","WHITE","FALLOW").set("actor:victim:_antiviral",1).set("actor:victim:_isafungus",true);immuneSelf.actions.set(0,call("neutralized","self"));result.add(immuneSelf);
        Spec retained=restore("restoration-reset-and-retained-caches","GREEN").set("actor:victim:_healing",777).set("actor:victim:_antiviral",555).set("actor:victim:_createlavender",444).set("actor:victim:_yellowCounter",55).set("actor:victim:_indigo",333).set("actor:victim:_earlyReproduceEnergy",1234).set("actor:victim:_reproducelate",60).set("actor:victim:_reproduceEnergy",1001).set("actor:victim:_gold",4).set("actor:victim:_max_age",777).set("actor:victim:_skyversion",2).set("actor:victim:_jadefactor",9).set("actor:victim:_spin",17).set("actor:victim:_sporetime",321).set("actor:victim:_photosynthesis",9.25d).set("actor:victim:_methanotrophy",7.5d).set("actor:victim:_colonyPhotosynthesis",3.75d).set("actor:victim:_forestphoto",2.5d).set("actor:victim:_leafphoto",1.25d).set("actor:victim:_maintenance",0.5d).set("actor:victim:_lavender",123).set("actor:victim:_isinjured",true).set("actor:victim:_isfrozen",true).set("actor:victim:_allfrozen",true).set("actor:victim:_isauburn",true).set("actor:victim:_isaplant",true);
        retained.add(call("segmentsRestoreEffects","~"));result.add(retained);
        for(boolean passive:new boolean[]{false,true}){
            Spec spec=restore("restoration-teal-passive-"+passive,"TEAL").set("actor:victim:_canmove",false).set("actor:victim:_canreact",false).set("actor:victim:_useframemovement",false);spec.passive=passive;result.add(spec);
        }
        for(int mirror:new int[]{0,1}){
            Spec spec=restore("restoration-symmetry3-mirror-"+mirror,"YELLOW","INDIGO","MINT","MAGENTA");spec.symmetry=3;spec.mirror=mirror;spec.length=12.25;result.add(spec);
        }
        Spec symmetry8=restore("restoration-symmetry8-jade-shield","JADE","LAVENDER","INDIGO","MINT","MAGENTA");symmetry8.symmetry=8;result.add(symmetry8);
        for(int modifier:new int[]{2,3,6,7}){Spec spec=restore("restoration-spore-modifier-"+modifier,"SPORE");spec.spore=modifier;result.add(spec);}
        Spec dodge=restore("restoration-dodge-guards","GREEN").set("actor:victim:_candodge",false).set("actor:victim:_skyversion",0).set("actor:victim:_isinfectious",false).set("actor:victim:_plagueversion",0);
        for(String flag:new String[]{"_isaconsumer","_isafungus","_isakiller"}){
            Action off=call("segmentsRestoreEffects","~").set("actor:victim:_candodge",false).set("actor:victim:"+flag,true);dodge.add(off);
            dodge.add(call("segmentsRestoreEffects","~").set("actor:victim:_candodge",false).set("actor:victim:"+flag,false));
        }
        dodge.add(call("segmentsRestoreEffects","~").set("actor:victim:_candodge",false).set("actor:victim:_skyversion",1));
        dodge.add(call("segmentsRestoreEffects","~").set("actor:victim:_candodge",false).set("actor:victim:_skyversion",0).set("actor:victim:_isinfectious",true));
        dodge.add(call("segmentsRestoreEffects","~").set("actor:victim:_candodge",true).set("actor:victim:_canreact",false));result.add(dodge);
        Spec bypass=restore("restoration-passive-viral-dodge-bypass","GREEN").set("actor:victim:_candodge",false).set("actor:victim:_isinfectious",true).set("actor:victim:_iscoral",true).set("actor:victim:_plagueversion",1).set("actor:victim:_methanotrophy",0d);bypass.passive=true;
        bypass.add(call("segmentsRestoreEffects","~").set("actor:victim:_candodge",false).set("actor:victim:_methanotrophy",1d));
        bypass.add(call("segmentsRestoreEffects","~").set("actor:victim:_candodge",false).set("actor:victim:_methanotrophy",0d).set("actor:victim:_isaplant",true));result.add(bypass);
        Spec early=restore("restoration-early-minimum-classifications","BLOND").set("actor:victim:_reproduceEnergy",20).set("actor:victim:_reproducelate",0).set("actor:victim:_isaplant",true);
        early.add(call("segmentsRestoreEffects","~").set("actor:victim:_isaplant",false).set("actor:victim:_isaconsumer",true));
        early.add(call("segmentsRestoreEffects","~").set("actor:victim:_isaconsumer",false).set("actor:victim:_isafungus",true));
        early.add(call("segmentsRestoreEffects","~").set("actor:victim:_isafungus",false).set("actor:victim:_transfersenergy",true).set("actor:victim:_isinfectious",true).set("actor:victim:_isblond",true));
        early.add(call("segmentsRestoreEffects","~").set("actor:victim:_transfersenergy",false).set("actor:victim:_isinfectious",false).set("actor:victim:_sporeversion",-2));
        early.add(call("segmentsRestoreEffects","~").set("actor:victim:_sporeversion",6));
        early.add(call("segmentsRestoreEffects","~").set("actor:victim:_sporeversion",0));
        early.add(call("segmentsRestoreEffects","~").set("actor:victim:_transfersenergy",true).set("actor:victim:_isinfectious",true).set("actor:victim:_isblond",false));result.add(early);
        for(String type:new String[]{"BLOND","FLOWER","GOLD"}){
            Spec spec=restore("restoration-large-valid-"+type,type).huge().set("actor:victim:_reproduceEnergy",Integer.MAX_VALUE).set("actor:victim:_reproducelate",0).set("actor:victim:_gold",0);spec.length=214748360d;
            if(type.equals("GOLD"))spec.setting("GOLD_DIVISOR",0.01d);result.add(spec);
        }
        Spec infectiousFlower=restore("restoration-large-infectious-flower","FLOWER").huge().set("actor:victim:_reproduceEnergy",20).set("actor:victim:_reproducelate",0).set("actor:victim:_isinfectious",true);infectiousFlower.length=16777219d;result.add(infectiousFlower);
        for(String type:new String[]{"BLOND","GOLD"}){
            Spec spec=restore("restoration-wrapped-"+type,type,type).huge().set("actor:victim:_gold",0).set("actor:victim:_max_age",777);spec.length=214748360d;result.add(spec);
        }
        Spec reverse=restore("restoration-reverse-flower-overflow","FLOWER","FLOWER","FLOWER").huge().set("actor:victim:_reproducelate",0).set("actor:victim:_reproduceEnergy",1001);reverse.lengths=new double[]{1d,214748360d,214748360d};result.add(reverse);
        for(double divisor:new double[]{2d,Math.nextUp(2d)}){Spec spec=restore("restoration-gold-double-"+bits(divisor),"GOLD").setting("GOLD_DIVISOR",divisor);spec.length=1d;result.add(spec);}
        Spec sequence=operation("survival-then-neutralization","survive","MAGENTA","WHITE");sequence.add(call("neutralized","killer"));result.add(sequence);
        Spec repeated=operation("inhibition-and-repeated-restoration","inhibited","SPORE","AUBURN","INDIGO","YELLOW","BLOND");repeated.add(call("segmentsRestoreEffects","~")).add(call("segmentsRestoreEffects","~"));result.add(repeated);
        Spec frost=operation("freeze-then-restoration","frozen","TEAL","CYAN","JADE","SPRING","MAGENTA","YELLOW");frost.add(call("segmentsRestoreEffects","~"));result.add(frost);
        Spec pure=restore("restoration-pure-auburn-cache-retention","GREEN").set("actor:victim:_isauburn",true).set("actor:victim:_lavender",123).set("actor:victim:_usefriendeffects",2);
        for(String flag:new String[]{"_isgray","_isspike","_isenhanced","_transfersenergy","_usepretoucheffects","_useextraeffects"})pure.set("actor:victim:"+flag,true);result.add(pure);
        Spec eyes=restore("restoration-eye-cache-through-freeze","EYE","GREEN");eyes.add(call("frozen","killer"));result.add(eyes);
        Spec modifiedSky=restore("restoration-modified-sky-cache","SKY","DEEPSKY").set("actor:victim:_skyversion",0);modifiedSky.modifiedSky=true;result.add(modifiedSky);
        return result;
    }
    private static String capture()throws Exception{
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.ContactStateOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        String text=capture();Files.writeString(directory.resolve("contact_state.tsv"),text,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("CONTACT_STATE_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls);
    }
}
