// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Color;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.regex.*;
import org.xml.sax.*;

/** Observe original genotype XML methods with isolated, explicit inputs */
public final class XmlOracle {
    private static final long SEED=827364L;
    private static final String[] BASE=("GREEN FOREST SPRING SUMMER LIME LEAF C4 JADE GRASS BARK PURPLE RED FIRE ORANGE MAROON PINK CREAM SILVER SPIKE LILAC GRAY VIOLET OLIVE SKY BLUE OCHRE FALLOW SPORE WHITE PLAGUE CORAL MINT LAVENDER MAGENTA ROSE CYAN TEAL YELLOW AUBURN INDIGO BLOND FLOWER DARKGRAY GOLD DARK EYE").split(" ");
    private static final String[] INTERNAL=("WINTER OLDBARK DARKJADE DARKGREEN DARKFIRE DARKLILAC DEEPSKY DARKOLIVE SPIKEPOINT FRUIT VISION ICE LIGHT_BLUE LIGHTBROWN GREENBROWN BROKEN DEADBARK BROWN").split(" ");
    private static final String MARKER="XML_ORACLE_OWNED_NONSECRET_MARKER";
    private static final StringBuilder ROWS=new StringBuilder();
    private static Path output;
    private static int caseCount,methodCount,successCount,failureCount;
    private static final class Spec {
        final String name;
        String xml,encoding="UTF-8",writerEncoding="DEFAULT",color="GREEN",mode="parse";
        int genes=1,variant;
        boolean shared;
        final Map<String,Object> globals=new LinkedHashMap<>();
        final Map<String,String> auxiliary=new LinkedHashMap<>();
        Spec(String name,String xml){this.name=name;this.xml=xml;}
    }
    private static final class Capture {
        final Spec spec;
        final Path directory;
        final IdentityHashMap<GeneticCode,String> references=new IdentityHashMap<>();
        int stage;
        Capture(Spec spec,int index)throws Exception {
            this.spec=spec;directory=output.resolve(String.format(Locale.ROOT,"cases/%04d",index));Files.createDirectories(directory);
            reset();for(var entry:spec.globals.entrySet())Utils.class.getDeclaredField(entry.getKey()).set(null,entry.getValue());
            Utils.random=new Random(SEED);row("C",spec.name,spec.mode,SEED);
            int settings=0;for(Field field:Utils.class.getDeclaredFields())if(Modifier.isStatic(field.getModifiers())&&!Modifier.isFinal(field.getModifiers())&&field.getType().isPrimitive()){row("U",field.getName(),field.getType().getSimpleName(),scalar(field.get(null)));settings++;}require(settings==147,"Primitive settings drift");
            for(var entry:spec.auxiliary.entrySet())Files.writeString(directory.resolve(entry.getKey()),entry.getValue(),StandardCharsets.UTF_8,StandardOpenOption.CREATE_NEW);
            random("initial");
        }
        void random(String stage)throws Exception {Random copy=copyRandom(Utils.random);row("R",stage,copy.nextLong(),copy.nextInt(10000),bits(copy.nextDouble()));}
        String label(GeneticCode code){return references.computeIfAbsent(code,key->"g"+references.size());}
        void state(String stage,GeneticCode code)throws Exception {
            row("G",stage,"genotype","_reference",label(code));row("G",stage,"genotype","_nGenes",code.getNGenes());
            for(Field field:fields(GeneticCode.class))if(!field.getName().equals("_genes"))row("G",stage,"genotype",field.getName(),scalar(field.get(code)));
            IdentityHashMap<Gene,Integer> roots=new IdentityHashMap<>();StringJoiner aliases=new StringJoiner(",");
            for(int index=0;index<code.getNGenes();index++){
                Gene gene=code.getGene(index);Integer root=roots.get(gene);if(root==null){root=index;roots.put(gene,index);}aliases.add(root.toString());
                for(Field field:fields(Gene.class))row("N",stage,"genotype",index,field.getName(),scalar(field.get(gene)));
            }
            row("A",stage,"gene_alias_roots",aliases);
        }
        void file(String stage,String role,Path path,String encoding)throws Exception {row("F",stage,role,output.relativize(path).toString().replace('\\','/'),sha(Files.readAllBytes(path)),Files.size(path),encoding);}
        void failure(String stage,Exception error){failureCount++;row("O",stage,"failure");String message=String.valueOf(error.getMessage()).replace(directory.toUri().toString(),"case:/").replace(directory.toString(),"@CASE@");row("X",stage,error.getClass().getName(),b64(message));}
        GeneticCode parse(Path path)throws Exception {
            String current="step-"+stage++;row("K",current,"parse",output.relativize(path).toString().replace('\\','/'));methodCount++;
            BioXMLParser parser=new BioXMLParser();require(parser.builder!=null,"Original XML builder unavailable");
            parser.builder.setEntityResolver((publicID,systemID)->{
                try {
                    java.net.URI uri=java.net.URI.create(systemID);require(uri.getScheme().equals("file"),"Non-file XML resolution denied");
                    Path target=Path.of(uri).toRealPath();String name=target.getFileName().toString();
                    require(target.getParent().equals(directory.toRealPath())&&spec.auxiliary.containsKey(name),"XML resource escaped owned allowlist");
                    row("V",current,"owned_external_resolution",name);return null;
                } catch(IOException exception){throw new SAXException(exception);}
            });
            GeneticCode result;
            try {result=parser.parseGeneticCode(path.toFile());}
            catch(IllegalStateException error){throw error;}
            catch(Exception error){failure(current,error);random(current);return null;}
            successCount++;row("O",current,"success");state(current,result);row("T",current,b64(parser.doc.getDocumentElement().getTextContent()));random(current);return result;
        }
        Path write(GeneticCode code)throws Exception {
            String current="step-"+stage++;Path path=directory.resolve(current+".bgg");row("K",current,"write",output.relativize(path).toString().replace('\\','/'));methodCount++;
            ByteArrayOutputStream bytes=new ByteArrayOutputStream();Exception failure=null;
            try(PrintStream stream=new PrintStream(bytes,false,spec.writerEncoding.equals("DEFAULT")?Charset.defaultCharset():Charset.forName(spec.writerEncoding))){try{BioXMLParser.writeGeneticCode(stream,code);}catch(Exception error){failure=error;}require(!stream.checkError(),"In-memory PrintStream failure");}
            Files.write(path,bytes.toByteArray(),StandardOpenOption.CREATE_NEW);file(current,failure==null?"written":"partial",path,spec.writerEncoding);
            if(failure!=null){failure(current,failure);random(current);return null;}
            successCount++;row("O",current,"success");random(current);return path;
        }
        void run()throws Exception {
            GeneticCode code;
            if(spec.mode.equals("write")){code=recipe(spec);state("input",code);}
            else {Path path=directory.resolve("input.bgg");Files.write(path,spec.xml.getBytes(Charset.forName(spec.encoding)),StandardOpenOption.CREATE_NEW);file("input","supplied",path,spec.encoding);code=parse(path);}
            if(code!=null){Path written=write(code);if(written!=null){GeneticCode restored=parse(written);if(restored!=null)write(restored);}}
            for(String name:spec.auxiliary.keySet())file("auxiliary","owned",directory.resolve(name),"UTF-8");
            row("E");caseCount++;
        }
    }
    private static void reset()throws Exception {
        for(Field field:Utils.class.getDeclaredFields())if(field.getName().startsWith("DEF_"))Utils.class.getDeclaredField(field.getName().substring(4)).set(null,field.get(null));
        Utils.ACCEPT_CONNECTIONS=false;Utils.AUTO_BACKUP=false;Utils.INITIAL_ORGANISMS=0;
    }
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String bits(double value){return String.format(Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static String scalar(Object value){if(value==null)return "~";if(value instanceof Double number)return bits(number);if(value instanceof Color color)return String.format(Locale.ROOT,"%08x",color.getRGB());if(value instanceof String text)return "b64:"+b64(text);return value.toString();}
    private static String b64(String text){return Base64.getEncoder().encodeToString(text.getBytes(StandardCharsets.UTF_8));}
    private static String sha(byte[] data)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(data));}
    private static void row(Object...values){for(int i=0;i<values.length;i++){if(i>0)ROWS.append('\t');ROWS.append(values[i]);}ROWS.append('\n');}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Random copyRandom(Random original)throws Exception {ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream stream=new ObjectOutputStream(bytes)){stream.writeObject(original);}try(ObjectInputStream stream=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)stream.readObject();}}
    private static Color color(String name)throws Exception {switch(name){case "NULL":return null;case "UNKNOWN":return new Color(0x12345678,true);case "TRANSLUCENT":return new Color(0x12ff00ff,true);case "TRANSPARENT":return new Color(0x00ff00ff,true);case "RED":return Color.RED;case "GREEN":return Color.GREEN;case "BLUE":return Color.BLUE;case "CYAN":return Color.CYAN;case "WHITE":return Color.WHITE;case "GRAY":return Color.GRAY;case "MAGENTA":return Color.MAGENTA;case "PINK":return Color.PINK;case "ORANGE":return Color.ORANGE;case "YELLOW":return Color.YELLOW;case "SILVER":return Color.LIGHT_GRAY;case "DARKGRAY":return Color.DARK_GRAY;default:return (Color)Utils.class.getDeclaredField("Color"+name).get(null);}}
    private static GeneticCode recipe(Spec spec)throws Exception {
        List<Gene> genes=new ArrayList<>();for(int i=0;i<spec.genes;i++){
            Gene gene;if(spec.shared&&i>0)gene=genes.get(0);else{
                gene=new Gene();gene.setLength(i==0?12.5d:-0d);gene.setTheta(i==0?-0.25d:0x1.23456789abcdefp-10);gene.setColor(color(spec.color));gene.setBranch(i==0?-1:Integer.MIN_VALUE);
                int reaction=0;for(Field field:fields(Gene.class))if(field.getName().endsWith("reaction"))field.setInt(gene,spec.variant==1?Integer.MIN_VALUE+reaction++:reaction++-7);
            }genes.add(gene);
        }
        boolean enabled=spec.variant==2;
        GeneticCode code=new GeneticCode(genes,enabled?8:1,enabled?1:0,enabled?10000:17,enabled?2:0,enabled?3:1,enabled?4:1,enabled?12:1,enabled?70:1,enabled,enabled,enabled,enabled,enabled,enabled,enabled,enabled,enabled,enabled,enabled,enabled,enabled,enabled,enabled);
        code._generation=91;code._cladeID="xml\u0000\u00e9";code._updateClade=true;
        if(spec.variant==3){code._mirror=-7;code._symmetry=9;}if(spec.variant==4)code._mirror=-7;return code;
    }
    private static String export(GeneticCode code)throws Exception {ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(PrintStream stream=new PrintStream(bytes,false,StandardCharsets.UTF_8)){BioXMLParser.writeGeneticCode(stream,code);}return bytes.toString(StandardCharsets.UTF_8);}
    private static String change(String xml,String attribute,String value){int start=xml.indexOf("<genetic_code ");String head=xml.substring(0,start),body=xml.substring(start);Matcher matcher=Pattern.compile("(?<![a-z])"+Pattern.quote(attribute)+"=\"[^\"]*\"").matcher(body);require(matcher.find(),"Missing explicit XML attribute");return head+matcher.replaceFirst(Matcher.quoteReplacement(attribute+"=\""+value+"\""));}
    private static String omit(String xml,String owner,String attribute){int start=xml.indexOf(owner.equals("root")?"<genetic_code ":"<gene ");String head=xml.substring(0,start),body=xml.substring(start);body=body.replaceFirst("\\s"+Pattern.quote(attribute)+"=\"[^\"]*\"","");String result=head+body;return result.replaceFirst("(<!ATTLIST "+(owner.equals("root")?"genetic_code":"gene")+" "+attribute+" [^>]*?)#REQUIRED", "$1#IMPLIED");}
    private static String permissive(String xml,String owner,String attribute){return xml.replaceFirst("<!ATTLIST "+owner+" "+attribute+" [^>]*>","<!ATTLIST "+owner+" "+attribute+" CDATA #REQUIRED>");}
    private static Spec parseSpec(String name,String xml){return new Spec(name,xml);}
    private static Spec writer(String name,String color){Spec spec=new Spec(name,null);spec.mode="write";spec.color=color;return spec;}
    private static List<Spec> cases()throws Exception {
        reset();Spec baseSpec=writer("baseline","GREEN");String base=export(recipe(baseSpec));List<Spec> result=new ArrayList<>();
        result.add(parseSpec("canonical-baseline",base));
        for(String name:BASE)result.add(writer("writer-color-"+name,name));
        for(String name:INTERNAL)result.add(writer("writer-state-color-"+name,name));
        for(String name:new String[]{"UNKNOWN","TRANSLUCENT","NULL"})result.add(writer("writer-color-"+name,name));
        Spec flags=writer("writer-all-flags-maxima","GREEN");flags.variant=2;result.add(flags);
        Spec signed=writer("writer-signed-reactions-shared-alias","GREEN");signed.genes=2;signed.shared=true;signed.variant=1;result.add(signed);
        Spec many=writer("writer-129-genes","GREEN");many.genes=129;result.add(many);
        Spec empty=writer("writer-empty-code","GREEN");empty.genes=0;result.add(empty);
        Spec raw=writer("writer-invalid-raw-metadata","GREEN");raw.variant=3;result.add(raw);
        Spec iso=writer("writer-latin1-ascii-schema","GREEN");iso.writerEncoding="ISO-8859-1";result.add(iso);
        Spec utf16=writer("writer-utf16-with-utf8-declaration","GREEN");utf16.writerEncoding="UTF-16";result.add(utf16);
        String[] numeric={"symmetry","mutationrate","activity","modifiescream","modifiesfallow","modifiesspore","adaptspore"};int[] lower={1,0,0,1,1,1,1},upper={8,10000,2,3,4,12,70};
        for(int i=0;i<numeric.length;i++)for(int value:new int[]{lower[i],upper[i],lower[i]-1,upper[i]+1})result.add(parseSpec("root-"+numeric[i]+"-"+value,change(permissive(base,"genetic_code",numeric[i]),numeric[i],Integer.toString(value))));
        List<String> roots=new ArrayList<>(),attributes=new ArrayList<>();Matcher declarations=Pattern.compile("<!ATTLIST (genetic_code|gene) ([a-z]+) ").matcher(base);while(declarations.find())(declarations.group(1).equals("genetic_code")?roots:attributes).add(declarations.group(2));require(roots.size()==23&&attributes.size()==40,"XML attribute inventory drift");
        for(String name:roots){result.add(parseSpec("missing-root-"+name,omit(base,"root",name)));if(!List.of(numeric).contains(name)){result.add(parseSpec("boolean-onehot-"+name,change(base,name,"yes")));result.add(parseSpec("invalid-boolean-"+name,change(permissive(base,"genetic_code",name),name,"true")));}}
        for(String name:numeric)result.add(parseSpec("malformed-root-"+name,change(permissive(base,"genetic_code",name),name,"1.0")));
        for(String name:attributes)result.add(parseSpec("missing-gene-"+name,omit(base,"gene",name)));
        String[] decimals={"-0.0","-4.5","NaN","Infinity","-Infinity","1.7976931348623157E308","4.9E-324","0x1.8p2","1.25d"," 2.5 ","+3.0","1e309","-1e-400","nan","inf","1,5",""};
        for(int i=0;i<decimals.length;i++)result.add(parseSpec("double-length-"+i,change(base,"length",decimals[i])));
        for(int i=0;i<6;i++)result.add(parseSpec("double-theta-"+i,change(base,"theta",decimals[i])));
        String[] integers={"2147483647","-2147483648","+7"," 7 ","2147483648","1.0","0x2"};for(int i=0;i<integers.length;i++)result.add(parseSpec("branch-integer-"+i,change(base,"branch",integers[i])));
        result.add(parseSpec("reaction-integer-overflow",change(base,"friendreaction","2147483648")));
        for(String bad:new String[]{"GREEN","unknown","#ff00ff"})result.add(parseSpec("invalid-color-"+bad.replace('#','x'),change(base,"color",bad)));
        result.add(parseSpec("missing-canonical-required-attribute",base.replaceFirst(" symmetry=\"1\"","")));
        result.add(parseSpec("comments-and-whitespace",base.replace("<genetic_code ","<!-- before -->\n<genetic_code ").replace("\t<gene ","\n<!-- between -->\n\t<gene ")));
        result.add(parseSpec("no-doctype",base.replaceFirst("(?s)<!DOCTYPE.*?]>\\s*","")));
        result.add(parseSpec("wrong-root",base.replace("genetic_code","wrong_root")));
        result.add(parseSpec("wrong-child",base.replaceAll("\\bgene\\b","wrong_child")));
        result.add(parseSpec("duplicate-attribute",base.replace("<genetic_code ","<genetic_code symmetry=\"1\" ")));
        result.add(parseSpec("empty-canonical-gene-list",base.replaceFirst("(?m)^\\s*<gene [^\\n]*\\n","")));
        result.add(parseSpec("empty-permissive-gene-list",base.replace("(gene+)","(gene*)").replaceFirst("(?m)^\\s*<gene [^\\n]*\\n","")));
        String defaulted=omit(base,"root","mutationrate").replace("mutationrate CDATA #IMPLIED","mutationrate CDATA \"1234\"");result.add(parseSpec("internal-dtd-default",defaulted));
        result.add(parseSpec("internal-entity-attribute",change(base.replace("<!DOCTYPE genetic_code [","<!DOCTYPE genetic_code [\n<!ENTITY amount \"6.25\">"),"length","&amount;")));
        Spec latin=parseSpec("latin1-input-comment",base.replace("UTF-8","ISO-8859-1").replace("<genetic_code ","<!-- caf\u00e9 -->\n<genetic_code "));latin.encoding="ISO-8859-1";result.add(latin);
        Spec unicode=parseSpec("utf16-input",base.replace("UTF-8","UTF-16"));unicode.encoding="UTF-16";result.add(unicode);
        String dtd=base.substring(base.indexOf("[\n")+2,base.indexOf("]>"));String body=base.substring(base.indexOf("<genetic_code "));
        Spec external=parseSpec("owned-external-dtd-default","<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE genetic_code SYSTEM \"owned.dtd\">\n"+body.replaceFirst(" mutationrate=\"17\"",""));external.auxiliary.put("owned.dtd",dtd.replace("mutationrate CDATA #REQUIRED","mutationrate CDATA \"321\""));result.add(external);
        Spec parameter=parseSpec("owned-external-parameter-entity",change(base.replace("<!DOCTYPE genetic_code [","<!DOCTYPE genetic_code [\n<!ENTITY % owned SYSTEM \"parameter.dtd\">\n%owned;"),"length","&amount;"));parameter.auxiliary.put("parameter.dtd","<!ENTITY amount \"22.5\">\n");result.add(parameter);
        Spec general=parseSpec("owned-external-general-entity",base.replace("(gene+)","ANY").replace("<!DOCTYPE genetic_code [","<!DOCTYPE genetic_code [\n<!ENTITY owned SYSTEM \"owned-marker.txt\">").replace("\t<gene ","&owned;\n\t<gene "));general.auxiliary.put("owned-marker.txt",MARKER);result.add(general);
        Spec pref=parseSpec("mutationrate-independent-of-preferences",change(base,"mutationrate","10000"));pref.globals.put("MIN_MUTATION_RATE",2222);pref.globals.put("MAX_MUTATION_RATE",3333);result.add(pref);
        result.add(parseSpec("additional-length-positive-zero",change(base,"length","0.0")));
        result.add(parseSpec("reaction-signed-maximum",change(base,"friendreaction","2147483647")));
        result.add(parseSpec("reaction-signed-minimum",change(base,"friendreaction","-2147483648")));
        String defaults=base;
        for(String owner:new String[]{"genetic_code","gene"}){
            int start=base.indexOf("<"+owner+" ");String element=base.substring(start,base.indexOf('>',start));
            Matcher values=Pattern.compile("([a-z]+)=\"([^\"]*)\"").matcher(element);
            while(values.find())defaults=defaults.replaceFirst("(<!ATTLIST "+owner+" "+values.group(1)+" [^>]*?)#REQUIRED", "$1\""+values.group(2)+"\"");
            defaults=defaults.replaceFirst("<"+owner+" [^>]*>",owner.equals("gene")?"<gene />":"<genetic_code>");
        }
        result.add(parseSpec("all-internal-dtd-attribute-defaults",defaults));
        Spec mirror=writer("writer-nonzero-raw-mirror","GREEN");mirror.variant=4;result.add(mirror);
        result.add(writer("writer-transparent-alpha-zero","TRANSPARENT"));
        return result;
    }
    public static void main(String[]args)throws Exception {
        if(args.length!=1)throw new IllegalArgumentException("Usage: biogenesis.XmlOracle NEW_OUTPUT_DIRECTORY");
        output=Path.of(args[0]).toAbsolutePath().normalize();Path parent=output.getParent().toRealPath();
        for(String name:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){String value=System.getProperty(name);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+name);}
        require(!Files.exists(output),"Output directory must not exist");Files.createDirectory(output);
        System.setProperty("javax.xml.accessExternalDTD","file");System.setProperty("javax.xml.accessExternalSchema","");
        int index=0;for(Spec spec:cases())new Capture(spec,index++).run();
        Files.writeString(output.resolve("xml.tsv"),ROWS,StandardCharsets.UTF_8,StandardOpenOption.CREATE_NEW);
        Files.writeString(output.resolve("environment.txt"),"locale="+Locale.getDefault().toLanguageTag()+"\ncharset="+Charset.defaultCharset().name()+"\nexternalDTD=file; exact owned allowlist resolver\n",StandardCharsets.UTF_8,StandardOpenOption.CREATE_NEW);
        System.out.println("XML_ORACLE PASS cases="+caseCount+" methods="+methodCount+" successes="+successCount+" failures="+failureCount);
    }
}
