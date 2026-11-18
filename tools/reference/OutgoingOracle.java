// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Color;
import java.awt.Graphics;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Shape;
import java.awt.Image;
import java.awt.image.BufferedImage;
import java.awt.image.ImageObserver;
import java.text.AttributedCharacterIterator;
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

/** Observe real outgoing transfer, paint transitions, and retained aliases */
public final class OutgoingOracle {
    private static final String[] COLORS=("RED GREEN BLUE CYAN WHITE GRAY MAGENTA PINK CORAL ORANGE FOREST SPRING LEAF SUMMER LIME BARK VIOLET TEAL EYE MAROON OLIVE MINT CREAM ROSE DARK OCHRE SKY LILAC SILVER FIRE DARKGRAY GOLD BLOND FLOWER AUBURN PLAGUE SPIKE INDIGO LAVENDER FALLOW SPORE JADE C4 GRASS PURPLE YELLOW").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static int cases,snapshots,calls,newborns;
    private static MainWindow window;
    private static final StringBuilder uiRows=new StringBuilder();
    private static final class Main extends MainWindow {
        @Override protected void checkLastVersion(){throw new AssertionError("Version check forbidden");}
        @Override public NetServerThread startServer(){throw new AssertionError("Server startup forbidden");}
        @Override public void startApp(){setSize(900,700);setVisible(true);}
    }
    private static final class Code {
        String[] colors={"GREEN"};
        int symmetry=1,mirror,rate,spore=1,adapt=1,generation;
        int[] branches;
        double length=12;
        boolean disperse,selfish,plague,generationBattle,siblingBattle,social,peaceful;
        String clade="root";
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
        String[] roles={"selector","victim","killer"};
        boolean integration,worldRules;
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
        final Map<String,OutCorridor> corridors=new LinkedHashMap<>();
        final Map<String,InCorridor> incoming=new LinkedHashMap<>();
        final IdentityHashMap<InCorridor,String> incomingNames=new IdentityHashMap<>();
        final Map<String,IdentityHashMap<Object,String>> owners=new LinkedHashMap<>();
        final Map<String,RecordingConnection> connections=new LinkedHashMap<>();
        final IdentityHashMap<RecordingConnection,String> connectionNames=new IdentityHashMap<>();
        final IdentityHashMap<OutCorridor,String> corridorNames=new IdentityHashMap<>();
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
            row("C",spec.name,spec.worldRules?"world-rules":spec.integration?"outgoing-frames":"outgoing",spec.seed);
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g4")?"clone:g0":"explicit");
                genomeState("input",entry.getKey(),entry.getValue());
            }
            if(spec.roles.length>0){
                control("setup","world:_CO2",Utils.INITIAL_CO2);control("setup","world:_O2",Utils.INITIAL_O2);control("setup","world:_CH4",Utils.INITIAL_CH4);
            }
            control("setup","utils:USER_NAME","oracle");
            for(String role:spec.roles){
                String code=role.equals("victim")?"g0":role.equals("killer")?"g1":"g2";int center=role.equals("selector")?1000:role.equals("victim")?3500:5000;
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
            for(InCorridor corridor:incoming.values()){
                Organism candidate=(Organism)read(corridor,"nextOrganism");
                if(candidate!=null&&!actorNames.containsKey(candidate))registerArrival(stage,candidate);
                for(Object code:(java.util.Queue<?>)read(corridor,"pendingOrganisms"))reference((GeneticCode)code);
            }
            for(OutCorridor corridor:corridors.values()){
                Organism traveller=corridor.getTravellingOrganism();
                require(traveller==null||actorNames.containsKey(traveller),"Outgoing actor must originate from real supplied actors");
            }
            for(RecordingConnection connection:connections.values())for(GeneticCode code:connection.recorded)reference(code);
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
            if(value instanceof InCorridor corridor){String label=incomingNames.get(corridor);require(label!=null,"Unknown incoming corridor reference");return label;}
            if(value instanceof OutCorridor corridor){String label=corridorNames.get(corridor);require(label!=null,"Unknown corridor reference");return label;}
            if(value instanceof RecordingConnection connection){String label=connectionNames.get(connection);require(label!=null,"Unknown recording connection");return label;}
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
            for(Map.Entry<String,OutCorridor> entry:corridors.entrySet()){
                OutCorridor corridor=entry.getValue();
                row("F",stage,entry.getKey(),"_reference","String",entry.getKey());
                row("F",stage,entry.getKey(),"bounds","Rectangle",scalar(corridor.getBounds()));
                for(Field field:fields(Corridor.class))row("F",stage,entry.getKey(),field.getName(),field.getType().getSimpleName(),scalar(field.get(corridor)));
                row("F",stage,entry.getKey(),"canSendOrganism","boolean",corridor.canSendOrganism());
            }
            for(Map.Entry<String,InCorridor> entry:incoming.entrySet()){
                InCorridor corridor=entry.getValue();
                require(fields(InCorridor.class).length==2&&fields(Corridor.class).length==3,"Unexpected incoming corridor instance schema");
                row("IN",stage,entry.getKey(),"_reference","String",entry.getKey());
                row("IN",stage,entry.getKey(),"bounds","Rectangle",scalar(corridor.getBounds()));
                for(Class<?> type:new Class<?>[]{InCorridor.class,Corridor.class})for(Field field:fields(type)){
                    Object value=field.get(corridor);
                    row("IN",stage,entry.getKey(),field.getName(),field.getType().getSimpleName(),scalar(value));
                    if(value instanceof java.util.Queue<?> queue)for(Object code:queue)if(code!=null)available.put((GeneticCode)code,true);
                }
            }
            if(spec.worldRules){
                owner(stage,"world",world,"world");owner(stage,"statistics",world.worldStatistics,"statistics");
                owner(stage,"members",world._organisms,"members");owner(stage,"inCorridors",read(world,"inCorridors"),"incoming-list");
                owner(stage,"outCorridors",read(world,"outCorridors"),"outgoing-list");
                for(Map.Entry<String,Organism> entry:actors.entrySet())ownerLink(stage,"actor:"+entry.getKey()+":world",entry.getValue()._world,"world","world");
                for(Map.Entry<String,InCorridor> entry:incoming.entrySet()){
                    owner(stage,"incoming:"+entry.getKey(),entry.getValue(),entry.getKey()+":");
                    owner(stage,"pending:"+entry.getKey(),read(entry.getValue(),"pendingOrganisms"),"queue:"+entry.getKey()+":");
                    ownerLink(stage,"incoming:"+entry.getKey()+":world",entry.getValue().world,"world","world");
                }
                for(Map.Entry<String,RecordingConnection> entry:connections.entrySet())owner(stage,"connection:"+entry.getKey(),entry.getValue(),entry.getKey()+":");
                for(Map.Entry<String,OutCorridor> entry:corridors.entrySet()){
                    owner(stage,"outgoing:"+entry.getKey(),entry.getValue(),entry.getKey()+":");
                    ownerLink(stage,"outgoing:"+entry.getKey()+":world",entry.getValue().world,"world","world");
                    String connection=connectionNames.get(entry.getValue().connection);require(connection!=null,"Known outgoing connection owner required");
                    ownerLink(stage,"outgoing:"+entry.getKey()+":connection",entry.getValue().connection,"connection:"+connection,connection+":");
                }
            }
            for(Map.Entry<String,RecordingConnection> entry:connections.entrySet()){
                RecordingConnection connection=entry.getValue();
                row("CC",stage,entry.getKey(),"_reference","String",entry.getKey());
                row("CC",stage,entry.getKey(),"state","int",connection.getState());
                row("CC",stage,entry.getKey(),"recordedGenotypes","List",scalar(connection.recorded));
                require(readConnection(connection,"timer")==null&&connection.mainWindow==null&&connection.inCorridor==null&&connection.outCorridor==null,
                    "Recording boundary must not initialize timers, UI, or transport corridors");
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
        void owner(String stage,String path,Object value,String prefix){ownerLink(stage,path,value,path,prefix);}
        void ownerLink(String stage,String path,Object value,String domain,String prefix){
            require(value!=null,"Owner identity must exist: "+path);
            IdentityHashMap<Object,String> identities=owners.computeIfAbsent(domain,key->new IdentityHashMap<>());
            String identity=identities.get(value);
            if(identity==null){identity=prefix+identities.size();identities.put(value,identity);}
            row("ID",stage,path,identity);
        }
        void guardActiveGenesis()throws Exception {
            InCorridor in=incoming.get("i0");OutCorridor out=corridors.get("c0");
            require(Utils.INITIAL_ORGANISMS==2&&world.getWidth()==7000&&world.getHeight()==7000,"Fixed active genesis settings required");
            List<?> inRegistry=(List<?>)read(world,"inCorridors"),outRegistry=(List<?>)read(world,"outCorridors");
            require(inRegistry.size()==1&&inRegistry.get(0)==in&&outRegistry.size()==1&&outRegistry.get(0)==out&&(Boolean)read(world,"_corridorexists"),"Actual ordered active pair required");
            Organism candidate=(Organism)read(in,"nextOrganism");
            require(candidate!=null&&world._organisms.stream().noneMatch(actor->actor==candidate)&&candidate._geneticCode==genomes.get("g0"),"Retained nonmember incoming candidate required");
            Object[] queued=((java.util.Queue<?>)read(in,"pendingOrganisms")).toArray();
            require(queued.length==2&&queued[0]==genomes.get("g0")&&queued[1]==genomes.get("g4"),"Exact retained pending aliases required");
            require(out.getTravellingOrganism()==actors.get("victim")&&world._organisms.stream().noneMatch(actor->actor==actors.get("victim")),"Actual removed outgoing traveller required");
            require(connections.get("n0").recorded.size()==1&&connections.get("n0").recorded.get(0)==genomes.get("g0"),"Actual shared-code transport record required");
            require(storedEnvelopes(world.colDetTree).containsKey(actors.get("victim")),"Current frozen tree must retain traveller");
        }
        void guardGlobalBaseline()throws Exception {
            require(world._organisms.size()==3&&world._organisms.get(0)==actors.get("selector")&&world._organisms.get(1)==actors.get("victim")&&world._organisms.get(2)==actors.get("killer"),"Fixed global member order required");
            require(!actors.get("selector").isAlive()&&actors.get("victim").isAlive()&&actors.get("killer").isAlive(),"One existing corpse and two live actors required");
            require(actors.get("selector").getEnergy()==8d&&actors.get("victim").getEnergy()==13d&&actors.get("killer").getEnergy()==21d,"Declared global energies required");
            double oxygen=spec.name.endsWith("sufficient-o2")?64d:spec.name.endsWith("partial-o2")?5d:0d;
            require(world.getO2()==oxygen,"Declared global oxygen required");
        }
        void control(String stage,String target,Object value)throws Exception {
            String[] path=target.split(":",3);Object owner;String name;
            if(target.equals("visible:selected")){world._visibleWorld.setSelectedOrganism(value.equals("~")?null:actors.get((String)value));row("I",stage,target,"String",value);return;}
            if(path[0].equals("rng")){require(path[1].equals("seed"),"Unknown RNG control");Utils.random=new Random((Long)value);row("I",stage,target,"long",value);return;}
            if(target.equals("utils:USER_NAME")){Utils.USER_NAME=(String)value;row("I",stage,target,"String",value);return;}
            if(target.equals("world:outCorridors")){
                @SuppressWarnings("unchecked") List<OutCorridor> actual=(List<OutCorridor>)read(world,"outCorridors");
                actual.clear();if(!value.equals("~"))for(String label:((String)value).split(",")){require(corridors.containsKey(label),"Unknown registry identity");actual.add(corridors.get(label));}
                row("I",stage,target,"List",value);return;
            }
            switch(path[0]){
                case "actor":owner=actors.get(path[1]);name=path[2];break;case "world":owner=world;name=path[1];break;case "corridor":owner=corridors.get(path[1]);name=path[2];break;
                case "connection":owner=connections.get(path[1]);name=path[2];break;case "incoming":owner=incoming.get(path[1]);name=path[2];break;
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
            Field field=owner instanceof RecordingConnection&&!name.equals("failOnSend")?Connection.class.getDeclaredField(name):owner.getClass().getDeclaredField(name);field.setAccessible(true);
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
            if(action.operation.equals("genesis"))guardActiveGenesis();
            if(spec.worldRules&&action.operation.equals("killAll")&&index==1)guardGlobalBaseline();
            if(action.operation.equals("draw")||action.operation.equals("world.draw")||spec.worldRules&&List.of("genesis","killAll","disperseAll").contains(action.operation))snapshot("before-"+stage);
            row("K",stage,action.operation,action.target,action.argument);Throwable observed=null;
            try {
                switch(action.operation){
                    case "connection.new":{
                        require(!connections.containsKey(action.target)&&action.argument.equals("~"),"Fresh recording boundary required");
                        RecordingConnection connection=recordingConnection();connections.put(action.target,connection);connectionNames.put(connection,action.target);break;
                    }
                    case "corridor.new":{
                        require(!corridors.containsKey(action.target)&&connections.containsKey(action.argument),"Fresh outgoing corridor and known recording boundary required");
                        OutCorridor corridor=new OutCorridor(world,connections.get(action.argument));corridors.put(action.target,corridor);corridorNames.put(corridor,action.target);break;
                    }
                    case "incoming.new":{
                        require(!incoming.containsKey(action.target)&&action.argument.equals("~"),"Fresh incoming corridor required");
                        InCorridor corridor=new InCorridor(world);incoming.put(action.target,corridor);incomingNames.put(corridor,action.target);break;
                    }
                    case "incoming.receive":incoming.get(action.target).receiveOrganism(action.argument.equals("~")?null:genomes.get(action.argument));break;
                    case "incoming.frame":require(action.argument.equals("~"),"One actual incoming frame required");incoming.get(action.target).frame();break;
                    case "world.addCorridors":world.addCorridors(incoming.get(action.target),corridors.get(action.argument));break;
                    case "genesis":require(spec.worldRules&&action.argument.equals("~"),"Actual world genesis required");world.genesis();break;
                    case "killAll":require(spec.worldRules&&action.argument.equals("~"),"Actual world killAll required");world.killAll();break;
                    case "disperseAll":require(spec.worldRules&&action.argument.equals("~"),"Actual world disperseAll required");world.disperseAll();break;
                    case "send":row("B",stage,"send",corridors.get(action.target).sendOrganism(actors.get(action.argument)));break;
                    case "canSend":require(action.argument.equals("~"),"No capacity arguments required");row("B",stage,"canSend",corridors.get(action.target).canSendOrganism());break;
                    case "checkHitCorridor":require(action.argument.equals("~"),"No selection arguments required");row("B",stage,"checkHitCorridor",scalar(world.checkHitCorridor(actors.get(action.target))));break;
                    case "move":require(action.argument.equals("~"),"One actual actor move required");row("B",stage,"move",actors.get(action.target).move());break;
                    case "die":require(action.argument.equals("~"),"Explicit null killer required");actors.get(action.target).die(null);break;
                    case "time":require(spec.integration&&action.argument.equals("~")&&action.target.equals("victim"),"One actual integration World.time required");world.time();break;
                    case "draw":case "world.draw":{
                        require(action.argument.equals("~"),"No paint arguments required");
                        BufferedImage image=new BufferedImage(64,64,BufferedImage.TYPE_INT_ARGB);Graphics delegate=image.getGraphics();delegate.setColor(Color.WHITE);
                        RecordingGraphics graphics=new RecordingGraphics(delegate,this,stage,new int[]{0});
                        try{if(action.operation.equals("draw"))corridors.get(action.target).draw(graphics);else world.draw(graphics);}finally{graphics.dispose();}
                        require(!window.isProcessActive(),"Paint observations require the real paused application");break;
                    }
                    default:throw new IllegalArgumentException("Unknown outgoing oracle operation: "+action.operation);
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
    private static final class RecordingGraphics extends Graphics {
        final Graphics delegate;
        final Capture capture;
        final String stage;
        final int[] ordinal;
        RecordingGraphics(Graphics delegate,Capture capture,String stage,int[] ordinal){this.delegate=delegate;this.capture=capture;this.stage=stage;this.ordinal=ordinal;}
        public Graphics create(){return new RecordingGraphics(delegate.create(),capture,stage,ordinal);}
        public void translate(int x,int y){delegate.translate(x,y);}
        public Color getColor(){return delegate.getColor();}
        public void setColor(Color color){delegate.setColor(color);}
        public void setPaintMode(){delegate.setPaintMode();}
        public void setXORMode(Color color){delegate.setXORMode(color);}
        public Font getFont(){return delegate.getFont();}
        public void setFont(Font font){delegate.setFont(font);}
        public FontMetrics getFontMetrics(Font font){return delegate.getFontMetrics(font);}
        public Rectangle getClipBounds(){return delegate.getClipBounds();}
        public void clipRect(int x,int y,int width,int height){delegate.clipRect(x,y,width,height);}
        public void setClip(int x,int y,int width,int height){delegate.setClip(x,y,width,height);}
        public Shape getClip(){return delegate.getClip();}
        public void setClip(Shape shape){delegate.setClip(shape);}
        public void copyArea(int x,int y,int width,int height,int dx,int dy){delegate.copyArea(x,y,width,height,dx,dy);}
        public void drawLine(int x1,int y1,int x2,int y2){capture.row("L",stage,ordinal[0]++,"drawLine",x1,y1,x2,y2,delegate.getColor().getRGB());delegate.drawLine(x1,y1,x2,y2);}
        public void drawRect(int x,int y,int width,int height){capture.row("L",stage,ordinal[0]++,"drawRect",x,y,width,height,delegate.getColor().getRGB());delegate.drawRect(x,y,width,height);}
        public void fillRect(int x,int y,int width,int height){capture.row("L",stage,ordinal[0]++,"fillRect",x,y,width,height,delegate.getColor().getRGB());delegate.fillRect(x,y,width,height);}
        public void clearRect(int x,int y,int width,int height){delegate.clearRect(x,y,width,height);}
        public void drawRoundRect(int x,int y,int width,int height,int aw,int ah){delegate.drawRoundRect(x,y,width,height,aw,ah);}
        public void fillRoundRect(int x,int y,int width,int height,int aw,int ah){delegate.fillRoundRect(x,y,width,height,aw,ah);}
        public void drawOval(int x,int y,int width,int height){delegate.drawOval(x,y,width,height);}
        public void fillOval(int x,int y,int width,int height){delegate.fillOval(x,y,width,height);}
        public void drawArc(int x,int y,int width,int height,int start,int arc){delegate.drawArc(x,y,width,height,start,arc);}
        public void fillArc(int x,int y,int width,int height,int start,int arc){delegate.fillArc(x,y,width,height,start,arc);}
        public void drawPolyline(int[] x,int[] y,int count){delegate.drawPolyline(x,y,count);}
        public void drawPolygon(int[] x,int[] y,int count){delegate.drawPolygon(x,y,count);}
        public void fillPolygon(int[] x,int[] y,int count){delegate.fillPolygon(x,y,count);}
        public void drawString(String text,int x,int y){delegate.drawString(text,x,y);}
        public void drawString(AttributedCharacterIterator text,int x,int y){delegate.drawString(text,x,y);}
        public boolean drawImage(Image image,int x,int y,ImageObserver observer){return delegate.drawImage(image,x,y,observer);}
        public boolean drawImage(Image image,int x,int y,int width,int height,ImageObserver observer){return delegate.drawImage(image,x,y,width,height,observer);}
        public boolean drawImage(Image image,int x,int y,Color background,ImageObserver observer){return delegate.drawImage(image,x,y,background,observer);}
        public boolean drawImage(Image image,int x,int y,int width,int height,Color background,ImageObserver observer){return delegate.drawImage(image,x,y,width,height,background,observer);}
        public boolean drawImage(Image image,int dx1,int dy1,int dx2,int dy2,int sx1,int sy1,int sx2,int sy2,ImageObserver observer){return delegate.drawImage(image,dx1,dy1,dx2,dy2,sx1,sy1,sx2,sy2,observer);}
        public boolean drawImage(Image image,int dx1,int dy1,int dx2,int dy2,int sx1,int sy1,int sx2,int sy2,Color background,ImageObserver observer){return delegate.drawImage(image,dx1,dy1,dx2,dy2,sx1,sy1,sx2,sy2,observer);}
        public void dispose(){delegate.dispose();}
    }
    private static Object readConnection(RecordingConnection owner,String name)throws Exception{Field field=Connection.class.getDeclaredField(name);field.setAccessible(true);return field.get(owner);}
    private static final class RecordingConnection extends Connection {
        List<GeneticCode> recorded;
        boolean failOnSend;
        private RecordingConnection(){super(null,null,0);throw new IllegalStateException("Recording constructor must never execute");}
        @Override public void send(GeneticCode code){recorded.add(code);if(failOnSend)throw new IllegalStateException("recording-send-failure");}
    }
    private static RecordingConnection recordingConnection()throws Exception {
        Class<?> type=Class.forName("sun.misc.Unsafe");Field singleton=type.getDeclaredField("theUnsafe");singleton.setAccessible(true);
        RecordingConnection connection=(RecordingConnection)type.getMethod("allocateInstance",Class.class).invoke(singleton.get(null),RecordingConnection.class);
        require(RecordingConnection.class.getDeclaredMethods().length==1,"Only send(GeneticCode) may override production behavior");
        connection.recorded=new ArrayList<>();connection.state=Connection.STATE_CONNECTED;return connection;
    }
    private static Object read(Object owner,String name)throws Exception{Field field=owner instanceof RecordingConnection&&!name.equals("failOnSend")?Connection.class.getDeclaredField(name):owner.getClass().getDeclaredField(name);field.setAccessible(true);return field.get(owner);}
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
    private static Spec outgoing(String name){Spec spec=scenario(name);spec.add(operation("connection.new","n0","~")).add(operation("corridor.new","c0","n0"));return spec;}
    private static Action send(String actor){return operation("send","c0",actor);}
    private static Action paint(){return operation("draw","c0","~");}
    private static Action attach(Action action,String order){return action.set("world:outCorridors",order).set("world:_corridorexists",true);}
    private static Spec moving(String name){Spec spec=outgoing(name);spec.set("actor:victim:_age",1).set("actor:victim:dx",1d).set("actor:victim:dy",0d).set("actor:victim:dtheta",0d);return spec;}
    private static List<Spec> canonical()throws Exception {
        List<Spec> result=new ArrayList<>();
        result.add(outgoing("real-corridor-constructor-and-empty-capacity").add(operation("canSend","c0","~")));
        Spec alias=outgoing("send-retains-primary-genotype-and-later-mutations");
        alias.add(send("victim")).add(operation("canSend","c0","~")).add(send("killer").set("genome:g0:_generation",73).set("genome:g0:_cladeID","changed:shared").set("genome:g0:gene0._length",19d));result.add(alias);
        result.add(outgoing("existing-colon-clade-is-not-prefixed").add(send("victim").set("genome:g0:_cladeID","already:qualified")));
        result.add(outgoing("empty-clade-receives-explicit-user-prefix").add(send("victim").set("genome:g0:_cladeID","")));
        result.add(outgoing("null-clade-fails-before-send-and-slot-assignment").add(send("victim").set("genome:g0:_cladeID",null).throwsClass("java.lang.NullPointerException")));
        result.add(outgoing("disconnected-empty-slot-reports-capacity-but-rejects-send").add(operation("canSend","c0","~").set("connection:n0:state",50)).add(send("victim")).add(send("victim").set("connection:n0:state",51)));
        result.add(outgoing("not-initialized-connection-rejects-send").add(send("victim").set("connection:n0:state",52)));
        result.add(outgoing("full-traveller-prevents-second-send").add(send("killer")).add(send("victim")));
        result.add(outgoing("recording-send-throws-after-clade-prefix-before-other-effects").add(send("victim").set("connection:n0:failOnSend",true).throwsClass("java.lang.IllegalStateException")).add(send("victim").set("connection:n0:failOnSend",false)));
        result.add(outgoing("scarce-oxygen-leaves-dead-traveller-energy").add(send("victim").set("world:_O2",1d).set("actor:victim:_energy",40d)));
        result.add(outgoing("empty-oxygen-still-sends-and-decrements-population").add(send("victim").set("world:_O2",0d).set("actor:victim:_energy",40d)));
        result.add(outgoing("already-dead-actor-is-sent-with-another-population-decrement").add(operation("die","victim","~")).add(send("victim")));
        result.add(outgoing("zero-energy-still-transfers").add(send("victim").set("actor:victim:_energy",0d)));
        result.add(outgoing("population-decrement-wraps-after-send").add(send("victim").set("world:_population",Integer.MIN_VALUE)));
        result.add(outgoing("paint15-reaches16-before-next-paint-releases-capacity").add(send("victim")).add(paint().set("actor:victim:_growthRatio",15)).add(operation("canSend","c0","~")).add(paint()).add(operation("canSend","c0","~")));
        result.add(outgoing("paint16-releases-without-drawing-or-rotation").add(send("victim")).add(paint().set("actor:victim:_growthRatio",16)));
        result.add(outgoing("paint-ratio-above16-releases-without-rebuild").add(send("victim")).add(paint().set("actor:victim:_growthRatio",17)));
        result.add(outgoing("paused-repeated-paints-progress-without-simulation-ticks").add(send("victim")).add(paint().set("actor:victim:_growthRatio",13)).add(paint()).add(paint()).add(paint()));
        result.add(outgoing("positive-tint-and-unknown-translucent-argb-survive-send").add(send("victim")).add(paint().set("actor:victim:_growthRatio",14).set("actor:victim:_framesColor",2).set("actor:victim:_color",color("TRANSLUCENT")).set("actor:victim:_haseyes",false).set("actor:victim:_segColor",phenotype("UNKNOWN"))));
        Spec inherited=outgoing("null-tint-inherits-corridor-color-before-eye-and-eye-color-afterward");inherited.parent.colors("GREEN","EYE","RED");
        inherited.add(send("victim")).add(paint().set("actor:victim:_growthRatio",14).set("actor:victim:_framesColor",2).set("actor:victim:_color",null).set("actor:victim:_haseyes",true));result.add(inherited);
        Spec tinted=outgoing("dead-eyes-exempt-only-exact-eye-from-temporary-tint");tinted.parent.colors("EYE","GREEN","RED");
        tinted.add(send("victim")).add(paint().set("actor:victim:_growthRatio",14).set("actor:victim:_framesColor",2).set("actor:victim:_color",color("ORANGE")).set("actor:victim:_haseyes",true).set("actor:victim:_segColor",new Color[]{color("EYE"),new Color(color("EYE").getRGB()&0x00ffffff|0x12000000,true),color("UNKNOWN")}));result.add(tinted);
        result.add(outgoing("dead-no-eyes-zero-and-negative-timer-draw-brown").add(send("victim")).add(paint().set("actor:victim:_growthRatio",13).set("actor:victim:_framesColor",0).set("actor:victim:_haseyes",false)).add(paint().set("actor:victim:_framesColor",-1)));
        result.add(outgoing("world-draw-processes-duplicate-corridor-reference-in-list-order").add(send("victim")).add(attach(operation("world.draw","victim","~"),"c0,c0").set("actor:victim:_growthRatio",14)));
        result.add(outgoing("stationary-no-eye-motion-gate-skips-transfer").add(attach(operation("move","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200)).set("actor:victim:_age",1).set("actor:victim:dx",0d).set("actor:victim:dy",0d).set("actor:victim:dtheta",0d).set("actor:victim:_haseyes",false)));
        result.add(moving("ordinary-motion-sends-before-frame-effects-and-breath").add(attach(operation("move","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200))));
        result.add(outgoing("eye-canmove-gate-sends-with-zero-velocity").add(attach(operation("move","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200)).set("actor:victim:_age",1).set("actor:victim:dx",0d).set("actor:victim:dy",0d).set("actor:victim:dtheta",0d).set("actor:victim:_haseyes",true).set("actor:victim:_canmove",true)));
        result.add(moving("inactive-moving-actor-skips-outgoing-transfer").add(attach(operation("move","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200)).set("actor:victim:active",false)));
        Spec full=moving("first-containing-full-corridor-shadows-later-connected-corridor");full.add(operation("connection.new","n1","~")).add(operation("corridor.new","c1","n1")).add(send("killer"));
        full.add(attach(operation("move","victim","~"),"c0,c1").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200)).set("corridor:c1:bounds",new Rectangle(3400,3400,200,200)));result.add(full);
        Spec disconnected=moving("first-containing-disconnected-corridor-shadows-later-connected-corridor");disconnected.add(operation("connection.new","n1","~")).add(operation("corridor.new","c1","n1"));
        disconnected.add(attach(operation("move","victim","~"),"c0,c1").set("connection:n0:state",50).set("corridor:c0:bounds",new Rectangle(3400,3400,200,200)).set("corridor:c1:bounds",new Rectangle(3400,3400,200,200)));result.add(disconnected);
        result.add(moving("border-response-precedes-successful-send-and-skips-rollback").add(attach(operation("move","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(6990,3400,200,200)).set("actor:victim:_centerX",6999).set("actor:victim:_dCenterX",6999d).set("actor:victim:dx",3d)));
        Spec edges=outgoing("contains-lower-upper-empty-and-wrapped-integer-edges");
        edges.add(attach(operation("checkHitCorridor","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(3500,3500,1,1)))
            .add(operation("checkHitCorridor","victim","~").set("actor:victim:_centerX",3501))
            .add(operation("checkHitCorridor","victim","~").set("corridor:c0:bounds",new Rectangle(3500,3500,0,10)).set("actor:victim:_centerX",3500))
            .add(operation("checkHitCorridor","victim","~").set("corridor:c0:bounds",new Rectangle(Integer.MAX_VALUE-2,3500,8,1)).set("actor:victim:_centerX",Integer.MAX_VALUE))
            .add(operation("checkHitCorridor","victim","~").set("actor:victim:_centerX",Integer.MIN_VALUE));result.add(edges);
        result.add(moving("dead-active-moving-actor-still-transfers").add(operation("die","victim","~")).add(attach(operation("move","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200))));
        require(result.stream().map(spec->spec.name).distinct().count()==result.size(),"Unique outgoing cases required");return result;
    }
    private static List<Spec> integration()throws Exception {
        List<Spec> result=new ArrayList<>();
        Spec retained=moving("frame-removal-retains-traveller-and-current-frozen-tree-identity");retained.integration=true;
        retained.add(attach(operation("time","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200)).set("actor:victim:_growthRatio",15));
        retained.add(operation("time","victim","~")).add(operation("time","victim","~")).add(paint()).add(paint());result.add(retained);
        Spec two=moving("two-frame-transfers-and-paused-world-paints-release-both-slots");two.integration=true;
        two.add(operation("connection.new","n1","~")).add(operation("corridor.new","c1","n1"));
        two.add(attach(operation("time","victim","~"),"c0,c1").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200)).set("corridor:c1:bounds",new Rectangle(4900,4900,200,200)).set("actor:victim:_growthRatio",15).set("actor:killer:_growthRatio",15).set("actor:killer:_age",1).set("actor:killer:dx",1d));
        two.add(operation("time","victim","~")).add(operation("world.draw","victim","~")).add(operation("world.draw","victim","~"));result.add(two);
        Spec contact=moving("later-member-queries-removed-traveller-in-the-same-frozen-tree");contact.integration=true;
        contact.add(attach(operation("time","victim","~"),"c0").set("corridor:c0:bounds",new Rectangle(3500,3500,1,1)).set("actor:victim:dx",0d).set("actor:victim:dtheta",0.01d)
            .set("actor:killer:_centerX",3501).set("actor:killer:_dCenterX",3501d).set("actor:killer:_centerY",3500).set("actor:killer:_dCenterY",3500d).set("actor:killer:_age",1).set("actor:killer:dx",0d).set("actor:killer:dtheta",0.01d));
        contact.add(paint().set("actor:victim:_growthRatio",15));result.add(contact);
        return result;
    }
    private static List<Spec> worldRules()throws Exception {
        List<Spec> result=new ArrayList<>();
        Spec active=moving("active-pair-genesis-retains-queue-candidate-traveller");
        active.worldRules=true;active.integration=true;active.roles=new String[]{"selector","victim","blocker"};
        active.setting("INITIAL_ORGANISMS",2).setting("WORLD_WIDTH",7000).setting("WORLD_HEIGHT",7000);
        active.parent.colors("EYE","GREEN");active.parent.branches=new int[]{0,-1};
        active.set("actor:victim:_growthRatio",15);
        active.add(operation("incoming.new","i0","~")).add(operation("world.addCorridors","i0","c0"));
        active.add(operation("incoming.receive","i0","g0")).add(operation("incoming.receive","i0","g0")).add(operation("incoming.receive","i0","g4"));
        active.add(operation("incoming.frame","i0","~").set("incoming:i0:bounds",new Rectangle(3500,3500,1,1)).set("actor:blocker:bounds",new Rectangle(3400,3400,200,200)));
        active.add(operation("time","victim","~").set("corridor:c0:bounds",new Rectangle(3400,3400,200,200)));
        active.add(operation("genesis","victim","~")).add(operation("time","victim","~")).add(operation("time","victim","~")).add(operation("world.draw","victim","~"));
        result.add(active);
        String[] names={"sufficient-o2","partial-o2","zero-o2"};double[] oxygen={64d,5d,0d};
        for(int index=0;index<names.length;index++){
            Spec global=scenario("global-mixed-"+names[index]);global.worldRules=true;global.integration=true;
            global.setting("WORLD_WIDTH",7000).setting("WORLD_HEIGHT",7000);
            global.add(operation("die","selector","~"));
            global.add(operation("killAll","victim","~").set("actor:selector:_energy",8d).set("actor:victim:_energy",13d).set("actor:killer:_energy",21d).set("world:_O2",oxygen[index]));
            global.add(operation("killAll","victim","~")).add(operation("disperseAll","victim","~")).add(operation("disperseAll","victim","~")).add(operation("time","victim","~"));
            result.add(global);
        }
        require(result.size()==4&&result.stream().map(spec->spec.name).distinct().count()==4,"Four fixed world-rule recipes required");return result;
    }
    private static String captureWorldRules()throws Exception {
        StringBuilder output=new StringBuilder();for(Spec spec:worldRules()){
            Capture capture=new Capture(spec);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index));output.append(capture.finish());
        }return output.toString();
    }
    private static String capture(boolean frames)throws Exception {
        StringBuilder output=new StringBuilder();for(Spec spec:frames?integration():canonical()){
            Capture capture=new Capture(spec);for(int index=0;index<spec.actions.size();index++)capture.action(index,spec.actions.get(index));output.append(capture.finish());
        }return output.toString();
    }
    public static void main(String[]args)throws Exception {
        boolean worldRulesOnly=args.length==2&&args[1].equals("--world-rules");
        if(args.length!=1&&!worldRulesOnly)throw new IllegalArgumentException("Usage: biogenesis.OutgoingOracle NEW_OUTPUT_DIRECTORY [--world-rules]");
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
            SwingUtilities.invokeAndWait(()->{Utils.random.setSeed(827364);Utils.readPreferences();window=new Main();});
            for(Frame frame:Frame.getFrames())if(frame instanceof MainWindow main)window=main;
            require(window!=null&&!window.isProcessActive()&&window.serverThread==null&&window._timer==null,"Actual paused window without timer or network server required");
            if(worldRulesOnly){
                FutureTask<String> measurement=new FutureTask<>(()->captureWorldRules());SwingUtilities.invokeAndWait(measurement);
                Files.writeString(directory.resolve("world_rules.tsv"),measurement.get(),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
            } else {
            FutureTask<String> measurement=new FutureTask<>(()->capture(false));SwingUtilities.invokeAndWait(measurement);
            Files.writeString(directory.resolve("outgoing.tsv"),measurement.get(),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
            FutureTask<String> diagnostics=new FutureTask<>(()->capture(true));SwingUtilities.invokeAndWait(diagnostics);
            Files.writeString(directory.resolve("outgoing_frames.tsv"),diagnostics.get(),StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
            }
            Files.writeString(directory.resolve("ui.tsv"),uiRows,StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
            Files.writeString(directory.resolve("bootstrap.json"),"{\"main_window\":\""+window.getClass().getName()+"\",\"info_toolbar\":\""+window.getInfoPanel().getClass().getName()+"\",\"paused\":true,\"timer_cancelled\":false,\"timer_absent\":true,\"owned_display\":\""+System.getenv("DISPLAY")+"\",\"cases\":"+cases+"}\n",StandardCharsets.UTF_8,java.nio.file.StandardOpenOption.CREATE_NEW);
            System.out.println("OUTGOING_ORACLE PASS cases="+cases+" snapshots="+snapshots+" calls="+calls+" discoveries="+newborns);
        } catch(Throwable error){failure=error;status=1;}
        finally {
            try {
                SwingUtilities.invokeAndWait(()->{for(Frame frame:Frame.getFrames())if(frame instanceof MainWindow main&&main._timer!=null)main._timer.cancel();for(Window target:Window.getWindows())target.dispose();});
            } catch(Throwable cleanup){status=1;if(failure==null)failure=cleanup;else failure.addSuppressed(cleanup);}
            finally {if(failure!=null)failure.printStackTrace();System.exit(status);}
        }
    }
}
