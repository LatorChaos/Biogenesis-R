// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Color;
import java.awt.Rectangle;
import java.awt.Frame;
import java.awt.Window;
import java.util.concurrent.FutureTask;
import java.util.prefs.Preferences;
import javax.swing.JLabel;
import javax.swing.SwingUtilities;
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
import org.locationtech.jts.index.strtree.AbstractSTRtree;
import org.locationtech.jts.index.strtree.AbstractNode;
import org.locationtech.jts.index.strtree.ItemBoundable;

/** Observe real incoming candidates, retained retries, and local frames */
public final class IncomingEmptyOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static int cases,snapshots,calls,newborns;
    private static MainWindow window;
    private static final StringBuilder uiRows=new StringBuilder();
    private static final class Code {
        String[] colors={"GREEN"};
        int symmetry=1,mirror,rate,spore=1,adapt=1,generation;
        int[] branches;
        double length=12;
        boolean disperse,selfish,plague,generationBattle,siblingBattle,social,peaceful;
        String clade;
        Code colors(String...value){colors=value;return this;}
    }
    private static final class Action {
        final String operation,argument;
        String target="victim",exception;
        final Map<String,Object> controls=new LinkedHashMap<>();
        Action(String operation,String argument){this.operation=operation;this.argument=argument;}
        Action set(String target,Object value){controls.put(target,value);return this;}
        Action target(String value){target=value;return this;}
        Action throwsClass(String value){exception=value;return this;}
    }
    private static final class Spec {
        final String name;
        final Code parent=new Code(),recipient=new Code(),infected=new Code().colors("WHITE");
        final Map<String,Object> settings=new LinkedHashMap<>(),controls=new LinkedHashMap<>();
        final List<Action> actions=new ArrayList<>();
        String[] roles={"selector"};
        boolean diagnostic;
        boolean naturalSettings,contactGeometry;
        long seed=827364;
        Spec(String name){this.name=name;}
        Spec set(String target,Object value){controls.put(target,value);return this;}
        Spec setting(String target,Object value){settings.put(target,value);return this;}
        Spec add(Action value){actions.add(value);return this;}
        Spec frames(int count){actions.clear();for(int i=0;i<count;i++)actions.add(new Action("time","~"));return this;}
    }
    private static final class Capture {
        final StringBuilder rows=new StringBuilder();
        final Map<String,GeneticCode> genomes=new LinkedHashMap<>();
        final Map<String,Organism> actors=new LinkedHashMap<>();
        final Map<String,InCorridor> corridors=new LinkedHashMap<>();
        final IdentityHashMap<InCorridor,String> corridorNames=new IdentityHashMap<>();
        final IdentityHashMap<GeneticCode,String> references=new IdentityHashMap<>();
        final IdentityHashMap<Organism,String> actorNames=new IdentityHashMap<>();
        final World world;
        final Spec spec;
        int discovered;
        Capture(Spec spec)throws Exception {
            this.spec=spec;
            for(Field field:Utils.class.getDeclaredFields())if(field.getName().startsWith("DEF_"))Utils.class.getDeclaredField(field.getName().substring(4)).set(null,field.get(null));
            Utils.INITIAL_ORGANISMS=0;Utils.INITIAL_O2=1000;
            Utils.ACCEPT_CONNECTIONS=false;Utils.CONNECT_TO_SERVER=false;Utils.AUTO_BACKUP=false;Utils.HARDWARE_ACCELERATION=0;
            if(!spec.naturalSettings){Utils.MIN_MUTATION_RATE=0;Utils.MAX_MUTATION_RATE=10000;Utils.META_MUTATION_RATE=0;
                for(String name:COLORS)Utils.class.getDeclaredField(name+"_PROB").setInt(null,name.equals("GREEN")?7:0);
            }
            for(Map.Entry<String,Object> entry:spec.settings.entrySet())Utils.class.getDeclaredField(entry.getKey()).set(null,entry.getValue());
            Utils.random=new Random(spec.seed);
            VisibleWorld visible=window.getVisibleWorld();visible.setSelectedOrganism(null);world=new World(visible);window._world=world;
            require(window.getWorld()==world&&visible._mainWindow==window&&window.getInfoPanel()!=null,"Actual UI collaborators must own the case World");
            register("g0",genome(spec.parent));register("g1",genome(spec.recipient));register("g2",genome(new Code().colors("GREEN")));
            register("g3",genome(spec.infected));register("g4",(GeneticCode)genomes.get("g0").clone());
            row("C",spec.name,spec.diagnostic?"incoming-diagnostic":"incoming",spec.seed);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            if(spec.roles.length>0){
                control("setup","world:_CO2",Utils.INITIAL_CO2);control("setup","world:_O2",Utils.INITIAL_O2);control("setup","world:_CH4",Utils.INITIAL_CH4);
            }
            for(String role:spec.roles){
                String code=role.equals("victim")?"g0":role.equals("blocker")?"g1":"g2";int center=role.equals("selector")?1000:3500;
                Organism actor=paste(role,code,center,center);world.addOrganism(actor,null);
                if(role.equals("selector"))control("setup","visible:selected",role);
            }
            if(spec.contactGeometry){geometry("victim",actors.get("victim"),false);geometry("killer",actors.get("killer"),true);}
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
                registerArrival(stage,actor);
            }
            for(InCorridor corridor:corridors.values()){
                Organism candidate=(Organism)read(corridor,"nextOrganism");
                if(candidate!=null&&!actorNames.containsKey(candidate))registerArrival(stage,candidate);
            }
            for(Organism actor:actors.values()){reference(actor._geneticCode);reference(actor._infectedGeneticCode);reference(actor._savedGeneticCode);}
            for(Field field:fields(WorldStatistics.class))if(field.getType()==GeneticCode.class)reference((GeneticCode)field.get(world.worldStatistics));
        }
        void registerArrival(String stage,Organism actor){
            String label="arrival"+discovered++;newborns++;actors.put(label,actor);actorNames.put(actor,label);
            row("D",stage,label,actor.getID(),reference(actor._geneticCode));
        }
        Organism paste(String role,String code,int x,int y)throws Exception {
            Organism actor=new Organism(world,genomes.get(code));require(actor.pasteOrganism(x,y),"Real actor paste failed: "+role);
            actors.put(role,actor);actorNames.put(actor,role);row("A",role,code,x,y);return actor;
        }
        void row(Object...values){for(int i=0;i<values.length;i++){if(i!=0)rows.append('\t');rows.append(values[i]);}rows.append('\n');}
        String scalar(Object value)throws Exception{
            if(value==null)return "~";
            if(value instanceof Double number)return bits(number);
            if(value instanceof Color color)return String.format(java.util.Locale.ROOT,"%08x",color.getRGB());
            if(value instanceof GeneticCode code)return reference(code);
            if(value instanceof InCorridor corridor){String label=corridorNames.get(corridor);require(label!=null,"Unknown corridor reference");return label;}
            if(value instanceof Organism actor){String label=actorNames.get(actor);require(label!=null,"Unknown actor reference");return label;}
            if(value instanceof Rectangle r)return r.x+","+r.y+","+r.width+","+r.height;
            if(value instanceof World||value instanceof VisibleWorld||value instanceof WorldStatistics)return "present";
            if(value instanceof STRtree tree)return "present,"+storedEnvelopes(tree).size();
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
            for(Map.Entry<String,InCorridor> entry:corridors.entrySet()){
                InCorridor corridor=entry.getValue();
                row("F",stage,entry.getKey(),"_reference","String",entry.getKey());
                row("F",stage,entry.getKey(),"bounds","Rectangle",scalar(corridor.getBounds()));
                for(Class<?> type:new Class<?>[]{InCorridor.class,Corridor.class})for(Field field:fields(type)){
                    Object value=field.get(corridor);
                    row("F",stage,entry.getKey(),field.getName(),field.getType().getSimpleName(),scalar(value));
                }
            }
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
            Random peek=copyRandom(Utils.random);row("R",stage,peek.nextLong(),peek.nextInt(10000),bits(peek.nextDouble()));
            Organism selected=world._visibleWorld.getSelectedOrganism();String selectedName=selected==null?"~":actorNames.get(selected);
            uiRows.append(spec.name).append('\t').append(stage).append('\t').append(selectedName).append('\t')
                .append(label("_lEnergy")).append('\t').append(label("_lAge")).append('\t').append(label("_lMass")).append('\n');snapshots++;
        }
        void control(String stage,String target,Object value)throws Exception {
            String[] path=target.split(":",3);Object owner;String name;
            if(target.equals("visible:selected")){world._visibleWorld.setSelectedOrganism(value.equals("~")?null:actors.get((String)value));row("I",stage,target,"String",value);return;}
            if(path[0].equals("rng")){require(path[1].equals("seed"),"Unknown RNG control");Utils.random=new Random((Long)value);row("I",stage,target,"long",value);return;}
            if(target.equals("world:inCorridors")){
                @SuppressWarnings("unchecked") List<InCorridor> actual=(List<InCorridor>)read(world,"inCorridors");
                actual.clear();if(!value.equals("~"))for(String label:((String)value).split(",")){require(corridors.containsKey(label),"Unknown registry identity");actual.add(corridors.get(label));}
                row("I",stage,target,"List",value);return;
            }
            switch(path[0]){
                case "actor":owner=actors.get(path[1]);name=path[2];break;case "world":owner=world;name=path[1];break;case "corridor":owner=corridors.get(path[1]);name=path[2];break;
                case "statistics":owner=world.worldStatistics;name=path[1];break;case "genome":owner=genomes.get(path[1]);name=path[2];break;
                default:throw new IllegalArgumentException("Unknown explicit input target: "+target);
            }
            if(name.equals("bounds")){((Rectangle)owner).setBounds((Rectangle)value);row("I",stage,target,"Rectangle",scalar(value));return;}
            if(name.startsWith("gene")){
                int dot=name.indexOf('.');int index=Integer.parseInt(name.substring(4,dot));Gene gene=((GeneticCode)owner).getGene(index);String property=name.substring(dot+1);
                if(property.equals("_length")){gene.setLength((Double)value);row("I",stage,target,"double",scalar(value));}
                else if(property.equals("_color")){gene.setColor((Color)value);row("I",stage,target,"Color",scalar(value));}
                else if(property.equals("_branch")){gene.setBranch((Integer)value);row("I",stage,target,"int",value);}
                else throw new IllegalArgumentException("Unknown original Gene setter");return;
            }
            Field field=owner.getClass().getDeclaredField(name);field.setAccessible(true);
            if(owner instanceof Organism actor&&field.getType().isArray())require(Array.getLength(value)==actor._segments,"Explicit array input length must match actual segments");
            if(field.getType()==GeneticCode.class){field.set(owner,value.equals("~")?null:genomes.get((String)value));row("I",stage,target,"GeneticCode",value);}
            else {field.set(owner,value);row("I",stage,target,field.getType().getSimpleName(),scalar(value));}
        }
        void frozenRows(String stage)throws Exception {
            IdentityHashMap<Organism,Envelope> entries=storedEnvelopes(world.colDetTree);int seen=0;
            for(Map.Entry<String,Organism> entry:actors.entrySet()){
                Envelope bounds=entries.get(entry.getValue());if(bounds==null)continue;seen++;
                row("J",stage,entry.getKey(),exactInt(bounds.getMinX()),exactInt(bounds.getMinY()),exactInt(bounds.getWidth()),exactInt(bounds.getHeight()));
            }
            require(seen==entries.size(),"Every actual stored identity must be observed");
        }
        void action(int index,Action action)throws Exception {
            String stage="step-"+index;
            for(Map.Entry<String,Object> entry:action.controls.entrySet())control(stage,entry.getKey(),entry.getValue());
            row("K",stage,action.operation,action.target,action.argument);Throwable observed=null;
            try {
                switch(action.operation){
                    case "corridor.new":{
                        require(!corridors.containsKey(action.target)&&action.argument.equals("~"),"Fresh incoming identity required");
                        InCorridor corridor=new InCorridor(world);corridors.put(action.target,corridor);corridorNames.put(corridor,action.target);break;
                    }
                    case "corridor.receive":corridors.get(action.target).receiveOrganism(action.argument.equals("~")?null:genomes.get(action.argument));break;
                    case "corridor.frame":require(action.argument.equals("~"),"One actual incoming frame required");corridors.get(action.target).frame();break;
                    case "construct":{
                        require(!actors.containsKey(action.target)&&genomes.containsKey(action.argument),"Fresh actor and existing code required");
                        Organism actor=new Organism(world,genomes.get(action.argument));actors.put(action.target,actor);actorNames.put(actor,action.target);break;
                    }
                    case "paste":{
                        String[] point=action.argument.split(",");require(point.length==2,"Explicit integer paste coordinates required");
                        boolean result=actors.get(action.target).pasteOrganism(Integer.parseInt(point[0]),Integer.parseInt(point[1]));
                        row("B",stage,"paste",result);break;
                    }
                    case "time":require(action.argument.equals("~")&&action.target.equals("victim"),"One actual World.time required");world.time();break;
                    default:throw new IllegalArgumentException("Unknown incoming oracle operation: "+action.operation);
                }
            } catch(RuntimeException error){observed=error;}
            if(action.exception==null){if(observed!=null)throw new IllegalStateException("Unexpected source failure in "+spec.name+"/"+stage,observed);}
            else {
                require(observed!=null&&observed.getClass().getName().equals(action.exception),"Expected actual source exception was absent or differed");
                String message=observed.getMessage()==null?"~":java.util.Base64.getEncoder().encodeToString(observed.getMessage().getBytes(StandardCharsets.UTF_8));
                row("X",stage,observed.getClass().getName(),message);
            }
            calls++;discover(stage);frozenRows(stage);snapshot(stage);
        }
        String finish(){cases++;row("E");return rows.toString();}
    }
    private static Object read(Object owner,String name)throws Exception{Field field=owner.getClass().getDeclaredField(name);field.setAccessible(true);return field.get(owner);}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Random copyRandom(Random random)throws Exception {ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream out=new ObjectOutputStream(bytes)){out.writeObject(random);}try(ObjectInputStream in=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)in.readObject();}}
    private static Color color(String name)throws Exception {
        switch(name){case "UNKNOWN":return new Color(0x12345678,true);case "TRANSLUCENT":return new Color(0x12ff00ff,true);case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}
    }
    private static GeneticCode genome(Code recipe)throws Exception {
        List<Gene> genes=new ArrayList<>();for(int index=0;index<recipe.colors.length;index++){
            Gene gene=new Gene();gene.setLength(recipe.length);gene.setTheta(index*0.25);gene.setBranch(recipe.branches==null?0:recipe.branches[index]);gene.setColor(color(recipe.colors[index]));genes.add(gene);
        }
        GeneticCode code=new GeneticCode(genes,recipe.symmetry,recipe.mirror,recipe.rate,0,1,1,recipe.spore,recipe.adapt,recipe.plague,recipe.disperse,recipe.generationBattle,recipe.siblingBattle,false,false,recipe.social,recipe.peaceful,false,false,false,false,false,false,recipe.selfish);
        code._generation=recipe.generation;code._cladeID=recipe.clade;return code;
    }
    private static int[] integers(int count,int value){int[] values=new int[count];Arrays.fill(values,value);return values;}
    private static double[] doubles(int count,double value){double[] values=new double[count];Arrays.fill(values,value);return values;}
    private static Color[] phenotype(String...names)throws Exception{Color[] colors=new Color[names.length];for(int i=0;i<names.length;i++)colors[i]=color(names[i]);return colors;}
    private static int exactInt(double value){int integer=(int)value;require(value==integer,"Frozen envelope component must remain an exact i32");return integer;}
    private static void collectStored(Object value,IdentityHashMap<Organism,Envelope> entries){
        if(value==null)return;
        if(value instanceof ItemBoundable item){require(item.getItem() instanceof Organism&&item.getBounds() instanceof Envelope,"Unexpected stored tree entry");require(entries.put((Organism)item.getItem(),(Envelope)item.getBounds())==null,"Duplicate source tree identity");}
        else if(value instanceof AbstractNode node){for(Object child:node.getChildBoundables())collectStored(child,entries);}
        else throw new IllegalStateException("Unknown actual JTS boundable");
    }
    private static IdentityHashMap<Organism,Envelope> storedEnvelopes(STRtree tree)throws Exception {
        Field pending=AbstractSTRtree.class.getDeclaredField("itemBoundables"),root=AbstractSTRtree.class.getDeclaredField("root"),built=AbstractSTRtree.class.getDeclaredField("built");
        pending.setAccessible(true);root.setAccessible(true);built.setAccessible(true);Object pendingBefore=pending.get(tree),rootBefore=root.get(tree);boolean builtBefore=built.getBoolean(tree);
        IdentityHashMap<Organism,Envelope> entries=new IdentityHashMap<>();
        if(pendingBefore!=null){for(Object item:(List<?>)pendingBefore)collectStored(item,entries);}else collectStored(rootBefore,entries);
        require(pending.get(tree)==pendingBefore&&root.get(tree)==rootBefore&&built.getBoolean(tree)==builtBefore,"Observing frozen envelopes must not build or mutate the tree");return entries;
    }
    private static String label(String field)throws Exception{Field selected=InfoToolbar.class.getDeclaredField(field);selected.setAccessible(true);return ((JLabel)selected.get(window.getInfoPanel())).getText();}
    private static Spec scenario(String name){return new Spec(name);}
    private static Action operation(String name,String target,String argument){return new Action(name,argument).target(target);}
    private static Spec incoming(String name){Spec spec=scenario(name);spec.add(operation("corridor.new","c0","~"));return spec;}
    private static Action attempt(String corridor){return operation("corridor.frame",corridor,"~");}
    private static List<Spec> canonical()throws Exception {
        Spec spec=incoming("empty-genome-arrival-retains-zero-mass-state-over-twelve-real-frames");
        spec.parent.colors();
        spec.add(operation("corridor.receive","c0","g0"));
        for(int frame=0;frame<12;frame++){
            Action time=operation("time","victim","~");
            if(frame==0)time.set("world:inCorridors","c0").set("world:_corridorexists",true).set("corridor:c0:bounds",new Rectangle(3500,3500,1,1));
            spec.add(time);
        }
        return List.of(spec);
    }
    private static List<Spec> diagnostic(){return List.of();}
    private static String capture(boolean diagnostics)throws Exception {
        StringBuilder output=new StringBuilder();for(Spec spec:diagnostics?diagnostic():canonical()){
            Capture capture=new Capture(spec);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index));output.append(capture.finish());
        }return output.toString();
    }
    public static void main(String[]args)throws Exception {
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.IncomingEmptyOracle NEW_OUTPUT_DIRECTORY");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent)&&!Path.of(value).toRealPath().equals(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");
        require("1".equals(System.getenv("BGR_ISOLATED_XVFB"))&&System.getenv("DISPLAY")!=null&&System.getenv("DISPLAY").matches(":[1-9][0-9]{2,}"),"Owned isolated Xvfb required");
        Files.createDirectory(directory);Throwable failure=null;int status=0;
        try {
            Preferences preferences=Preferences.userNodeForPackage(Utils.class);
            preferences.putInt("VERSION",700);preferences.putInt("INITIAL_ORGANISMS",0);preferences.putInt("WORLD_WIDTH",7000);preferences.putInt("WORLD_HEIGHT",7000);
            preferences.putBoolean("ACCEPT_CONNECTIONS",false);preferences.putBoolean("CONNECT_TO_SERVER",false);preferences.putBoolean("AUTO_BACKUP",false);
            preferences.putInt("HARDWARE_ACCELERATION",0);preferences.putInt("DELAY",20);preferences.put("LOCALE","en");preferences.flush();
            SwingUtilities.invokeAndWait(()->MainWindow.main(new String[]{"827364"}));
            for(Frame frame:Frame.getFrames())if(frame instanceof MainWindow main)window=main;
            require(window!=null&&!window.isProcessActive()&&window.serverThread==null,"Actual paused window without a network server required");
            if(window._timer!=null)window._timer.cancel();
            FutureTask<String> measurement=new FutureTask<>(()->capture(false));SwingUtilities.invokeAndWait(measurement);
            Files.writeString(directory.resolve("incoming_empty.tsv"),measurement.get(),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
            Files.writeString(directory.resolve("ui.tsv"),uiRows,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
            Files.writeString(directory.resolve("bootstrap.json"),"{\"main_window\":\""+window.getClass().getName()+"\",\"info_toolbar\":\""+window.getInfoPanel().getClass().getName()+"\",\"paused\":true,\"timer_cancelled\":true,\"owned_display\":\""+System.getenv("DISPLAY")+"\",\"cases\":"+cases+"}\n",StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
            System.out.println("INCOMING_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls+" discoveries="+newborns);
        } catch(Throwable error){failure=error;status=1;}
        finally {
            try {
                SwingUtilities.invokeAndWait(()->{for(Frame frame:Frame.getFrames())if(frame instanceof MainWindow main&&main._timer!=null)main._timer.cancel();for(Window target:Window.getWindows())target.dispose();});
            } catch(Throwable cleanup){status=1;if(failure==null)failure=cleanup;else failure.addSuppressed(cleanup);}
            finally {if(failure!=null)failure.printStackTrace();System.exit(status);}
        }
    }
}
