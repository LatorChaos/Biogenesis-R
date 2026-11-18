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

/** Observe original death and boundary effects on real world-owned actors */
public final class DeathOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static Method inside,grow,offset;
    private static int cases,snapshots,calls;

    private static final class Action{
        final String operation,argument;
        final Map<String,Object> controls=new LinkedHashMap<>();
        Action(String operation,String argument){this.operation=operation;this.argument=argument;}
        Action set(String target,Object value){controls.put(target,value);return this;}
    }
    private static final class Spec{
        final String name;
        boolean shared,allColors,zero,rawAllocation;
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
            genomes.put("g0",genome(spec.zero?1:spec.allColors?66:3,spec.zero,false));
            genomes.put("g1",genome(1,false,true));genomes.put("g2",genome(1,false,true));
            genomes.put("g3",(GeneticCode)genomes.get("g0").clone());genomes.put("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,diagnostic?"raw-invalid":"canonical",827364);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                references.put(entry.getValue(),entry.getKey());
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g3")||entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            Organism selector=paste("selector","g2",1000,1000);visible._selectedOrganism=selector;
            row("I","setup","visible:selected","String","selector");world.addOrganism(selector,null);
            Organism victim=paste("victim","g0",3500,3500);world.addOrganism(victim,null);
            Organism killer=paste("killer",spec.shared?"g0":"g1",2000,2000);world.addOrganism(killer,null);
            if(spec.rawAllocation){
                Organism raw=new Organism(world);actors.put("victim",raw);row("F","replace_victim_with_unregistered_raw_allocation","~");
            }
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
            switch(path[0]){case "actor":owner=actors.get(path[1]);name=path[2];break;case "world":owner=world;name=path[1];break;case "statistics":owner=world.worldStatistics;name=path[1];break;case "genome":owner=genomes.get(path[1]);name=path[2];break;default:throw new IllegalArgumentException("Unknown control target");}
            if(name.equals("bounds")){((Organism)owner).setBounds((Rectangle)value);row("I",stage,target,"Rectangle",scalar(value));return;}
            Field field=owner.getClass().getDeclaredField(name);field.setAccessible(true);
            if(field.getType()==GeneticCode.class){field.set(owner,genomes.get((String)value));row("I",stage,target,"GeneticCode",value);}
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
            switch(action.operation){
                case "die":victim.die(action.argument.equals("~")?null:action.argument.equals("self")?victim:actors.get(action.argument));break;
                case "isInsideWorld":row("B",stage,"inside",inside.invoke(victim));break;
                case "grow":grow.invoke(victim);break;
                case "offset_velocity":offset.invoke(victim,victim.dx,victim.dy,victim.dtheta);break;
                case "bounds":victim.calculateBounds(Boolean.parseBoolean(action.argument));break;
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
    private static GeneticCode genome(int count,boolean zero,boolean ordinary)throws Exception{
        List<Gene> genes=new ArrayList<>();for(int i=0;i<count;i++){
            Gene gene=new Gene();gene.setLength(zero?0:12);gene.setTheta(0);gene.setBranch(0);gene.setColor(!ordinary&&count==3?(i==0?Color.GREEN:i==1?Color.CYAN:Utils.ColorEYE):Color.GREEN);genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,1,0,0,0,1,1,1,1,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false);code._generation=41;code._cladeID="root";return code;
    }
    private static Color[] currentColors()throws Exception{
        List<Color> colors=new ArrayList<>();for(String name:COLORS)colors.add(color(name));for(String name:INTERNAL)colors.add(color(name));
        colors.add(new Color(0x12345678,true));colors.add(new Color((Utils.ColorEYE.getRGB()&0x00ffffff)|0x12000000,true));return colors.toArray(Color[]::new);
    }
    private static Action die(String killer){return new Action("die",killer);}
    private static Action boundary(Rectangle bounds){return new Action("isInsideWorld","~").set("actor:victim:bounds",bounds).set("actor:victim:dx",1.25d).set("actor:victim:dy",-0.75d).set("actor:victim:dtheta",0.125d);}
    private static List<Spec> canonical()throws Exception{
        List<Spec> result=new ArrayList<>();
        for(String killer:new String[]{"~","killer","self"}){
            result.add(new Spec("death-"+killer.replace('~','n')).add(die(killer)));
            result.add(new Spec("repeated-death-"+killer.replace('~','n')).add(die(killer)).add(die(killer)).add(die(killer)));
            Spec colors=new Spec("all-current-colors-"+killer.replace('~','n'));colors.allColors=true;colors.add(die(killer).set("actor:victim:_segColor",currentColors()));result.add(colors);
        }
        Spec populated=new Spec("populated-state-retention");Action state=die("killer");
        for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus","_isakiller","_isgray","_isspike","_isfrozen","_allfrozen","_isinjured","_isreproductive","_isinfectious","_iscoral","_isauburn","_isblond","_issilver","_isenhanced","_islime","_isonlyc4","_transfersenergy","_crowded","_candodge","_dodge","_hasdodged","_useextraeffects","_usepretoucheffects","_useframemovement","_canmove","_canreact"})state.set("actor:victim:"+flag,true);
        for(String field:new String[]{"_indigo","_antiviral","_lavender","_createlavender","_healing","_sporetime","_spin","_gold","_jadefactor","_lengthfriend","_thetafriend","_lowersymmetric","_nChildren","_nVirusChildren","_nTotalChildren","_nTotalInfected","_yellowCounter"})state.set("actor:victim:"+field,17);
        state.set("actor:victim:_sporeversion",-3).set("actor:victim:_skyversion",2).set("actor:victim:_fallowversion",4).set("actor:victim:_plagueversion",2).set("actor:victim:_summerinactivity",1).set("actor:victim:_reproducelate",-4);
        state.set("actor:victim:_usefriendeffects",2).set("actor:victim:_segmentpretoucheffects",2).set("actor:victim:_updateEffects",1).set("actor:victim:_nTotalKills",11).set("actor:victim:hasMoved",false);
        state.set("actor:victim:_energy",123.25d).set("actor:victim:dx",1.25d).set("actor:victim:dy",-0.75d).set("actor:victim:dtheta",0.125d);
        state.set("actor:victim:_photosynthesis",123.5d).set("actor:victim:_methanotrophy",67.25d).set("actor:victim:_colonyPhotosynthesis",19.75d).set("actor:victim:_forestphoto",1.875d).set("actor:victim:_leafphoto",2.625d).set("actor:victim:_maintenance",9.125d);
        state.set("actor:victim:_age",1023).set("actor:victim:_max_age",77).set("actor:victim:_reproduceEnergy",101).set("actor:victim:_earlyReproduceEnergy",31).set("actor:victim:_infectedReproduceEnergy",211).set("actor:victim:_timeToReproduce",4).set("actor:victim:_timeToReproduceMax",15);
        state.set("actor:victim:_nPosition","0123XXXX").set("actor:victim:_color",Color.MAGENTA).set("actor:victim:_framesColor",9).set("actor:victim:_infectedGeneticCode","g3").set("actor:victim:_savedGeneticCode","g4").set("statistics:time",7L);
        populated.add(state);result.add(populated);
        Spec shared=new Spec("shared-genotype-statistics-alias");shared.shared=true;shared.add(die("killer").set("actor:victim:_infectedGeneticCode","g0").set("actor:victim:_savedGeneticCode","g0"));
        shared.add(new Action("isInsideWorld","~").set("genome:g0:_cladeID","source:after-death"));result.add(shared);
        result.add(new Spec("killer-counter-wrapping").add(die("killer").set("actor:killer:_nTotalKills",Integer.MAX_VALUE-1)).add(die("killer")));
        result.add(new Spec("self-killer-counter-wrapping").add(die("self").set("actor:victim:_nTotalKills",Integer.MAX_VALUE)));
        result.add(new Spec("population-counter-wrapping").add(die("~").set("world:_population",Integer.MIN_VALUE)));
        result.add(new Spec("inactive-placed-death").add(die("~").set("actor:victim:active",false)));
        Rectangle[] bounds={new Rectangle(100,100,2,2),new Rectangle(0,100,2,2),new Rectangle(100,0,2,2),new Rectangle(6998,100,2,2),new Rectangle(100,6998,2,2),new Rectangle(6998,6998,2,2),new Rectangle(-1,-1,2,2),
                new Rectangle(0,0,2,2),new Rectangle(0,0,2,1),new Rectangle(0,0,1,2),new Rectangle(0,0,1,1),new Rectangle(0,0,0,0),new Rectangle(0,0,-2,-3),new Rectangle(100,100,-2,-3),
                new Rectangle(Integer.MAX_VALUE-2,100,8,2),new Rectangle(100,Integer.MAX_VALUE-2,2,8),new Rectangle(Integer.MAX_VALUE-2,Integer.MAX_VALUE-2,8,8)};
        for(int i=0;i<bounds.length;i++)result.add(new Spec("boundary-"+i).add(boundary(bounds[i])));
        result.add(new Spec("boundary-already-dead-origin").add(die("~")).add(boundary(new Rectangle(0,0,1,1))));
        result.add(new Spec("boundary-signed-zero").add(boundary(new Rectangle(0,0,2,2)).set("actor:victim:dx",-0d).set("actor:victim:dy",0d).set("actor:victim:dtheta",-0d)));
        result.add(new Spec("boundary-nan-velocity").add(boundary(new Rectangle(0,0,2,2)).set("actor:victim:dx",Double.longBitsToDouble(0xfff8000000000042L)).set("actor:victim:dy",Double.longBitsToDouble(0x7ff8000000000042L)).set("actor:victim:dtheta",Double.NaN)));
        Spec zero=new Spec("real-zero-mass-origin-death");zero.zero=true;
        zero.add(new Action("grow","~").set("actor:victim:_age",7).set("actor:victim:_energy",0d).set("actor:victim:active",true).set("actor:victim:dx",1.25d).set("actor:victim:dy",-0.75d).set("actor:victim:dtheta",0.125d));
        zero.add(new Action("offset_velocity","~")).add(new Action("bounds","true")).add(new Action("isInsideWorld","~"));result.add(zero);
        return result;
    }
    private static String capture(boolean diagnostic)throws Exception{
        List<Spec> specs;
        if(diagnostic){
            Spec nullColor=new Spec("null-current-color-exception");nullColor.add(die("killer").set("actor:victim:_segColor",new Color[]{null,Color.CYAN,Utils.ColorEYE}));
            Spec raw=new Spec("unregistered-uninitialized-origin");raw.rawAllocation=true;raw.add(new Action("isInsideWorld","~"));specs=List.of(nullColor,raw);
        }else specs=canonical();
        StringBuilder output=new StringBuilder();for(Spec spec:specs){Capture capture=new Capture(spec,diagnostic);for(int i=0;i<spec.actions.size();i++)capture.action(i,spec.actions.get(i));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception{
        if(args.length<1||args.length>2||(args.length==2&&!args[1].equals("raw-invalid")))throw new IllegalArgumentException("Usage: biogenesis.DeathOracle NEW_OUTPUT_DIRECTORY [raw-invalid]");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        inside=Organism.class.getDeclaredMethod("isInsideWorld");inside.setAccessible(true);grow=Organism.class.getDeclaredMethod("grow");grow.setAccessible(true);
        offset=Organism.class.getDeclaredMethod("offset",double.class,double.class,double.class);offset.setAccessible(true);
        boolean diagnostic=args.length==2;String text=capture(diagnostic);Files.writeString(directory.resolve(diagnostic?"death_invalid.tsv":"death.tsv"),text,StandardCharsets.UTF_8);
        System.out.println("DEATH_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls+(diagnostic?" mode=raw-invalid":""));
    }
}
