// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Frame;
import java.awt.Window;
import java.io.*;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.FutureTask;
import java.util.prefs.Preferences;
import javax.swing.SwingUtilities;

/** Observe compact actual workload scalar and generator histories through the real paused application */
public final class WorkloadTrajectoryOracle {
    private static final long SEED=827364L;
    private static final Map<Integer,String> ENDPOINTS=Map.of(
        0,"76b907d1415929f633e4f988d83b71b78d2ac6dfa23124264a94f20760a0a39d",
        32,"723d4d4dffcadd255b636102cc758aef9c1ca1bfe5355d43a3cfdf367efd5a3e",
        512,"ee9e144c5ad61e81ee9b8e43eea2e6484f80cbe6607ec36ed561549f845e597f",
        12500,"ad5f4e3b9257591c6a5055c3de1d472c4c15da3858018b706ed8125d767c383d");
    private static MainWindow window;
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static String bits(double value){return String.format(Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static String scalar(Object value){return value instanceof Double number?bits(number):value.toString();}
    private static void row(StringBuilder output,Object...values){for(int i=0;i<values.length;i++){if(i>0)output.append('\t');output.append(values[i]);}output.append('\n');require(output.length()<=2*1024*1024,"Compact output exceeds2MiB");}
    private static Field[] primitiveFields(Class<?> type,boolean globals){
        return Arrays.stream(type.getDeclaredFields()).filter(field->field.getType().isPrimitive()
            &&Modifier.isStatic(field.getModifiers())==globals&&(!globals||!Modifier.isFinal(field.getModifiers())))
            .peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);
    }
    private static String omittedFields(Class<?> type){return String.join(",",Arrays.stream(type.getDeclaredFields())
        .filter(field->!Modifier.isStatic(field.getModifiers())&&!field.getType().isPrimitive()).map(Field::getName).sorted().toArray(String[]::new));}
    private static byte[] serialized(Random original)throws Exception{
        require(original.getClass()==Random.class,"Only the original exact Random class is permitted");
        ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream stream=new ObjectOutputStream(bytes)){stream.writeObject(original);}
        require(bytes.size()<=1024,"Unexpected Random serialization size");return bytes.toByteArray();
    }
    private static final class RandomInput extends ObjectInputStream {
        long seed;
        double gaussian;
        boolean haveGaussian,captured;
        RandomInput(byte[] bytes)throws IOException{
            super(new ByteArrayInputStream(bytes));setObjectInputFilter(info->{
                if(info.depth()>4||info.references()>16||info.streamBytes()>1024||info.arrayLength()>0)return ObjectInputFilter.Status.REJECTED;
                return info.serialClass()==null?ObjectInputFilter.Status.UNDECIDED:
                    info.serialClass()==Random.class?ObjectInputFilter.Status.ALLOWED:ObjectInputFilter.Status.REJECTED;
            });
        }
        @Override public GetField readFields()throws IOException,ClassNotFoundException{
            GetField fields=super.readFields();require(!captured&&fields.getObjectStreamClass().forClass()==Random.class,"Unexpected Random field owner");
            require(!fields.defaulted("seed")&&!fields.defaulted("nextNextGaussian")&&!fields.defaulted("haveNextNextGaussian"),"Random fields must come from actual serialized state");
            seed=fields.get("seed",-1L);gaussian=fields.get("nextNextGaussian",0d);haveGaussian=fields.get("haveNextNextGaussian",false);captured=true;
            require(seed>=0&&seed<(1L<<48),"Original Random state must contain48bits");return fields;
        }
        @Override protected Class<?> resolveClass(ObjectStreamClass descriptor)throws IOException,ClassNotFoundException{
            require(descriptor.getName().equals("java.util.Random"),"Only an actual serialized Random is permitted");return super.resolveClass(descriptor);
        }
        @Override protected Class<?> resolveProxyClass(String[] interfaces)throws IOException{throw new InvalidClassException("Proxy classes are not permitted");}
    }
    private static Map<String,String> scalarState(World world,Field[] worldFields,Field[] statsFields)throws Exception{
        Map<String,String> values=new TreeMap<>();for(Field field:worldFields)values.put("V:"+field.getName(),scalar(field.get(world)));
        for(Field field:statsFields)values.put("S:"+field.getName(),scalar(field.get(world.worldStatistics)));
        values.put("W:_population",Integer.toString(world.getPopulation()));values.put("W:organism_count",Integer.toString(world._organisms.size()));
        values.put("W:corpses",Integer.toString(world.getNCorpses()));values.put("W:_O2",bits(world.getO2()));values.put("W:_CO2",bits(world.getCO2()));values.put("W:_CH4",bits(world.getCH4()));
        values.put("W:NEXT_ID",Integer.toString(world.NEXT_ID));values.put("W:nFrames",Long.toString(world.getFrame()));values.put("W:time",Long.toString(world.getTime()));return values;
    }
    private static void snapshot(World world,int frame,StringBuilder output,Field[] worldFields,Field[] statsFields)throws Exception{
        require(world.getFrame()==frame%256&&world.getTime()==frame/256,"Paused workload clock or frame order differs");
        Random original=Utils.random;WorldStatistics statistics=world.worldStatistics;List<Organism> members=world._organisms;
        byte[] before=serialized(original);Map<String,String> state=scalarState(world,worldFields,statsFields);
        for(Field field:worldFields)row(output,"V",frame,field.getName(),field.getType().getSimpleName(),state.get("V:"+field.getName()));
        for(Field field:statsFields)row(output,"S",frame,field.getName(),field.getType().getSimpleName(),state.get("S:"+field.getName()));
        for(String name:new String[]{"_population","organism_count","corpses","_O2","_CO2","_CH4","NEXT_ID","nFrames","time"})
            row(output,"W",frame,name,List.of("_O2","_CO2","_CH4").contains(name)?"double":List.of("nFrames","time").contains(name)?"long":"int",state.get("W:"+name));
        try(RandomInput stream=new RandomInput(before)){
            Object value=stream.readObject();require(value instanceof Random&&stream.captured&&stream.read()==-1,"Complete real Random clone required");Random copy=(Random)value;
            require(Arrays.equals(before,serialized(copy)),"Copied Random must preserve actual serialized state before suffix observation");
            row(output,"R",frame,stream.seed,stream.haveGaussian,bits(stream.gaussian),copy.nextLong(),copy.nextInt(10000),bits(copy.nextDouble()));
        }
        require(Utils.random==original&&world.worldStatistics==statistics&&world._organisms==members,"Observer replaced original state owners");
        require(Arrays.equals(before,serialized(original)),"Observer consumed or changed the original Random");
        require(state.equals(scalarState(world,worldFields,statsFields)),"Observer changed original World or statistics scalars");
        row(output,"Q",frame,"random_serialization_pure",true,"world_statistics_scalars_pure",true);
    }
    private static final class DigestOutput extends OutputStream {
        final MessageDigest digest;
        final OutputStream destination;
        long bytes;
        DigestOutput()throws Exception{this(null);}
        DigestOutput(OutputStream destination)throws Exception{digest=MessageDigest.getInstance("SHA-256");this.destination=destination;}
        @Override public void write(int value)throws IOException{
            if(destination!=null){require(bytes<(1L<<30),"BGW exceeds1GiB capture file cap");destination.write(value);}
            digest.update((byte)value);bytes++;
        }
        @Override public void write(byte[] value,int offset,int length)throws IOException{
            if(destination!=null){require(length>=0&&length<=(1L<<30)-bytes,"BGW exceeds1GiB capture file cap");destination.write(value,offset,length);}
            digest.update(value,offset,length);bytes+=length;
        }
        @Override public void close()throws IOException{if(destination!=null)destination.close();}
    }
    private static String observe(int attempts,Path output,boolean evolving512,boolean dense256)throws Exception{
        require(window!=null&&!window.isProcessActive()&&window.serverThread==null,"Actual paused offline MainWindow required");
        World world=window.getWorld();require(window.getInfoPanel()!=null&&world._visibleWorld._mainWindow==window,"Real original UI collaborators required");
        require(world.getFrame()==0&&!world._corridorexists&&world.inCorridors.isEmpty()&&world.outCorridors.isEmpty(),"Expected fresh ordinary workload genesis");
        if(window._timer!=null)window._timer.cancel();
        Field[] globals=primitiveFields(Utils.class,true),worldFields=primitiveFields(World.class,false),statsFields=primitiveFields(WorldStatistics.class,false);
        require(globals.length==147&&worldFields.length==9&&statsFields.length==37,"Primitive source schema drift");
        StringBuilder rows=new StringBuilder();row(rows,"C","workload-"+attempts,attempts,SEED,7000,7000,16,dense256?240:evolving512?496:64);
        for(Object[] preference:new Object[][]{{"VERSION","int",700},{"INITIAL_ORGANISMS","int",attempts},{"WORLD_WIDTH","int",7000},{"WORLD_HEIGHT","int",7000},{"ACCEPT_CONNECTIONS","boolean",false},{"CONNECT_TO_SERVER","boolean",false},{"AUTO_BACKUP","boolean",false},{"HARDWARE_ACCELERATION","int",0},{"DELAY","int",20},{"LOCALE","String","en"}})
            row(rows,"P",preference[0],preference[1],preference[2]);
        for(Field field:globals)row(rows,"U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
        row(rows,"X","World","nonprimitive_fields",omittedFields(World.class));row(rows,"X","WorldStatistics","nonprimitive_fields",omittedFields(WorldStatistics.class));
        snapshot(world,0,rows,worldFields,statsFields);
        String endpoint80=null;long endpoint80Bytes=0;
        for(int frame=1;frame<=16;frame++){row(rows,"K",frame,"time");world.time();snapshot(world,frame,rows,worldFields,statsFields);}
        for(int frame=17;frame<=(dense256?256:evolving512?512:80);frame++){
            row(rows,"K",frame,"time");world.time();snapshot(world,frame,rows,worldFields,statsFields);
            if((evolving512||dense256)&&frame==80){
                byte[] randomBefore=serialized(Utils.random);Map<String,String> stateBefore=scalarState(world,worldFields,statsFields);
                DigestOutput witness=new DigestOutput();try(ObjectOutputStream stream=new ObjectOutputStream(witness)){stream.writeObject(world);}
                endpoint80=HexFormat.of().formatHex(witness.digest.digest());endpoint80Bytes=witness.bytes;
                require(endpoint80.equals(ENDPOINTS.get(attempts)),"Observed frame80 graph differs from the pinned original workload");
                require(Arrays.equals(randomBefore,serialized(Utils.random))&&stateBefore.equals(scalarState(world,worldFields,statsFields)),"Frame80 serialization changed original scalar or Random state");
            }
        }
        require(!window.isProcessActive()&&window.serverThread==null,"Workload must remain paused and offline");row(rows,"E");
        Path graph=dense256?output.resolve("world-after-256.bgw"):null;
        DigestOutput sink=dense256?new DigestOutput(Files.newOutputStream(graph,StandardOpenOption.CREATE_NEW)):new DigestOutput();try(ObjectOutputStream stream=new ObjectOutputStream(sink)){stream.writeObject(world);}
        String endpoint=HexFormat.of().formatHex(sink.digest.digest());
        if(dense256)require(Files.size(graph)==sink.bytes,"Streamed BGW byte count differs from file size");
        if(evolving512||dense256)require(endpoint80!=null,"Frame80 witness missing");
        else require(endpoint.equals(ENDPOINTS.get(attempts)),"Observed frame80 graph differs from the pinned original workload");
        Files.writeString(output.resolve("trajectory.tsv"),rows,StandardCharsets.US_ASCII,StandardOpenOption.CREATE_NEW);
        if(dense256){
        Files.writeString(output.resolve("bootstrap.json"),"{\"main_window\":\""+window.getClass().getName()+"\",\"info_toolbar\":\""+window.getInfoPanel().getClass().getName()+"\",\"paused\":true,\"timer_cancelled\":true,\"owned_display\":\""+System.getenv("DISPLAY")+"\",\"endpoint80_sha256\":\""+endpoint80+"\",\"endpoint80_bytes\":"+endpoint80Bytes+",\"endpoint256_sha256\":\""+endpoint+"\",\"endpoint256_bytes\":"+sink.bytes+",\"ticks\":256,\"snapshots\":257,\"final_frame\":"+world.getFrame()+",\"final_time\":"+world.getTime()+",\"witness_scalar_random_pure\":true,\"seed_cache_extraction\":\"actualRandom.readFields-on-clone; no module overrides\"}\n",StandardCharsets.US_ASCII,StandardOpenOption.CREATE_NEW);
        }else if(evolving512){
        Files.writeString(output.resolve("bootstrap.json"),"{\"main_window\":\""+window.getClass().getName()+"\",\"info_toolbar\":\""+window.getInfoPanel().getClass().getName()+"\",\"paused\":true,\"timer_cancelled\":true,\"owned_display\":\""+System.getenv("DISPLAY")+"\",\"endpoint80_sha256\":\""+endpoint80+"\",\"endpoint80_bytes\":"+endpoint80Bytes+",\"endpoint512_sha256\":\""+endpoint+"\",\"endpoint512_bytes\":"+sink.bytes+",\"ticks\":512,\"snapshots\":513,\"final_frame\":"+world.getFrame()+",\"final_time\":"+world.getTime()+",\"witness_scalar_random_pure\":true,\"seed_cache_extraction\":\"actualRandom.readFields-on-clone; no module overrides\"}\n",StandardCharsets.US_ASCII,StandardOpenOption.CREATE_NEW);
        }else{
        Files.writeString(output.resolve("bootstrap.json"),"{\"main_window\":\""+window.getClass().getName()+"\",\"info_toolbar\":\""+window.getInfoPanel().getClass().getName()+"\",\"paused\":true,\"timer_cancelled\":true,\"owned_display\":\""+System.getenv("DISPLAY")+"\",\"endpoint80_sha256\":\""+endpoint+"\",\"endpoint80_bytes\":"+sink.bytes+",\"seed_cache_extraction\":\"actualRandom.readFields-on-clone; no module overrides\"}\n",StandardCharsets.US_ASCII,StandardOpenOption.CREATE_NEW);
        }
        if(dense256)return "WORKLOAD_TRAJECTORY PASS attempts="+attempts+" snapshots=257 calls=256 endpoint256="+endpoint;
        if(evolving512)return "WORKLOAD_TRAJECTORY PASS attempts="+attempts+" snapshots=513 calls=512 endpoint512="+endpoint;
        return "WORKLOAD_TRAJECTORY PASS attempts="+attempts+" snapshots=81 calls=80 endpoint="+endpoint;
    }
    public static void main(String[]args)throws Exception{
        boolean evolving512=args.length==3&&args[2].equals("--evolving-512");
        boolean dense256=args.length==3&&args[2].equals("--dense-256");
        require(args.length==2||evolving512||dense256,"Usage: biogenesis.WorkloadTrajectoryOracle ATTEMPTS NEW_OUTPUT_DIRECTORY [--evolving-512|--dense-256]");
        int attempts=Integer.parseInt(args[0]);require(ENDPOINTS.containsKey(attempts),"Unspecified original workload cohort");
        require(!evolving512||attempts==512,"The evolving512 mode requires512 initial attempts");
        require(!dense256||attempts==12500,"The dense256 mode requires12500 initial attempts");
        Path output=Path.of(args[1]).toAbsolutePath().normalize(),parent=output.getParent().toRealPath();
        for(String key:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(key);require(value!=null&&Path.of(value).toRealPath().startsWith(parent)&&!Path.of(value).toRealPath().equals(parent),"Explicit isolated profile required: "+key);
        }
        require(!Files.exists(output),"Output directory must be new");
        require("1".equals(System.getenv("BGR_ISOLATED_XVFB"))&&System.getenv("DISPLAY")!=null&&System.getenv("DISPLAY").matches(":[1-9][0-9]{2,}"),"Owned isolated Xvfb required");
        Files.createDirectory(output);Throwable failure=null;int status=0;
        try{
            Preferences preferences=Preferences.userNodeForPackage(Utils.class);
            preferences.putInt("VERSION",700);preferences.putInt("INITIAL_ORGANISMS",attempts);preferences.putInt("WORLD_WIDTH",7000);preferences.putInt("WORLD_HEIGHT",7000);
            preferences.putBoolean("ACCEPT_CONNECTIONS",false);preferences.putBoolean("CONNECT_TO_SERVER",false);preferences.putBoolean("AUTO_BACKUP",false);
            preferences.putInt("HARDWARE_ACCELERATION",0);preferences.putInt("DELAY",20);preferences.put("LOCALE","en");preferences.flush();
            SwingUtilities.invokeAndWait(()->MainWindow.main(new String[]{"827364"}));
            for(Frame frame:Frame.getFrames())if(frame instanceof MainWindow main){require(window==null,"Exactly one original MainWindow required");window=main;}
            FutureTask<String> capture=new FutureTask<>(()->observe(attempts,output,evolving512,dense256));SwingUtilities.invokeAndWait(capture);System.out.println(capture.get());
        }catch(Throwable error){failure=error;status=1;}
        finally{
            try{SwingUtilities.invokeAndWait(()->{for(Frame frame:Frame.getFrames())if(frame instanceof MainWindow main&&main._timer!=null)main._timer.cancel();for(Window target:Window.getWindows())target.dispose();});}
            catch(Throwable cleanup){status=1;if(failure==null)failure=cleanup;else failure.addSuppressed(cleanup);}
            finally{if(failure!=null)failure.printStackTrace();System.exit(status);}
        }
    }
}
