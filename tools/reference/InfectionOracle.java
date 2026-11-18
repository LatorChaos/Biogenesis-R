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

/** Observe original infection assignments, event routing, thresholds, and genome identity */
public final class InfectionOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static int cases,snapshots,calls;

    private static final class Action{
        final String operation,argument;
        final Map<String,Object> controls=new LinkedHashMap<>();
        Action(String operation,String argument){this.operation=operation;this.argument=argument;}
        Action set(String target,Object value){controls.put(target,value);return this;}
    }
    private static final class Spec{
        final String name;
        String killerCode="g1";
        final List<Action> actions=new ArrayList<>();
        Spec(String name){this.name=name;}
        Spec add(Action action){actions.add(action);return this;}
    }
    private static final class Capture{
        final StringBuilder rows=new StringBuilder();
        final Map<String,GeneticCode> genomes=new LinkedHashMap<>();
        final Map<String,Organism> actors=new LinkedHashMap<>();
        final IdentityHashMap<GeneticCode,String> references=new IdentityHashMap<>();
        final World world;
        final boolean diagnostic;
        Capture(Spec spec,boolean diagnostic)throws Exception{
            this.diagnostic=diagnostic;
            for(Field original:Utils.class.getDeclaredFields())if(original.getName().startsWith("DEF_"))Utils.class.getDeclaredField(original.getName().substring(4)).set(null,original.get(null));
            Utils.INITIAL_ORGANISMS=0;Utils.MIN_MUTATION_RATE=0;Utils.MAX_MUTATION_RATE=10000;Utils.META_MUTATION_RATE=0;
            for(String name:COLORS)Utils.class.getDeclaredField(name+"_PROB").setInt(null,name.equals("GREEN")?7:0);
            Utils.random=new Random(827364);
            VisibleWorld visible=new VisibleWorld(null);world=new World(visible);
            world.addCO2(Utils.INITIAL_CO2);world.addO2(Utils.INITIAL_O2);world.addCH4(Utils.INITIAL_CH4);
            genomes.put("g0",genome(3,false,false));
            genomes.put("g1",genome(1,false,true));genomes.put("g2",genome(1,false,true));
            genomes.put("g3",(GeneticCode)genomes.get("g0").clone());genomes.put("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,diagnostic?"callback-diagnostic":"canonical",827364);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                references.put(entry.getValue(),entry.getKey());
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g3")||entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            Organism peer=paste("peer","g2",1000,1000);visible._selectedOrganism=peer;
            row("I","setup","visible:selected","String","peer");world.addOrganism(peer,null);
            Organism victim=paste("victim","g0",3500,3500);world.addOrganism(victim,null);
            Organism killer=paste("killer",spec.killerCode,2000,2000);world.addOrganism(killer,null);
            control("setup","actor:victim:_infectedGeneticCode","g3");control("setup","actor:victim:_savedGeneticCode","g4");control("setup","actor:victim:_infectedReproduceEnergy",314);
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
            if(name.equals("gene0._length")){((GeneticCode)owner).getGene(0).setLength((Double)value);row("I",stage,target,"double",bits((Double)value));return;}
            Field field=owner.getClass().getDeclaredField(name);field.setAccessible(true);
            if(field.getType()==GeneticCode.class){field.set(owner,value.equals("~")?null:genomes.get((String)value));row("I",stage,target,"GeneticCode",value);}
            else{field.set(owner,value);row("I",stage,target,field.getType().getSimpleName(),scalar(value));}
        }
        void action(int index,Action action)throws Exception{
            String stage="step-"+index;
            for(Map.Entry<String,Object> input:action.controls.entrySet())control(stage,input.getKey(),input.getValue());
            Organism victim=actors.get("victim");row("K",stage,action.operation,"victim",action.argument);
            if(diagnostic){
                try{invoke(action,victim,stage);row("X",stage,"none");}catch(NullPointerException error){row("X",stage,error.getClass().getName());}
            }else invoke(action,victim,stage);
            calls++;snapshot(stage);
        }
        void invoke(Action action,Organism victim,String stage)throws Exception{
            Organism donor=action.argument.equals("self")?victim:actors.get(action.argument);
            switch(action.operation){
                case "infectedByCode":victim.infectedBy(action.argument.equals("~")?(GeneticCode)null:genomes.get(action.argument));break;
                case "infectedBy":victim.infectedBy(donor);break;
                case "infectedByFruit":victim.infectedByFruit(donor);break;
                case "infectedByFruitWeak":victim.infectedByFruitWeak(donor);break;
                default:throw new IllegalArgumentException("Unknown actual operation");
            }
        }
        String finish(){row("E");cases++;return rows.toString();}
    }
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static Random copyRandom(Random random)throws Exception{ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static GeneticCode genome(int count,boolean zero,boolean ordinary)throws Exception{
        List<Gene> genes=new ArrayList<>();for(int i=0;i<count;i++){
            Gene gene=new Gene();gene.setLength(zero?0:12);gene.setTheta(0);gene.setBranch(0);gene.setColor(!ordinary&&count==3?(i==0?Color.GREEN:i==1?Color.CYAN:Utils.ColorEYE):Color.GREEN);genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,1,0,0,0,1,1,1,1,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false);code._generation=41;code._cladeID="root";return code;
    }
    private static Action call(String name,String donor){return new Action(name,donor);}
    private static List<Spec> canonical(){
        List<Spec> result=new ArrayList<>();
        result.add(new Spec("code-null-still-notifies").add(call("infectedByCode","~")).add(call("infectedByCode","~")));
        result.add(new Spec("code-primary-identity").add(call("infectedByCode","g0")));
        result.add(new Spec("code-equal-clone-identity").add(call("infectedByCode","g3")));
        result.add(new Spec("code-repeated-identity-sequence").add(call("infectedByCode","g0")).add(call("infectedByCode","g3")).add(call("infectedByCode","g0")));
        result.add(new Spec("ordinary-donor-copy").add(call("infectedBy","killer").set("actor:killer:_reproduceEnergy",101)));
        result.add(new Spec("ordinary-donor-negative-copy").add(call("infectedBy","killer").set("actor:killer:_reproduceEnergy",Integer.MIN_VALUE)));
        result.add(new Spec("ordinary-donor-max-copy").add(call("infectedBy","killer").set("actor:killer:_reproduceEnergy",Integer.MAX_VALUE)));
        result.add(new Spec("ordinary-self-donor").add(call("infectedBy","self").set("actor:victim:_reproduceEnergy",101)));
        result.add(new Spec("ordinary-repeated-no-deduplication").add(call("infectedBy","killer")).add(call("infectedBy","killer").set("actor:victim:_savedGeneticCode","g4")).add(call("infectedBy","killer")));
        Spec shared=new Spec("ordinary-shared-primary-code");shared.killerCode="g0";shared.add(call("infectedBy","killer"));result.add(shared);
        Spec equal=new Spec("ordinary-equal-distinct-code");equal.killerCode="g3";equal.add(call("infectedBy","killer"));result.add(equal);
        result.add(new Spec("donor-counter-wrapping").add(call("infectedBy","killer").set("actor:killer:_nTotalInfected",Integer.MAX_VALUE-1)).add(call("infectedBy","killer")).add(call("infectedBy","killer")));
        result.add(new Spec("self-counter-wrapping").add(call("infectedBy","self").set("actor:victim:_nTotalInfected",Integer.MAX_VALUE)));
        result.add(new Spec("donor-record-tie-and-replacement").add(call("infectedBy","killer").set("actor:killer:_nTotalInfected",2).set("statistics:beingMostInfections","g4").set("statistics:beingMostInfectionsNumber",3).set("statistics:beingMostInfectionsTime",7L).set("statistics:time",8L)).add(call("infectedBy","killer")));
        result.add(new Spec("infection-sum-wrapping").add(call("infectedByCode","~").set("statistics:infectionsSum",Long.MAX_VALUE)).add(call("infectedBy","killer")));
        for(String operation:new String[]{"infectedByFruit","infectedByFruitWeak"}){
            result.add(new Spec(operation+"-ordinary").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",101).set("actor:killer:_sporetime",3)));
            result.add(new Spec(operation+"-self").add(call(operation,"self").set("actor:victim:_reproduceEnergy",101).set("actor:victim:_sporetime",3)));
            result.add(new Spec(operation+"-repeated").add(call(operation,"killer").set("actor:killer:_sporetime",3)).add(call(operation,"killer")).add(call(operation,"killer")));
            result.add(new Spec(operation+"-odd-threshold").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",101).set("actor:killer:_sporetime",1)));
            result.add(new Spec(operation+"-negative-threshold").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",-101).set("actor:killer:_sporetime",3)));
            result.add(new Spec(operation+"-zero-spore-time").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",101).set("actor:killer:_sporetime",0)));
            result.add(new Spec(operation+"-negative-spore-time").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",101).set("actor:killer:_sporetime",-3)));
            result.add(new Spec(operation+"-max-spore-time").add(call(operation,"killer").set("actor:killer:_sporetime",Integer.MAX_VALUE)));
            result.add(new Spec(operation+"-min-spore-time").add(call(operation,"killer").set("actor:killer:_sporetime",Integer.MIN_VALUE)));
            result.add(new Spec(operation+"-max-reproduction-threshold").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",Integer.MAX_VALUE).set("actor:killer:_sporetime",1)));
            result.add(new Spec(operation+"-min-reproduction-threshold").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",Integer.MIN_VALUE).set("actor:killer:_sporetime",1)));
            result.add(new Spec(operation+"-negative-product-saturates-positive").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",Integer.MIN_VALUE).set("actor:killer:_sporetime",Integer.MIN_VALUE)));
            result.add(new Spec(operation+"-positive-reproduction-late").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",1001).set("actor:victim:_reproducelate",1).set("actor:killer:_sporetime",3)));
            result.add(new Spec(operation+"-negative-reproduction-late").add(call(operation,"killer").set("actor:victim:_reproduceEnergy",-1001).set("actor:victim:_reproducelate",-1).set("actor:killer:_sporetime",3)));
        }
        result.add(new Spec("live-distinct-genotype-mutation").add(call("infectedBy","killer")).add(call("infectedByCode","~").set("genome:g0:_cladeID","source:recipient-after-notification").set("genome:g0:_generation",Integer.MIN_VALUE).set("genome:g0:gene0._length",25.5d).set("genome:g1:_cladeID","source:donor-after-notification").set("genome:g1:_generation",Integer.MAX_VALUE).set("genome:g1:gene0._length",31.25d)));
        Spec sharedEdit=new Spec("live-shared-genotype-mutation");sharedEdit.killerCode="g0";sharedEdit.add(call("infectedBy","killer")).add(call("infectedByCode","g0").set("genome:g0:_cladeID","source:shared-after-notification").set("genome:g0:gene0._length",25.5d));result.add(sharedEdit);
        Spec equalEdit=new Spec("live-equal-clone-mutation");equalEdit.killerCode="g3";equalEdit.add(call("infectedBy","killer")).add(call("infectedByCode","g3").set("genome:g3:_cladeID","source:clone-after-notification").set("genome:g3:gene0._length",25.5d));result.add(equalEdit);
        return result;
    }
    private static String capture(boolean diagnostic)throws Exception{
        List<Spec> specs;
        if(diagnostic){specs=new ArrayList<>();for(String operation:new String[]{"infectedBy","infectedByFruit","infectedByFruitWeak"})specs.add(new Spec("selected-donor-callback-"+operation).add(call(operation,"peer").set("actor:peer:_reproduceEnergy",101).set("actor:peer:_sporetime",3)));}
        else specs=canonical();
        StringBuilder output=new StringBuilder();for(Spec spec:specs){Capture capture=new Capture(spec,diagnostic);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length<1||args.length>2||(args.length==2&&!args[1].equals("callback-diagnostic")))throw new IllegalArgumentException("Usage: biogenesis.InfectionOracle NEW_OUTPUT_DIRECTORY [callback-diagnostic]");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        boolean diagnostic=args.length==2;String text=capture(diagnostic);Files.writeString(directory.resolve(diagnostic?"infection_callback.tsv":"infection.tsv"),text,StandardCharsets.UTF_8);
        System.out.println("INFECTION_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls+(diagnostic?" mode=callback-diagnostic":""));
    }
}
