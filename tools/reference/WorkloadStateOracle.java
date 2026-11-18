// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Color;
import java.awt.Rectangle;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.regex.*;
import org.locationtech.jts.geom.Envelope;
import org.locationtech.jts.index.strtree.*;

/** Decode only pinned workload endpoints and expose their complete logical object graphs */
public final class WorkloadStateOracle {
    private static final long MAX_BYTES=36_000_000,MAX_REFERENCES=4_000_000,MAX_ARRAY=20_000,MAX_OUTPUT=600L*1024*1024;
    private static final long DENSE256_BYTES=36_074_003;
    private static final String DENSE256_SHA256="f912ec8932a482283696fd5265878b96bc8439fc8d395dbcf73064639e8a5598";
    private static final Set<String> HISTORIES=Set.of("populationList","birthList","deathList","oxygenList","carbonDioxideList","methaneList");
    private static final Set<String> CLASSES=Set.of("biogenesis.World","biogenesis.Organism","biogenesis.GeneticCode","biogenesis.Gene","biogenesis.WorldStatistics","java.awt.Rectangle","java.awt.Color","java.util.Collections$SynchronizedCollection","java.util.Collections$SynchronizedList","java.util.ArrayList","org.locationtech.jts.index.strtree.AbstractSTRtree","org.locationtech.jts.index.strtree.STRtree","org.locationtech.jts.index.strtree.AbstractNode","org.locationtech.jts.index.strtree.STRtree$STRtreeNode","org.locationtech.jts.index.strtree.ItemBoundable","org.locationtech.jts.geom.Envelope","[I","[D","[Ljava.awt.Color;","[Lbiogenesis.Gene;");
    private static final Map<Integer,String> INPUTS=Map.of(0,"76b907d1415929f633e4f988d83b71b78d2ac6dfa23124264a94f20760a0a39d",32,"723d4d4dffcadd255b636102cc758aef9c1ca1bfe5355d43a3cfdf367efd5a3e",512,"ee9e144c5ad61e81ee9b8e43eea2e6484f80cbe6607ec36ed561549f845e597f",12500,"ad5f4e3b9257591c6a5055c3de1d472c4c15da3858018b706ed8125d767c383d");
    private static final Map<Integer,String> PREFS=Map.of(0,"6e5376b1bcd8096072de1c0f6d1582de925b5c8437a5ecc80a079f840b0a8cdb",32,"4fa6f778ccbc5448b2e504f08cf0fef3996d78dd3773b5d98d07069c40da643e",512,"52972d1204cac9c90d9de31bc20a7da8e6810de6923f80d00a106dfa3b4461ec",12500,"38dea666061cf85aff216877349d133186f2bb040bb7800e4c4d9695b8d6c31d");
    private static final class FilteredInput extends ObjectInputStream {
        long depth,references,array,bytes;
        final boolean dense256;
        final long maxBytes;
        final Set<String> descriptors=new TreeSet<>();
        FilteredInput(InputStream input)throws IOException {this(input,false);}
        FilteredInput(InputStream input,boolean dense256)throws IOException {
            super(input);this.dense256=dense256;maxBytes=dense256?DENSE256_BYTES:MAX_BYTES;setObjectInputFilter(info->{
                depth=Math.max(depth,info.depth());references=Math.max(references,info.references());array=Math.max(array,info.arrayLength());bytes=Math.max(bytes,info.streamBytes());
                if(info.depth()>64||info.references()>MAX_REFERENCES||info.arrayLength()>MAX_ARRAY||info.streamBytes()>maxBytes)return ObjectInputFilter.Status.REJECTED;
                Class<?> type=info.serialClass();if(type==null)return ObjectInputFilter.Status.UNDECIDED;
                String name=type.getName();return admitted(name)||name.equals("[Ljava.lang.Object;")||name.equals("java.util.Collections$SynchronizedRandomAccessList")?ObjectInputFilter.Status.ALLOWED:ObjectInputFilter.Status.REJECTED;
            });
        }
        boolean numeric(String name){return name.equals("java.lang.Double")||name.equals("java.lang.Number");}
        boolean admitted(String name){return CLASSES.contains(name)||(dense256&&numeric(name));}
        void numericSchema(ObjectStreamClass descriptor){
            boolean boxed=descriptor.getName().equals("java.lang.Double");
            require(descriptor.getSerialVersionUID()==(boxed?-9172774392245257468L:-8742448824652078965L),"Numeric serial UID differs");
            ObjectStreamField[] fields=descriptor.getFields();require(fields.length==(boxed?1:0),"Numeric field count differs");
            if(boxed)require(fields[0].getName().equals("value")&&fields[0].getTypeCode()=='D'&&fields[0].getTypeString()==null,"Numeric value schema differs");
        }
        protected Class<?> resolveClass(ObjectStreamClass descriptor)throws IOException,ClassNotFoundException {
            String name=descriptor.getName();require(admitted(name),"Unapproved serialized class: "+name);
            if(dense256&&numeric(name))numericSchema(descriptor);
            Class<?> type=super.resolveClass(descriptor);
            if(dense256&&numeric(name)){
                require(type==(name.equals("java.lang.Double")?Double.class:Number.class),"Numeric runtime class differs");
                require(type.getSuperclass()==(type==Double.class?Number.class:Object.class),"Numeric superclass differs");
                numericSchema(ObjectStreamClass.lookup(type));
            }
            descriptors.add(name);return type;
        }
        protected Class<?> resolveProxyClass(String[] names)throws IOException {throw new InvalidClassException("Proxy classes are not allowed");}
    }
    private static final class Entry {
        final Organism actor;final Envelope envelope;
        Entry(Organism actor,Envelope envelope){this.actor=actor;this.envelope=envelope;}
    }
    private static final class Capture implements AutoCloseable {
        final BufferedWriter writer;
        final World world;
        final boolean dense256;
        final Set<String> historyOwners=new HashSet<>();
        final List<Entry> tree;
        final List<Organism> actors=new ArrayList<>();
        final List<GeneticCode> genomes=new ArrayList<>();
        final IdentityHashMap<Object,String> identities=new IdentityHashMap<>();
        final IdentityHashMap<Object,Boolean> containers=new IdentityHashMap<>();
        final IdentityHashMap<Gene,Boolean> geneAliases=new IdentityHashMap<>();
        final Map<String,Integer> kindCounts=new TreeMap<>(),rowCounts=new TreeMap<>();
        long outputBytes;
        Capture(World world,Path output)throws Exception {this(world,output,false);}
        Capture(World world,Path output,boolean dense256)throws Exception {
            this.dense256=dense256;this.world=world;tree=stored(world.colDetTree);identities.put(world,"w0");identities.put(world.worldStatistics,"s0");identities.put(world.colDetTree,"t0");
            for(Organism actor:world._organisms)actor(actor);
            for(Entry entry:tree)actor(entry.actor);
            for(Organism actor:actors){genome(actor._geneticCode);genome(actor._infectedGeneticCode);genome(actor._savedGeneticCode);}
            for(Field field:fields(WorldStatistics.class))if(field.getType()==GeneticCode.class)genome((GeneticCode)field.get(world.worldStatistics));
            for(GeneticCode genome:genomes)for(int index=0;index<genome.getNGenes();index++)identity(genome.getGene(index));
            writer=Files.newBufferedWriter(output,StandardCharsets.US_ASCII,StandardOpenOption.CREATE_NEW);
        }
        String actor(Organism actor){String label=identities.get(actor);if(label==null){label="o"+actors.size();identities.put(actor,label);actors.add(actor);}return label;}
        String genome(GeneticCode code){if(code==null)return "~";String label=identities.get(code);if(label==null){label="g"+genomes.size();identities.put(code,label);genomes.add(code);}return label;}
        String kind(Object value){
            if(value instanceof Organism)return "Organism";if(value instanceof World)return "World";if(value instanceof GeneticCode)return "GeneticCode";if(value instanceof Gene)return "Gene";if(value instanceof WorldStatistics)return "WorldStatistics";if(value instanceof STRtree)return "STRtree";if(value instanceof Rectangle)return "Rectangle";if(value instanceof Color)return "Color";if(value instanceof String)return "String";if(dense256&&value instanceof Double)return "Double";if(value instanceof List<?>)return "List";if(value.getClass().isArray())return value.getClass().getComponentType().getSimpleName()+"[]";throw new IllegalStateException("Unknown reference kind: "+value.getClass().getName());
        }
        String identity(Object value){
            String label=identities.get(value);if(label!=null)return label;
            String kind=kind(value);String prefix=switch(kind){case "Gene"->"n";case "Color"->"c";case "String"->"q";case "Double"->"d";case "Rectangle"->"b";case "List"->"l";default->{require(value.getClass().isArray(),"Unexpected undiscovered source reference");yield "a";}};
            int count=kindCounts.merge(prefix,1,Integer::sum)-1;label=prefix+count;identities.put(value,label);return label;
        }
        void alias(String owner,String label,String field,Object value)throws Exception {
            if(value==null)return;String kind=kind(value),reference=identity(value);boolean immutable=value instanceof Color||value instanceof String||value instanceof Double;
            row("A",owner,label,field,kind,reference,immutable?"immutable":"mutable");
            if((value instanceof List<?>||value.getClass().isArray())&&containers.put(value,true)==null){
                if(value instanceof List<?> list){for(int index=0;index<list.size();index++)alias("container",reference,Integer.toString(index),list.get(index));}
                else if(!value.getClass().getComponentType().isPrimitive())for(int index=0;index<Array.getLength(value);index++)alias("container",reference,Integer.toString(index),Array.get(value,index));
            }
        }
        void history(Object value)throws Exception {
            if(value==null)return;require(value instanceof List<?>,"History must retain its actual List");
            List<?> list=(List<?>)value;String owner=identity(list);if(!historyOwners.add(owner))return;
            row("L","endpoint",owner,list.getClass().getName(),list.size());
            for(int index=0;index<list.size();index++){
                Object box=list.get(index);require(box==null||box.getClass()==Double.class,"History element must retain its boxed Double");
                row("D","endpoint",owner,index,box==null?"~":identity(box),box==null?"~":bits((Double)box));
            }
        }
        void row(Object...values)throws IOException {
            StringBuilder text=new StringBuilder();for(int index=0;index<values.length;index++){if(index!=0)text.append('\t');text.append(values[index]);}text.append('\n');
            for(int index=0;index<text.length();index++)require(text.charAt(index)<128,"Canonical endpoint output must remain ASCII");outputBytes+=text.length();require(outputBytes<=MAX_OUTPUT,"Endpoint output exceeds the fixed byte bound");writer.write(text.toString());rowCounts.merge(values[0].toString(),1,Integer::sum);
        }
        String scalar(Object value)throws Exception {
            if(value==null)return "~";if(value instanceof Double number)return bits(number);if(value instanceof Color color)return String.format(Locale.ROOT,"%08x",color.getRGB());
            if(value instanceof Organism||value instanceof GeneticCode||value instanceof Gene||value instanceof World||value instanceof WorldStatistics)return identity(value);
            if(value instanceof Rectangle rectangle)return rectangle.x+","+rectangle.y+","+rectangle.width+","+rectangle.height;
            if(value instanceof STRtree)return "present,"+tree.size();
            if(value instanceof List<?> list){StringBuilder result=new StringBuilder();for(int index=0;index<list.size();index++){if(index!=0)result.append(',');result.append(scalar(list.get(index)));}return result.toString();}
            if(value.getClass().isArray()){StringBuilder result=new StringBuilder();for(int index=0;index<Array.getLength(value);index++){if(index!=0)result.append(',');result.append(scalar(Array.get(value,index)));}return result.toString();}
            require(value instanceof Number||value instanceof Boolean||value instanceof String,"Unsupported source scalar");return value.toString();
        }
        void field(String tag,String label,Field field,Object owner)throws Exception {
            if(Modifier.isTransient(field.getModifiers())){row(tag,"endpoint",label,field.getName(),field.getType().getSimpleName(),"absent");row("X",label,field.getName(),"transient-absent");return;}
            Object value=field.get(owner);row(tag,"endpoint",label,field.getName(),field.getType().getSimpleName(),scalar(value));
            if(!field.getType().isPrimitive())alias(tag,label,field.getName(),value);
        }
        void write(int attempts)throws Exception {
            row("C","workload-"+attempts,dense256?"endpoint256":"endpoint80",827364);
            row("X","Utils","settings","reconstructed-pinned-preferences-and-source-defaults");row("X","Utils","random","static-absent-no-final-RNG-observation");row("X","Organism","v","static-transient-absent");
            for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive())row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));
            int ordinal=0;for(Organism actor:world._organisms)row("M",ordinal++,identity(actor));
            ordinal=0;for(Entry entry:tree)row("F",ordinal++,identity(entry.actor),bits(entry.envelope.getMinX()),bits(entry.envelope.getMaxX()),bits(entry.envelope.getMinY()),bits(entry.envelope.getMaxY()));
            Field[] wf=fields(World.class);require(wf.length==15,"Unexpected World schema");for(Field field:wf)field("V","w0",field,world);
            row("W","endpoint","_population","int",world.getPopulation());row("W","endpoint","organism_count","int",world._organisms.size());row("W","endpoint","corpses","int",world.getNCorpses());row("W","endpoint","_O2","double",bits(world.getO2()));row("W","endpoint","_CO2","double",bits(world.getCO2()));row("W","endpoint","_CH4","double",bits(world.getCH4()));row("W","endpoint","NEXT_ID","int",world.NEXT_ID);row("W","endpoint","nFrames","long",world.getFrame());row("W","endpoint","time","long",world.getTime());StringBuilder ids=new StringBuilder();for(Organism actor:world._organisms){if(ids.length()!=0)ids.append(',');ids.append(actor.getID());}row("W","endpoint","organism_ids","int[]",ids);
            Field[] of=fields(Organism.class);require(of.length==153,"Unexpected Organism schema");
            for(Organism actor:actors){String label=identity(actor);require(actor._world==world,"Serialized actor World reference must share the root");for(Field field:of)field("O",label,field,actor);row("O","endpoint",label,"bounds","Rectangle",scalar(actor.getBounds()));row("H","endpoint",label,"genotype",genome(actor._geneticCode));row("H","endpoint",label,"infected",genome(actor._infectedGeneticCode));row("H","endpoint",label,"saved",genome(actor._savedGeneticCode));}
            Field[] sf=fields(WorldStatistics.class);require(sf.length==52,"Unexpected statistics schema");for(Field field:sf){Object value=field.get(world.worldStatistics);row("S","endpoint",field.getName(),field.getType().getSimpleName(),scalar(value));if(!field.getType().isPrimitive())alias("S","s0",field.getName(),value);if(dense256&&HISTORIES.contains(field.getName()))history(value);}
            Method[] getters=Arrays.stream(WorldStatistics.class.getDeclaredMethods()).filter(method->Modifier.isPublic(method.getModifiers())&&method.getParameterCount()==0&&method.getName().startsWith("get")).sorted(Comparator.comparing(Method::getName)).toArray(Method[]::new);require(getters.length==49,"Unexpected getter schema");for(Method getter:getters)row("T","endpoint",getter.getName(),getter.getReturnType().getSimpleName(),scalar(getter.invoke(world.worldStatistics)));
            for(GeneticCode code:genomes){String label=identity(code);row("G","endpoint",label,"_reference",label);row("G","endpoint",label,"_nGenes",code.getNGenes());for(Field field:fields(GeneticCode.class)){Object value=field.get(code);if(!field.getName().equals("_genes"))row("G","endpoint",label,field.getName(),scalar(value));if(!field.getType().isPrimitive())alias("G",label,field.getName(),value);}
                for(int index=0;index<code.getNGenes();index++){Gene gene=code.getGene(index);String name=identity(gene);row("J",label,index,name);boolean first=geneAliases.put(gene,true)==null;for(Field field:fields(Gene.class)){Object value=field.get(gene);row("N","endpoint",label,index,field.getName(),scalar(value));if(first&&!field.getType().isPrimitive())alias("N",name,field.getName(),value);}}
            }
            row("E");
        }
        public void close()throws IOException {writer.close();}
    }
    private static void require(boolean value,String message){if(!value)throw new IllegalStateException(message);}
    private static String bits(double value){return String.format(Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String digest(Path path)throws Exception {MessageDigest digest=MessageDigest.getInstance("SHA-256");try(InputStream stream=Files.newInputStream(path)){byte[] buffer=new byte[65536];for(int count;(count=stream.read(buffer))!=-1;)digest.update(buffer,0,count);}return HexFormat.of().formatHex(digest.digest());}
    private static List<Entry> stored(STRtree tree)throws Exception {
        Field pending=AbstractSTRtree.class.getDeclaredField("itemBoundables"),root=AbstractSTRtree.class.getDeclaredField("root"),built=AbstractSTRtree.class.getDeclaredField("built");pending.setAccessible(true);root.setAccessible(true);built.setAccessible(true);Object beforePending=pending.get(tree),beforeRoot=root.get(tree);boolean beforeBuilt=built.getBoolean(tree);List<Entry> result=new ArrayList<>();
        if(beforePending!=null)for(Object value:(List<?>)beforePending)collect(value,result);else collect(beforeRoot,result);
        require(beforePending==pending.get(tree)&&beforeRoot==root.get(tree)&&beforeBuilt==built.getBoolean(tree),"Stored tree observation must not mutate packing");IdentityHashMap<Organism,Boolean> unique=new IdentityHashMap<>();for(Entry entry:result)require(unique.put(entry.actor,true)==null,"Duplicate tree actor identity");return result;
    }
    private static void collect(Object value,List<Entry> entries){if(value==null)return;if(value instanceof ItemBoundable item){require(item.getItem() instanceof Organism&&item.getBounds() instanceof Envelope,"Unexpected stored entry");entries.add(new Entry((Organism)item.getItem(),(Envelope)item.getBounds()));}else if(value instanceof AbstractNode node){for(Object child:node.getChildBoundables())collect(child,entries);}else throw new IllegalStateException("Unknown stored boundable");}
    private static void settings(int attempts,Path preferences)throws Exception {
        require(digest(preferences).equals(PREFS.get(attempts)),"Unapproved preference bytes");String xml=Files.readString(preferences,StandardCharsets.UTF_8);Matcher matcher=Pattern.compile("<entry key=\"([^\"]+)\" value=\"([^\"]*)\"/>").matcher(xml);Map<String,String> values=new TreeMap<>();while(matcher.find())require(values.put(matcher.group(1),matcher.group(2))==null,"Duplicate preference key");require(values.size()==10&&values.get("INITIAL_ORGANISMS").equals(Integer.toString(attempts)),"Unexpected pinned preference scope");
        for(Field field:Utils.class.getDeclaredFields())if(field.getName().startsWith("DEF_"))Utils.class.getDeclaredField(field.getName().substring(4)).set(null,field.get(null));
        for(Map.Entry<String,String> entry:values.entrySet()){if(entry.getKey().equals("VERSION")||entry.getKey().equals("LOCALE"))continue;Field field=Utils.class.getDeclaredField(entry.getKey());if(field.getType()==boolean.class)field.setBoolean(null,Boolean.parseBoolean(entry.getValue()));else if(field.getType()==int.class)field.setInt(null,Integer.parseInt(entry.getValue()));else throw new IllegalStateException("Unexpected pinned preference type");}
    }
    public static void main(String[] args)throws Exception {
        boolean dense256=args.length==5&&args[4].equals("--dense-256");
        require(args.length==4||dense256,"Usage: biogenesis.WorkloadStateOracle ATTEMPTS INPUT_WORLD PREFS_XML NEW_OUTPUT_DIRECTORY [--dense-256]");
        require(!dense256||args[0].equals("12500"),"Dense256 mode requires exactly12500 initial attempts");Path output=Path.of(args[3]).toAbsolutePath().normalize(),parent=output.getParent().toRealPath();
        for(String key:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){String value=System.getProperty(key);require(value!=null&&Path.of(value).toRealPath().startsWith(parent)&&!Path.of(value).toRealPath().equals(parent),"Explicit isolated profile required: "+key);}
        require(!Files.exists(output),"Output directory must not exist");
        if(args[0].equals("filter-negative")){ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream stream=new ObjectOutputStream(bytes)){stream.writeObject(new HashMap<>());}try(FilteredInput stream=new FilteredInput(new ByteArrayInputStream(bytes.toByteArray()))){stream.readObject();}throw new IllegalStateException("Filter diagnostic unexpectedly accepted");}
        int attempts=Integer.parseInt(args[0]);require(INPUTS.containsKey(attempts),"Unspecified pinned workload");Path input=Path.of(args[1]).toRealPath();require((dense256?Files.size(input)==DENSE256_BYTES:Files.size(input)<=MAX_BYTES)&&digest(input).equals(dense256?DENSE256_SHA256:INPUTS.get(attempts)),"Unapproved endpoint bytes");settings(attempts,Path.of(args[2]));
        World world;long depth,references,array,bytes;Set<String> descriptors;
        try(FilteredInput stream=new FilteredInput(Files.newInputStream(input),dense256)){Object value=stream.readObject();require(value instanceof World,"World root required");world=(World)value;require(stream.read()==-1,"Trailing serialized data is not allowed");depth=stream.depth;references=stream.references;array=stream.array;bytes=stream.bytes;descriptors=stream.descriptors;}
        require(world.getFrame()==(dense256?0:80)&&world.getTime()==(dense256?1:0)&&!world._corridorexists&&world._visibleWorld==null,"Unexpected ordinary endpoint scope");Files.createDirectory(output);Capture capture;
        try(Capture owned=new Capture(world,output.resolve("state.tsv"),dense256)){capture=owned;owned.write(attempts);}
        Files.writeString(output.resolve("decode.json"),"{\"attempts\":"+attempts+",\"actors\":"+capture.actors.size()+",\"members\":"+world._organisms.size()+",\"tree_entries\":"+capture.tree.size()+",\"genomes\":"+capture.genomes.size()+",\"filter_depth\":"+depth+",\"filter_references\":"+references+",\"filter_array\":"+array+",\"filter_bytes\":"+bytes+",\"descriptors\":"+descriptors.size()+",\"output_bytes\":"+capture.outputBytes+",\"sha256\":\""+digest(output.resolve("state.tsv"))+"\"}\n",StandardCharsets.UTF_8,StandardOpenOption.CREATE_NEW);
        System.out.println("WORKLOAD_STATE PASS attempts="+attempts+" actors="+capture.actors.size()+" members="+world._organisms.size()+" tree="+capture.tree.size()+" bytes="+capture.outputBytes);
    }
}
