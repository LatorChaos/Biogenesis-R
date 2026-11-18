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
import org.locationtech.jts.geom.Envelope;
import org.locationtech.jts.index.strtree.STRtree;

/** Observe original contact and checkHit with real geometry, state, and identity */
public final class ContactOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static int cases,snapshots,calls,newborns;
    private static final class Code {
        String[] colors={"GREEN"};
        int symmetry=1,mirror,rate,spore=1,adapt=1,generation=41;
        double length=12;
        boolean disperse,selfish,plague,generationBattle,siblingBattle,social,peaceful;
        String clade="root";
        Code colors(String...value){colors=value;return this;}
    }
    private static final class Action {
        final String operation,argument;
        final Map<String,Object> controls=new LinkedHashMap<>();
        Action(String operation,String argument){this.operation=operation;this.argument=argument;}
        Action set(String target,Object value){controls.put(target,value);return this;}
    }
    private static final class Spec {
        final String name;
        final Code parent=new Code(),recipient=new Code(),infected=new Code().colors("WHITE");
        final Map<String,Object> settings=new LinkedHashMap<>(),controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        long seed=827364;
        Spec(String name,String weapon,String target)throws Exception {
            this.name=name;parent.colors(weapon);recipient.colors(target);
            for(String role:new String[]{"victim","killer"}) {
                controls.put("actor:"+role+":_energy",100d);
                controls.put("actor:"+role+":active",true);controls.put("actor:"+role+":alive",true);
                controls.put("actor:"+role+":_framesColor",0);controls.put("actor:"+role+":_parentID",-1);
                for(String flag:new String[]{"_isaplant","_isaconsumer","_isafungus","_isenhanced","_isinfectious","_iscoral","_isgray","_isinjured","_isfrozen","_allfrozen","_altruist","_transfersenergy","_canreact","_canmove","_isspike","_usepretoucheffects","_generationbattle","_siblingbattle"})controls.put("actor:"+role+":"+flag,false);
                for(String field:new String[]{"_plagueversion","_fallowversion","_sporeversion","_antiviral","_lavender","_createlavender","_segmentpretoucheffects","_usefriendeffects"})controls.put("actor:"+role+":"+field,0);
            }
            settings.put("TEAL_ENERGY_CONSUMPTION",1d);settings.put("RUBBING",1d);
            actions.add(tree());actions.add(new Action("contact","killer"));
        }
        Spec set(String target,Object value){controls.put(target,value);return this;}
        Spec setting(String target,Object value){settings.put(target,value);return this;}
        Spec add(Action value){actions.add(value);return this;}
    }
    private static final class Capture {
        final StringBuilder rows=new StringBuilder();
        final Map<String,GeneticCode> genomes=new LinkedHashMap<>();
        final Map<String,Organism> actors=new LinkedHashMap<>();
        final IdentityHashMap<GeneticCode,String> references=new IdentityHashMap<>();
        final IdentityHashMap<Organism,String> actorNames=new IdentityHashMap<>();
        final World world;
        final Spec spec;
        int discovered;
        Capture(Spec spec)throws Exception {
            this.spec=spec;
            for(Field field:Utils.class.getDeclaredFields())if(field.getName().startsWith("DEF_"))Utils.class.getDeclaredField(field.getName().substring(4)).set(null,field.get(null));
            Utils.INITIAL_ORGANISMS=0;Utils.INITIAL_O2=1000;Utils.MIN_MUTATION_RATE=0;Utils.MAX_MUTATION_RATE=10000;Utils.META_MUTATION_RATE=0;
            for(String name:COLORS)Utils.class.getDeclaredField(name+"_PROB").setInt(null,name.equals("GREEN")?7:0);
            for(Map.Entry<String,Object> entry:spec.settings.entrySet())Utils.class.getDeclaredField(entry.getKey()).set(null,entry.getValue());
            Utils.random=new Random(spec.seed);
            VisibleWorld visible=new VisibleWorld(null);world=new World(visible);
            world.addCO2(Utils.INITIAL_CO2);world.addO2(Utils.INITIAL_O2);world.addCH4(Utils.INITIAL_CH4);
            register("g0",genome(spec.parent));register("g1",genome(spec.recipient));register("g2",genome(new Code().colors("GREEN")));
            register("g3",genome(spec.infected));register("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,"contact",spec.seed);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            Organism blocker=paste("selector","g2",1000,1000);visible._selectedOrganism=blocker;
            row("I","setup","visible:selected","String","selector");world.addOrganism(blocker,null);
            Organism parent=paste("victim","g0",3500,3500);world.addOrganism(parent,null);
            Organism peer=paste("killer","g1",5000,5000);world.addOrganism(peer,null);
            geometry("victim",parent,false);geometry("killer",peer,true);
            for(Map.Entry<String,Object> entry:spec.controls.entrySet())control("setup",entry.getKey(),entry.getValue());
            snapshot("initial");
        }
        void geometry(String role,Organism actor,boolean vertical)throws Exception {
            control("setup","actor:"+role+":_centerX",3500);control("setup","actor:"+role+":_centerY",3500);
            control("setup","actor:"+role+":_dCenterX",3500d);control("setup","actor:"+role+":_dCenterY",3500d);
            control("setup","actor:"+role+":bounds",new Rectangle(3494,3494,13,13));
            control("setup","actor:"+role+":x1",integers(actor._segments,vertical?0:-5));control("setup","actor:"+role+":x2",integers(actor._segments,vertical?0:5));
            control("setup","actor:"+role+":y1",integers(actor._segments,vertical?-5:0));control("setup","actor:"+role+":y2",integers(actor._segments,vertical?5:0));
            control("setup","actor:"+role+":_m",doubles(actor._segments,2d));control("setup","actor:"+role+":_mphoto",doubles(actor._segments,0d));
            control("setup","actor:"+role+":_mass",1d);control("setup","actor:"+role+":_I",1d);
            control("setup","actor:"+role+":dx",0d);control("setup","actor:"+role+":dy",0d);control("setup","actor:"+role+":dtheta",0d);control("setup","actor:"+role+":hasGrown",0);
            control("setup","actor:"+role+":_haseyes",false);
        }
        void register(String label,GeneticCode code){require(code!=null&&!references.containsKey(code),"Duplicate genome recipe identity");genomes.put(label,code);references.put(code,label);}
        String reference(GeneticCode code){if(code==null)return "~";String label=references.get(code);if(label==null){label="g"+genomes.size();register(label,code);}return label;}
        void discover(String stage)throws Exception {
            for(Organism actor:world._organisms)if(!actorNames.containsKey(actor)){
                String label="newborn"+discovered++;newborns++;actors.put(label,actor);actorNames.put(actor,label);row("D",stage,label,actor.getID(),reference(actor._geneticCode));
            }
            for(Organism actor:actors.values()){reference(actor._geneticCode);reference(actor._infectedGeneticCode);reference(actor._savedGeneticCode);}
            for(Field field:fields(WorldStatistics.class))if(field.getType()==GeneticCode.class)reference((GeneticCode)field.get(world.worldStatistics));
        }
        Organism paste(String role,String code,int x,int y)throws Exception {
            Organism actor=new Organism(world,genomes.get(code));require(actor.pasteOrganism(x,y),"Real actor paste failed: "+role);
            actors.put(role,actor);actorNames.put(actor,role);row("A",role,code,x,y);return actor;
        }
        void row(Object...values){for(int i=0;i<values.length;i++){if(i!=0)rows.append('\t');rows.append(values[i]);}rows.append('\n');}
        String scalar(Object value){
            if(value==null)return "~";
            if(value instanceof Double number)return bits(number);
            if(value instanceof Color color)return String.format(java.util.Locale.ROOT,"%08x",color.getRGB());
            if(value instanceof GeneticCode code)return reference(code);
            if(value instanceof Organism actor){String label=actorNames.get(actor);require(label!=null,"Unknown actor reference");return label;}
            if(value instanceof Rectangle r)return r.x+","+r.y+","+r.width+","+r.height;
            if(value instanceof World||value instanceof VisibleWorld||value instanceof WorldStatistics)return "present";
            if(value instanceof STRtree tree)return "present,"+tree.size();
            if(value instanceof List<?> list){StringBuilder text=new StringBuilder();for(Object item:list){if(text.length()!=0)text.append(',');text.append(scalar(item));}return text.toString();}
            if(value.getClass().isArray()){StringBuilder text=new StringBuilder();for(int i=0;i<Array.getLength(value);i++){if(i!=0)text.append(',');text.append(scalar(Array.get(value,i)));}return text.toString();}
            return value.toString();
        }
        void genomeState(String stage,String name,GeneticCode code)throws Exception{
            row("G",stage,name,"_reference",references.get(code));row("G",stage,name,"_nGenes",code.getNGenes());
            for(Field field:fields(GeneticCode.class))if(!field.getName().equals("_genes"))row("G",stage,name,field.getName(),scalar(field.get(code)));
            for(int i=0;i<code.getNGenes();i++)for(Field field:fields(Gene.class))row("N",stage,name,i,field.getName(),scalar(field.get(code.getGene(i))));
        }

        void snapshot(String stage)throws Exception {
            discover(stage);row("P",stage);
            IdentityHashMap<GeneticCode,Boolean> available=new IdentityHashMap<>();
            for(int i=0;i<5;i++)available.put(genomes.get("g"+i),true);
            for(Map.Entry<String,Organism> entry:actors.entrySet()){
                Organism actor=entry.getValue();
                available.put(actor._geneticCode,true);available.put(actor._infectedGeneticCode,true);available.put(actor._savedGeneticCode,true);
                for(Field field:fields(Organism.class)){
                    Object value=field.get(actor);row("O",stage,entry.getKey(),field.getName(),field.getType().getSimpleName(),value instanceof GeneticCode?"present":scalar(value));
                }
                row("O",stage,entry.getKey(),"bounds","Rectangle",scalar(actor.getBounds()));
                row("H",stage,entry.getKey(),"genotype",reference(actor._geneticCode));row("H",stage,entry.getKey(),"infected",reference(actor._infectedGeneticCode));row("H",stage,entry.getKey(),"saved",reference(actor._savedGeneticCode));
            }
            for(Organism actor:world._organisms){available.put(actor._geneticCode,true);available.put(actor._infectedGeneticCode,true);available.put(actor._savedGeneticCode,true);}
            row("W",stage,"_population","int",world.getPopulation());row("W",stage,"organism_count","int",world._organisms.size());row("W",stage,"corpses","int",world.getNCorpses());
            row("W",stage,"_O2","double",bits(world.getO2()));row("W",stage,"_CO2","double",bits(world.getCO2()));row("W",stage,"_CH4","double",bits(world.getCH4()));
            row("W",stage,"NEXT_ID","int",world.NEXT_ID);row("W",stage,"nFrames","long",world.getFrame());row("W",stage,"time","long",world.getTime());
            StringBuilder ids=new StringBuilder();for(Organism actor:world._organisms){if(ids.length()!=0)ids.append(',');ids.append(actor.getID());}row("W",stage,"organism_ids","int[]",ids);
            Field[] worldFields=fields(World.class);require(worldFields.length==15,"Unexpected World instance schema");
            for(Field field:worldFields)row("V",stage,field.getName(),field.getType().getSimpleName(),scalar(field.get(world)));
            for(Map.Entry<String,Organism> entry:actors.entrySet()){Organism actor=entry.getValue();row("Q",stage,entry.getKey(),scalar(world.colDetTree.query(new Envelope(actor.getX(),actor.getMaxX(),actor.getY(),actor.getMaxY()))));}
            Field[] statsFields=fields(WorldStatistics.class);require(statsFields.length==52,"Unexpected statistics instance schema");
            for(Field field:statsFields){Object value=field.get(world.worldStatistics);if(value instanceof GeneticCode code)available.put(code,true);row("S",stage,field.getName(),field.getType().getSimpleName(),scalar(value));}
            Method[] getters=Arrays.stream(WorldStatistics.class.getDeclaredMethods()).filter(method->Modifier.isPublic(method.getModifiers())&&method.getParameterCount()==0&&method.getName().startsWith("get")).sorted(Comparator.comparing(Method::getName)).toArray(Method[]::new);
            require(getters.length==49,"Unexpected statistics getter schema");for(Method method:getters)row("T",stage,method.getName(),method.getReturnType().getSimpleName(),scalar(method.invoke(world.worldStatistics)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                row("Y",stage,entry.getKey(),available.containsKey(entry.getValue())?"observable":"rejected-only");genomeState(stage,entry.getKey(),entry.getValue());
            }
            Random peek=copyRandom(Utils.random);row("R",stage,peek.nextLong(),peek.nextInt(10000),bits(peek.nextDouble()));snapshots++;
        }
        void control(String stage,String target,Object value)throws Exception {
            String[] path=target.split(":",3);Object owner;String name;
            if(path[0].equals("rng")){require(path[1].equals("seed"),"Unknown RNG control");Utils.random=new Random((Long)value);row("I",stage,target,"long",value);return;}
            switch(path[0]){
                case "actor":owner=actors.get(path[1]);name=path[2];break;case "world":owner=world;name=path[1];break;
                case "statistics":owner=world.worldStatistics;name=path[1];break;case "genome":owner=genomes.get(path[1]);name=path[2];break;
                default:throw new IllegalArgumentException("Unknown explicit input target: "+target);
            }
            if(name.equals("bounds")){((Organism)owner).setBounds((Rectangle)value);row("I",stage,target,"Rectangle",scalar(value));return;}
            if(name.startsWith("gene")){
                int dot=name.indexOf('.');int index=Integer.parseInt(name.substring(4,dot));Gene gene=((GeneticCode)owner).getGene(index);String property=name.substring(dot+1);
                if(property.equals("_length")){gene.setLength((Double)value);row("I",stage,target,"double",scalar(value));}
                else if(property.equals("_color")){gene.setColor((Color)value);row("I",stage,target,"Color",scalar(value));}
                else throw new IllegalArgumentException("Unknown original Gene setter");return;
            }
            Field field=owner.getClass().getDeclaredField(name);field.setAccessible(true);
            if(owner instanceof Organism actor&&field.getType().isArray())require(Array.getLength(value)==actor._segments,"Explicit array input length must match actual segments");
            if(field.getType()==GeneticCode.class){field.set(owner,value.equals("~")?null:genomes.get((String)value));row("I",stage,target,"GeneticCode",value);}
            else {field.set(owner,value);row("I",stage,target,field.getType().getSimpleName(),scalar(value));}
        }
        void action(int index,Action action)throws Exception {
            String stage="step-"+index;
            for(Map.Entry<String,Object> entry:action.controls.entrySet())control(stage,entry.getKey(),entry.getValue());
            String argument=action.argument;
            if(action.operation.equals("build_tree")&&argument.equals("members")){
                StringBuilder names=new StringBuilder();for(Organism member:world._organisms){if(names.length()!=0)names.append(',');names.append(actorNames.get(member));}argument=names.toString();
            }
            Organism parent=actors.get("victim");row("K",stage,action.operation,"victim",argument);
            switch(action.operation){
                case "contact":{
                    Organism peer=actors.get(argument);require(peer!=null&&peer!=parent,"Distinct actual contact peer required");
                    row("B",stage,"contact",parent.contact(peer));break;
                }
                case "checkHit":{
                    require(argument.equals("~"),"World checkHit has no peer argument");Organism result=world.checkHit(parent);
                    require(result==null||result==parent,"Original checkHit must return its held actor");row("B",stage,"checkHit",scalar(result));break;
                }
                case "build_tree":{
                    world.colDetTree=new STRtree();if(!argument.equals("~"))for(String name:argument.split(",")){
                        Organism actor=actors.get(name);require(actor!=null,"Unknown actual member identity");world.colDetTree.insert(new Envelope(actor.getX(),actor.getMaxX(),actor.getY(),actor.getMaxY()),actor);
                        row("J",stage,name,actor.x,actor.y,actor.width,actor.height);
                    }world.colDetTree.build();break;
                }
                default:throw new IllegalArgumentException("Unknown original operation: "+action.operation);
            }
            calls++;snapshot(stage);
        }
        String finish(){cases++;row("E");return rows.toString();}
    }
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Random copyRandom(Random random)throws Exception {ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception {
        switch(name){case "UNKNOWN":return new Color(0x12345678,true);case "TRANSLUCENT":return new Color(0x12ff00ff,true);case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static GeneticCode genome(Code recipe)throws Exception {
        List<Gene> genes=new ArrayList<>();for(int index=0;index<recipe.colors.length;index++){
            Gene gene=new Gene();gene.setLength(recipe.length);gene.setTheta(index*0.25);gene.setBranch(0);gene.setColor(color(recipe.colors[index]));genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,recipe.symmetry,recipe.mirror,recipe.rate,0,1,1,recipe.spore,recipe.adapt,recipe.plague,recipe.disperse,recipe.generationBattle,recipe.siblingBattle,false,false,recipe.social,recipe.peaceful,false,false,false,false,false,false,recipe.selfish);
        code._generation=recipe.generation;code._cladeID=recipe.clade;return code;
    }
    private static Action tree(){return new Action("build_tree","members");}
    private static Color[] phenotype(String...names)throws Exception{Color[] colors=new Color[names.length];for(int i=0;i<names.length;i++)colors[i]=color(names[i]);return colors;}
    private static Spec scenario(String name,String weapon,String target)throws Exception{return new Spec(name,weapon,target);}
    private static int[] integers(int count,int value){int[] values=new int[count];Arrays.fill(values,value);return values;}
    private static double[] doubles(int count,double value){double[] values=new double[count];Arrays.fill(values,value);return values;}
    private static Spec eye(String name,boolean moving)throws Exception {
        Spec spec=scenario(name,"TEAL","GREEN");spec.parent.colors("TEAL","EYE");spec.recipient.colors("GREEN","RED");
        return spec.set("actor:victim:_haseyes",true).set("actor:victim:_m",new double[]{0.5d,0.5d})
            .set("actor:victim:_canmove",moving).set("actor:victim:_candodge",true)
            .set("actor:victim:_seggreenReaction",new int[]{0,1}).set("actor:victim:_segredReaction",new int[]{0,2})
            .set("actor:victim:dx",1d).set("rng:seed",1L);
    }
    private static Spec worldCase(Spec spec,String treeMembers)throws Exception {
        spec.actions.clear();spec.actions.add(new Action("build_tree",treeMembers));spec.actions.add(new Action("checkHit","~"));return spec;
    }
    private static Spec selectorGeometry(Spec spec)throws Exception {
        return spec.set("actor:selector:_centerX",3500).set("actor:selector:_centerY",3500)
            .set("actor:selector:_dCenterX",3500d).set("actor:selector:_dCenterY",3500d)
            .set("actor:selector:bounds",new Rectangle(3494,3494,13,13))
            .set("actor:selector:x1",new int[]{0}).set("actor:selector:x2",new int[]{0})
            .set("actor:selector:y1",new int[]{-5}).set("actor:selector:y2",new int[]{5})
            .set("actor:selector:_m",new double[]{2d}).set("actor:selector:_mphoto",new double[]{0d})
            .set("actor:selector:_mass",1d).set("actor:selector:_I",1d).set("actor:selector:_energy",100d)
            .set("actor:selector:dx",0d).set("actor:selector:dy",0d).set("actor:selector:dtheta",0d)
            .set("actor:selector:_canreact",false).set("actor:selector:_usepretoucheffects",false);
    }
    private static List<Spec> canonical()throws Exception {
        List<Spec> result=new ArrayList<>();
        result.add(scenario("physical-basic-crossing","GREEN","GREEN"));
        result.add(scenario("physical-no-bounds-hit","GREEN","GREEN").set("actor:killer:bounds",new Rectangle(5000,5000,13,13)));
        result.add(scenario("physical-zero-peer-bounds","GREEN","GREEN").set("actor:killer:bounds",new Rectangle(3500,3500,0,13)));
        result.add(scenario("physical-short-peer","GREEN","GREEN").set("actor:killer:_m",new double[]{0.999d}));
        result.add(scenario("physical-length-exact-one","GREEN","GREEN").set("actor:victim:_m",new double[]{1d}).set("actor:killer:_m",new double[]{1d}));
        result.add(scenario("physical-finite-separation","GREEN","GREEN").set("actor:killer:x1",new int[]{10}).set("actor:killer:x2",new int[]{10}));
        result.add(scenario("physical-signed-positive-determinant","GREEN","GREEN").set("actor:killer:y1",new int[]{-1}).set("actor:killer:y2",new int[]{9}).set("actor:victim:dy",1d));
        result.add(scenario("physical-signed-negative-determinant","GREEN","GREEN").set("actor:killer:y1",new int[]{9}).set("actor:killer:y2",new int[]{-1}).set("actor:victim:dy",1d));
        result.add(scenario("physical-collinear-endpoint","GREEN","GREEN").set("actor:killer:x1",new int[]{5}).set("actor:killer:x2",new int[]{10}).set("actor:killer:y1",new int[]{0}).set("actor:killer:y2",new int[]{0}));
        result.add(scenario("physical-degenerate-line","GREEN","GREEN").set("actor:victim:x1",new int[]{0}).set("actor:victim:x2",new int[]{0}));
        result.add(scenario("physical-zero-mass-inertia","GREEN","GREEN").set("actor:victim:_mass",0d).set("actor:killer:_mass",0d).set("actor:victim:_I",0d).set("actor:killer:_I",0d));
        result.add(scenario("physical-actor-growth-only","GREEN","GREEN").set("actor:victim:hasGrown",1).set("actor:killer:hasGrown",-1).set("actor:victim:_dCenterX",3499d));
        result.add(scenario("physical-wrapped-integer-endpoints","GREEN","GREEN")
            .set("actor:victim:_centerX",Integer.MAX_VALUE).set("actor:victim:_centerY",0).set("actor:victim:_dCenterX",(double)Integer.MIN_VALUE+1).set("actor:victim:_dCenterY",0d)
            .set("actor:victim:x1",new int[]{1}).set("actor:victim:x2",new int[]{3}).set("actor:victim:bounds",new Rectangle(Integer.MIN_VALUE,-2,4,5))
            .set("actor:killer:_centerX",Integer.MIN_VALUE+1).set("actor:killer:_centerY",0).set("actor:killer:_dCenterX",(double)Integer.MIN_VALUE+1).set("actor:killer:_dCenterY",0d)
            .set("actor:killer:y1",new int[]{-1}).set("actor:killer:y2",new int[]{1}).set("actor:killer:bounds",new Rectangle(Integer.MIN_VALUE,-2,4,5)));
        Spec spike=scenario("physical-descending-pair-spike-tie","SPIKE","SPIKE");spike.parent.colors("SPIKE","SPIKE");spike.recipient.colors("SPIKE","SPIKE");result.add(spike
            .set("actor:victim:_isspike",true).set("actor:killer:_isspike",true).set("actor:victim:_usepretoucheffects",true).set("actor:killer:_usepretoucheffects",true)
            .set("actor:victim:_mphoto",new double[]{-2d,-2d}).set("actor:killer:_mphoto",new double[]{-2d,-2d}));
        for(double marker:new double[]{0d,-1d,Double.longBitsToDouble(0x7ff8000000000042L)})result.add(scenario("spike-nonspike-tie-marker-"+bits(marker),"SPIKE","GREEN")
            .set("actor:victim:_isspike",true).set("actor:victim:_usepretoucheffects",true).set("actor:victim:_mphoto",new double[]{marker}).set("actor:victim:_parentID",2));
        result.add(scenario("spike-peer-pretouch-after-actor","GREEN","SPIKE").set("actor:killer:_isspike",true).set("actor:killer:_usepretoucheffects",true).set("actor:killer:_mphoto",new double[]{-2d}).set("actor:victim:_parentID",2));
        result.add(scenario("impulse-before-reaction-brake","TEAL","GREEN").setting("ELASTICITY",1d).set("actor:victim:_canreact",true).set("actor:victim:_seggreenReaction",new int[]{1}).set("actor:victim:dy",1d));
        Spec mimic=scenario("pretouch-mimic-fresh-friendship-and-marker","DARK","RED");mimic.parent.generationBattle=true;result.add(mimic.set("actor:victim:_parentID",2).set("actor:victim:_usepretoucheffects",true).set("actor:victim:_segmentpretoucheffects",2));
        result.add(scenario("peer-pretouch-mimic-and-current-reverse-marker","RED","DARK").set("actor:killer:_usepretoucheffects",true).set("actor:killer:_segmentpretoucheffects",2));
        for(String relation:new String[]{"parent","reverse-parent","sibling","generation-block","sibling-block","peaceful","social","social-mismatch"}){
            Spec friends=scenario("friendship-"+relation,"TEAL","GREEN").set("actor:victim:_canreact",true).set("actor:victim:_segfriendReaction",new int[]{1}).set("actor:victim:dy",1d);
            switch(relation){
                case "parent":friends.set("actor:victim:_parentID",2);break;
                case "reverse-parent":friends.set("actor:killer:_parentID",1);break;
                case "sibling":friends.set("actor:victim:_parentID",9).set("actor:killer:_parentID",9);break;
                case "generation-block":friends.set("actor:victim:_parentID",2).set("actor:killer:_generationbattle",true);break;
                case "sibling-block":friends.set("actor:victim:_parentID",9).set("actor:killer:_parentID",9).set("actor:victim:_siblingbattle",true);break;
                case "peaceful":friends.parent.peaceful=true;friends.recipient.peaceful=true;friends.set("actor:victim:_transfersenergy",true).set("actor:killer:_transfersenergy",true);break;
                default:friends.parent.social=true;friends.recipient.social=true;friends.set("actor:victim:_transfersenergy",true).set("actor:killer:_transfersenergy",true).set("actor:victim:_lengthfriend",23).set("actor:killer:_lengthfriend",23).set("actor:victim:_thetafriend",29).set("actor:killer:_thetafriend",relation.equals("social")?29:30);break;
            }result.add(friends);
        }
        Spec sharing=scenario("friendly-effects-after-forward-reaction","ROSE","GREEN");sharing.parent.colors("TEAL","ROSE");result.add(sharing.set("actor:victim:_parentID",2).set("actor:victim:_canreact",true).set("actor:victim:_segfriendReaction",new int[]{0,1}).set("actor:victim:dy",1d).set("actor:victim:_transfersenergy",true).set("actor:victim:_usefriendeffects",1).set("actor:victim:_growthRatio",1).set("actor:victim:_altruist",true).set("actor:killer:_altruist",true).set("actor:killer:_energy",5d));
        result.add(scenario("impulse-before-ochre-biological-overwrite","OCHRE","GREEN").set("actor:victim:_mphoto",new double[]{-1d}).set("actor:victim:_useframemovement",true).set("actor:victim:dy",1d).set("rng:seed",1L));
        result.add(scenario("dead-peer-does-not-use-friendship","TEAL","BROWN").set("actor:victim:_parentID",2).set("actor:killer:alive",false).set("actor:victim:_canreact",true).set("actor:victim:_segbrownReaction",new int[]{1}));
        Spec reverse=scenario("reverse-reaction-after-actor-death","GRAY","BLUE");reverse.recipient.colors("TEAL","BLUE");result.add(reverse
            .setting("BLUE_ENERGY_CONSUMPTION",0.125d).setting("TEAL_ENERGY_CONSUMPTION",0.25d)
            .set("actor:victim:_energy",0.5d).set("actor:victim:_mphoto",new double[]{-2d}).set("actor:killer:_isenhanced",true)
            .set("actor:killer:_canreact",true).set("actor:killer:_m",new double[]{0.5d,2d}).set("actor:killer:_segbrownReaction",new int[]{0,1}));
        result.add(scenario("forward-gray-kill-fresh-reverse-state","GRAY","TEAL").set("actor:victim:_mphoto",new double[]{-2d}).set("actor:killer:_canreact",true).set("actor:killer:_seggrayReaction",new int[]{1}).set("actor:killer:_mphoto",new double[]{-2d}));
        result.add(eye("eye-moving-ascending-peers-false-return",true).set("world:_O2",1.5d));
        result.add(eye("eye-immobile-brakes-every-hit",false).set("actor:victim:_segredReaction",new int[]{0,1}));
        result.add(eye("eye-friendly-brakes-without-canreact",false).set("actor:victim:_parentID",2).set("actor:victim:_segfriendReaction",new int[]{0,1}));
        result.add(eye("eye-dead-immobile-skips",false).set("actor:victim:alive",false));
        result.add(eye("eye-dead-moving-still-senses",true).set("actor:victim:alive",false));
        result.add(eye("eye-entry-flag-disabled",true).set("actor:victim:_haseyes",false));
        for(double length:new double[]{0d,Double.longBitsToDouble(0x7ff8000000000042L),2d})result.add(eye("eye-length-"+bits(length),true).set("actor:victim:_m",new double[]{0.5d,length}));
        result.add(eye("eye-translucent-ARGB-not-eye",true).set("actor:victim:_segColor",new Color[]{color("TEAL"),new Color(0x12004040,true)}));
        result.add(eye("eye-skips-short-and-nan-peer-segments",true).set("actor:killer:_m",new double[]{0.5d,Double.NaN}));
        Spec outer=eye("eye-descending-outer-rays",true);outer.parent.colors("TEAL","EYE","EYE");result.add(outer.set("actor:victim:_m",new double[]{0.5d,0.5d,0.5d}).set("actor:victim:_seggreenReaction",new int[]{0,2,1}).set("actor:victim:_segredReaction",new int[]{0,0,0}).set("world:_O2",1.5d));
        result.add(worldCase(scenario("world-self-identity-only-tree","GREEN","GREEN"),"victim"));
        result.add(worldCase(selectorGeometry(scenario("world-distinct-equal-numeric-id","GREEN","GREEN")).set("actor:selector:_ID",1),"victim,selector,killer"));
        result.add(worldCase(scenario("world-dead-peer-still-physical","GREEN","BROWN").set("actor:killer:alive",false),"victim,killer"));
        for(String who:new String[]{"victim","killer"})for(double energy:new double[]{Math.nextDown(Utils.tol),Utils.tol,Double.NaN})result.add(worldCase(scenario("world-energy-gate-"+who+"-"+bits(energy),"GREEN","GREEN").set("actor:"+who+":_energy",energy),"victim,killer"));
        result.add(worldCase(scenario("world-rectangle-edge-does-not-overlap","GREEN","GREEN").set("actor:killer:bounds",new Rectangle(3507,3494,13,13)),"victim,killer"));
        Spec continuation=eye("world-eye-false-continues-to-physical-peer",true);continuation.recipient.colors("GREEN");result.add(worldCase(selectorGeometry(continuation)
            .set("actor:victim:_m",new double[]{2d,0.5d}).set("actor:victim:y1",new int[]{10,0}).set("actor:victim:y2",new int[]{10,0})
            .set("actor:victim:bounds",new Rectangle(3494,3494,13,23)).set("actor:victim:_segredReaction",new int[]{0,0})
            .set("actor:killer:y1",new int[]{-5}).set("actor:killer:y2",new int[]{15}).set("actor:killer:bounds",new Rectangle(3494,3494,13,23)),"victim,selector,killer"));
        result.add(worldCase(eye("world-all-eye-candidates-false-with-effects",true).set("actor:victim:_segredReaction",new int[]{0,1}),"victim,killer"));
        for(String kind:new String[]{"PLAGUE","YELLOW"}){
            Spec born=scenario("physical-synchronous-birth-"+kind,kind,"GREEN").set("actor:victim:_mphoto",new double[]{-2d})
                .set("actor:victim:_isinfectious",true).set("actor:victim:_isaplant",kind.equals("PLAGUE")).set("actor:victim:_plagueversion",1).set("actor:victim:_energy",20d)
                .set("actor:killer:_isaplant",true).set("actor:killer:_infectedGeneticCode","g0").set("actor:killer:_savedGeneticCode","g4")
                .set("actor:killer:_infectedReproduceEnergy",43).set("actor:killer:_energy",1000d).set("actor:victim:_yellowCounter",1).setting("VIRUS_ENERGY_CONSUMPTION",0.125d);
            born.add(new Action("build_tree","members"));result.add(born);
        }
        Spec converted=scenario("physical-coral-live-transformation","CORAL","WHITE").set("actor:victim:_mphoto",new double[]{-1d})
            .set("actor:victim:_energy",20d).set("actor:killer:_energy",20d).set("actor:killer:_infectedGeneticCode","g3").set("actor:killer:_savedGeneticCode","g4");
        converted.add(new Action("build_tree","members"));result.add(converted);
        require(result.stream().map(spec->spec.name).distinct().count()==result.size(),"Case names must be unique");return result;
    }
    private static String capture()throws Exception {
        StringBuilder output=new StringBuilder();for(Spec spec:canonical()){Capture capture=new Capture(spec);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index));output.append(capture.finish());}return output.toString();
    }
    public static void main(String[]args)throws Exception {
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.ContactOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        Files.writeString(directory.resolve("contact.tsv"),capture(),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
        System.out.println("CONTACT_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls+" newborns="+newborns);
    }
}
