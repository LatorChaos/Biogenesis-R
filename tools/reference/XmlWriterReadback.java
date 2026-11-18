// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Color;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import org.xml.sax.SAXException;

/** Parse actual supplied documents with the unchanged original parser and observe complete genotypes */
public final class XmlWriterReadback {
    private static final StringBuilder rows=new StringBuilder();
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static void row(Object...items){for(int i=0;i<items.length;i++){if(i>0)rows.append('\t');rows.append(items[i]);}rows.append('\n');}
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(f->!Modifier.isStatic(f.getModifiers())).peek(f->f.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String scalar(Object value){if(value==null)return "~";if(value instanceof Double d)return String.format(Locale.ROOT,"%016x",Double.doubleToRawLongBits(d));if(value instanceof Color c)return String.format(Locale.ROOT,"%08x",c.getRGB());if(value instanceof String s)return "b64:"+Base64.getEncoder().encodeToString(s.getBytes(StandardCharsets.UTF_8));return value.toString();}
    private static byte[] randomBytes()throws Exception{ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream stream=new ObjectOutputStream(bytes)){stream.writeObject(Utils.random);}return bytes.toByteArray();}
    public static void main(String[]args)throws Exception{
        require(args.length==3,"Usage: XmlWriterReadback RUN_ROOT MANIFEST NEW_OUTPUT_FILE");
        Path root=Path.of(args[0]).toRealPath();
        for(String name:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(name);require(value!=null,"Missing explicit profile: "+name);
            require(Path.of(value).toRealPath().startsWith(root),"Profile outside owned run: "+name);
        }
        Path manifest=Path.of(args[1]).toRealPath();require(manifest.startsWith(root),"Manifest outside owned run");
        Path output=Path.of(args[2]).toAbsolutePath().normalize();require(output.getParent().toRealPath().startsWith(root)&&!Files.exists(output),"New owned output required");
        System.setProperty("javax.xml.accessExternalDTD","");System.setProperty("javax.xml.accessExternalSchema","");
        for(Field field:Utils.class.getDeclaredFields())if(field.getName().startsWith("DEF_"))Utils.class.getDeclaredField(field.getName().substring(4)).set(null,field.get(null));
        Utils.ACCEPT_CONNECTIONS=false;Utils.AUTO_BACKUP=false;Utils.INITIAL_ORGANISMS=0;Utils.random=new Random(827364L);
        Set<String> ids=new HashSet<>();int calls=0;
        for(String line:Files.readAllLines(manifest,StandardCharsets.UTF_8)){
            String[] parts=line.split("\t",-1);require(parts.length==2&&parts[0].matches("[a-z]+-[0-9]{4}"),"Manifest shape/identity");require(ids.add(parts[0]),"Duplicate identity");
            Path file=root.resolve(parts[1]).toRealPath();require(file.startsWith(root)&&Files.isRegularFile(file)&&Files.size(file)<=16L*1024*1024,"Input outside owned byte bound");
            byte[] before=randomBytes();row("C",parts[0]);BioXMLParser parser=new BioXMLParser();require(parser.builder!=null,"Original parser unavailable");
            parser.builder.setEntityResolver((publicID,systemID)->{throw new SAXException("External resources are denied");});
            GeneticCode code=null;Exception failure=null;
            try{code=parser.parseGeneticCode(file.toFile());}catch(Exception error){failure=error;}
            if(failure==null){
                row("O","success");row("G","_nGenes",code.getNGenes());
                for(Field field:fields(GeneticCode.class))if(!field.getName().equals("_genes"))row("G",field.getName(),scalar(field.get(code)));
                IdentityHashMap<Gene,Integer> roots=new IdentityHashMap<>();StringJoiner aliases=new StringJoiner(",");
                for(int i=0;i<code.getNGenes();i++){
                    Gene gene=code.getGene(i);Integer first=roots.get(gene);if(first==null){first=i;roots.put(gene,i);}aliases.add(first.toString());
                    for(Field field:fields(Gene.class))row("N",i,field.getName(),scalar(field.get(gene)));
                }
                row("A",aliases);
            }else{row("O","failure",failure.getClass().getName());String message=String.valueOf(failure.getMessage()).replace(root.toString(),"@RUN@");row("X",Base64.getEncoder().encodeToString(message.getBytes(StandardCharsets.UTF_8)));}
            require(Arrays.equals(before,randomBytes()),"Original parser changed RNG");row("R","unchanged");row("E");calls++;
        }
        Files.writeString(output,rows,StandardCharsets.UTF_8,StandardOpenOption.CREATE_NEW);
        System.out.println("XML_WRITER_READBACK PASS calls="+calls);
    }
}
