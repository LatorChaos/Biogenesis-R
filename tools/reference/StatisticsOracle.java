package biogenesis;

import java.awt.Color;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
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

/** Observe real statistics events and getters without reproducing their calculations */
public final class StatisticsOracle {
    private static int cases, snapshots, calls;

    private static final class Capture {
        final StringBuilder rows = new StringBuilder();
        final Map<String,GeneticCode> genomes = new LinkedHashMap<>();
        final Map<String,Organism> actors = new LinkedHashMap<>();
        final IdentityHashMap<GeneticCode,String> references = new IdentityHashMap<>();
        final WorldStatistics statistics;
        final long seed = 506172;
        int controls, events;
        Capture(String name,boolean raw,int population,double oxygen,double carbon,double methane)throws Exception{
            Utils.INITIAL_ORGANISMS=population;Utils.INITIAL_O2=oxygen;Utils.INITIAL_CO2=carbon;Utils.INITIAL_CH4=methane;
            Utils.random=new Random(seed);
            statistics=new WorldStatistics();World world=new World(null);
            row("C",name,raw?"raw-null-genome":"canonical",seed);
            row("U","INITIAL_ORGANISMS","int",population);row("U","INITIAL_O2","double",bits(oxygen));
            row("U","INITIAL_CO2","double",bits(carbon));row("U","INITIAL_CH4","double",bits(methane));
            GeneticCode original=explicit();
            genomes.put("g0",original);genomes.put("g1",(GeneticCode)original.clone());genomes.put("g2",explicit());
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet()){
                references.put(entry.getValue(),entry.getKey());
                row("I","setup","genome:"+entry.getKey()+":construction","String",entry.getKey().equals("g1")?"clone:g0":"explicit");
                genome("input",entry.getKey(),entry.getValue());
            }
            String[] names={"a","b","c","d"},codes={"g0","g1","g0","g2"};
            int[] ages={7,Integer.MAX_VALUE,-1,15};
            for(int i=0;i<names.length;i++){
                Organism actor=new Organism(world,raw?null:genomes.get(codes[i]));
                actor._age=ages[i];actor.alive=i!=3;actor.active=i!=1;
                actors.put(names[i],actor);
                row("A",names[i],raw?"~":codes[i],0,0,0,ages[i],actor.alive,actor.active);
            }
            snapshot("initial");
        }
        Capture(String name)throws Exception{this(name,false,Utils.DEF_INITIAL_ORGANISMS,Utils.DEF_INITIAL_O2,Utils.DEF_INITIAL_CO2,Utils.DEF_INITIAL_CH4);}
        void row(Object...values){for(int i=0;i<values.length;i++){if(i!=0)rows.append('\t');rows.append(values[i]);}rows.append('\n');}
        String scalar(Object value){
            if(value==null)return "~";
            if(value instanceof Double number)return bits(number);
            if(value instanceof Color color)return String.format(java.util.Locale.ROOT,"%08x",color.getRGB());
            if(value instanceof GeneticCode code){String name=references.get(code);require(name!=null,"Unmapped genome reference");return name;}
            if(value instanceof List<?> list){StringBuilder text=new StringBuilder();for(Object item:list){if(text.length()!=0)text.append(',');text.append(scalar(item));}return text.toString();}
            return value.toString();
        }
        void genome(String stage,String name,GeneticCode code)throws Exception{
            row("G",stage,name,"_reference",references.get(code));row("G",stage,name,"_nGenes",code.getNGenes());
            for(Field field:fields(GeneticCode.class))if(!field.getName().equals("_genes"))row("G",stage,name,field.getName(),scalar(field.get(code)));
            for(int index=0;index<code.getNGenes();index++)for(Field field:fields(Gene.class))row("N",stage,name,index,field.getName(),scalar(field.get(code.getGene(index))));
        }
        void snapshot(String stage)throws Exception{
            row("P",stage);
            Field[] fields=fields(WorldStatistics.class);require(fields.length==52,"Unexpected statistics field schema");
            for(Field field:fields)row("S",stage,field.getName(),field.getType().getSimpleName(),scalar(field.get(statistics)));
            Method[] getters=Arrays.stream(WorldStatistics.class.getDeclaredMethods()).filter(method->Modifier.isPublic(method.getModifiers())&&method.getParameterCount()==0&&method.getName().startsWith("get")).sorted(Comparator.comparing(Method::getName)).toArray(Method[]::new);
            require(getters.length==49,"Unexpected statistics getter schema");
            for(Method method:getters)row("T",stage,method.getName(),method.getReturnType().getSimpleName(),scalar(method.invoke(statistics)));
            for(Map.Entry<String,GeneticCode> entry:genomes.entrySet())genome(stage,entry.getKey(),entry.getValue());
            Random peek=copyRandom(Utils.random);long nextLong=peek.nextLong();int nextInt=peek.nextInt(10000);double nextDouble=peek.nextDouble();
            Random plain=new Random(seed);for(int i=0;i<4;i++)plain.nextDouble();
            require(nextLong==plain.nextLong()&&nextInt==plain.nextInt(10000)&&bits(nextDouble).equals(bits(plain.nextDouble())),"Statistics changed constructor RNG stream");
            row("R",stage,nextLong,nextInt,bits(nextDouble));snapshots++;
        }
        void set(String target,String kind,String value)throws Exception{
            row("I","control-"+controls++,target,kind,value);
            String[] path=target.split(":",3);Object owner;String fieldName;
            switch(path[0]){
                case "actor":owner=actors.get(path[1]);fieldName=path[2];break;
                case "statistics":owner=statistics;fieldName=path[1];break;
                case "genome":
                    if(path[2].equals("gene0._length")){genomes.get(path[1]).getGene(0).setLength(number(value));return;}
                    owner=genomes.get(path[1]);fieldName=path[2];break;
                default:throw new IllegalArgumentException("Unknown control target");
            }
            if(kind.equals("List")){
                @SuppressWarnings("unchecked") List<Double> history=(List<Double>)WorldStatistics.class.getMethod(historyGetter(fieldName)).invoke(statistics);
                history.clear();if(!value.isEmpty())for(String item:value.split(","))history.add(number(item));return;
            }
            Field field=owner.getClass().getDeclaredField(fieldName);field.setAccessible(true);
            switch(kind){
                case "int":field.setInt(owner,Integer.parseInt(value));break;
                case "long":field.setLong(owner,Long.parseLong(value));break;
                case "double":field.setDouble(owner,number(value));break;
                case "boolean":field.setBoolean(owner,Boolean.parseBoolean(value));break;
                case "String":field.set(owner,value.equals("~")?null:value);break;
                default:throw new IllegalArgumentException("Unknown control type");
            }
        }
        void integer(String target,int value)throws Exception{set(target,"int",Integer.toString(value));}
        void longValue(String target,long value)throws Exception{set(target,"long",Long.toString(value));}
        void flag(String target,boolean value)throws Exception{set(target,"boolean",Boolean.toString(value));}
        void call(String method,String...args)throws Exception{
            String stage="event-"+events++;Object[] record=new Object[3+args.length];record[0]="K";record[1]=stage;record[2]=method;System.arraycopy(args,0,record,3,args.length);row(record);
            switch(method){
                case "eventPopulationIncrease":statistics.eventPopulationIncrease(Integer.parseInt(args[0]));break;
                case "eventPopulationDecrease":statistics.eventPopulationDecrease(Integer.parseInt(args[0]));break;
                case "eventOrganismCreated":statistics.eventOrganismCreated();break;
                case "eventOrganismBorn":statistics.eventOrganismBorn(actor(args[0]),actor(args[1]));break;
                case "eventOrganismDie":statistics.eventOrganismDie(actor(args[0]),actor(args[1]));break;
                case "eventOrganismInfects":statistics.eventOrganismInfects(actor(args[0]),actor(args[1]));break;
                case "eventTime":statistics.eventTime(Integer.parseInt(args[0]),number(args[1]),number(args[2]),number(args[3]));break;
                case "findBestAliveBeings":
                    List<Organism> ordered=new ArrayList<>();if(!args[0].equals("~"))for(String name:args[0].split(","))ordered.add(actor(name));statistics.findBestAliveBeings(ordered);break;
                default:throw new IllegalArgumentException("Unknown actual event method");
            }
            calls++;
        }
        Organism actor(String name){return name.equals("~")?null:actors.get(name);}
        void event(String method,String...args)throws Exception{call(method,args);snapshot("after-"+(events-1));}
        void time(int population,double oxygen,double carbon,double methane,boolean observe)throws Exception{
            call("eventTime",Integer.toString(population),bits(oxygen),bits(carbon),bits(methane));if(observe)snapshot("after-"+(events-1));
        }
        String finish(){row("E");cases++;return rows.toString();}
    }
    private static Field[] fields(Class<?> type){return Arrays.stream(type.getDeclaredFields()).filter(field->!Modifier.isStatic(field.getModifiers())).peek(field->field.setAccessible(true)).sorted(Comparator.comparing(Field::getName)).toArray(Field[]::new);}
    private static String bits(double value){return String.format(java.util.Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    private static double number(String value){return Double.longBitsToDouble(Long.parseUnsignedLong(value,16));}
    private static void require(boolean condition,String message){if(!condition)throw new IllegalStateException(message);}
    private static Random copyRandom(Random random)throws Exception{
        ByteArrayOutputStream bytes=new ByteArrayOutputStream();try(ObjectOutputStream stream=new ObjectOutputStream(bytes)){stream.writeObject(random);}
        try(ObjectInputStream stream=new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray()))){return (Random)stream.readObject();}
    }
    private static GeneticCode explicit(){
        Gene gene=new Gene();gene.setLength(12);gene.setTheta(0.25);gene.setColor(Color.GREEN);gene.setBranch(-1);
        GeneticCode code=new GeneticCode(List.of(gene),1,0,0,0,1,1,1,1,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false);
        code._generation=41;code._cladeID="root";return code;
    }
    private static String historyGetter(String field){
        switch(field){case "populationList":return "getPopulationList";case "deathList":return "getDeathList";case "birthList":return "getBirthList";case "oxygenList":return "getOxygenList";case "carbonDioxideList":return "getCarbonDioxideList";case "methaneList":return "getMethaneList";default:throw new IllegalArgumentException("Unknown history field");}
    }
    private static String history(int count){StringBuilder value=new StringBuilder();for(int i=0;i<count;i++){if(i!=0)value.append(',');value.append(bits(i-50.5));}return value.toString();}
    private static String canonical()throws Exception{
        StringBuilder output=new StringBuilder();
        output.append(new Capture("defaults").finish());
        Capture population=new Capture("population-events-and-times");
        population.event("eventOrganismCreated");population.event("eventOrganismCreated");population.event("eventPopulationIncrease","32");
        population.time(32,1,500001,-1,true);population.event("eventPopulationIncrease","32");population.event("eventPopulationIncrease","33");
        population.event("eventPopulationDecrease","32");population.event("eventPopulationDecrease","0");
        population.time(1000,1,500001,-1,true);population.event("eventPopulationIncrease","33");population.event("eventPopulationDecrease","0");
        population.event("eventPopulationIncrease",Integer.toString(Integer.MAX_VALUE));population.event("eventPopulationDecrease",Integer.toString(Integer.MIN_VALUE));output.append(population.finish());
        Capture records=new Capture("record-ties-and-optional-donors");
        records.integer("actor:a:_nTotalChildren",5);records.integer("actor:a:_nTotalKills",6);records.integer("actor:a:_nTotalInfected",7);
        records.integer("actor:b:_nTotalChildren",5);records.integer("actor:b:_nTotalKills",6);records.integer("actor:b:_nTotalInfected",7);
        records.event("eventOrganismBorn","d","a");records.event("eventOrganismDie","c","a");records.event("eventOrganismInfects","b","a");
        records.time(12,2,499999,3,true);records.event("eventOrganismBorn","c","b");records.event("eventOrganismDie","a","b");records.event("eventOrganismInfects","c","b");
        records.integer("actor:b:_nTotalChildren",6);records.integer("actor:b:_nTotalKills",7);records.integer("actor:b:_nTotalInfected",8);
        records.event("eventOrganismBorn","a","b");records.event("eventOrganismDie","b","b");records.event("eventOrganismInfects","a","b");
        records.event("eventOrganismDie","d","~");records.event("eventOrganismInfects","d","~");output.append(records.finish());
        Capture alive=new Capture("alive-scans-reset-order-and-inactive");
        for(String actor:new String[]{"a","b"}){alive.integer("actor:"+actor+":_nTotalChildren",5);alive.integer("actor:"+actor+":_nTotalKills",6);alive.integer("actor:"+actor+":_nTotalInfected",7);}
        for(String field:new String[]{"_nTotalChildren","_nTotalKills","_nTotalInfected"}){alive.integer("actor:c:"+field,4);alive.integer("actor:d:"+field,100);}
        alive.event("findBestAliveBeings","a,b,c,d");alive.event("findBestAliveBeings","b,a,c,d");
        alive.integer("actor:b:_nTotalChildren",10);alive.integer("actor:b:_nTotalKills",20);alive.integer("actor:b:_nTotalInfected",30);alive.event("findBestAliveBeings","a,b,c,d");
        alive.flag("actor:b:alive",false);alive.event("findBestAliveBeings","a,b,c,d");alive.flag("actor:a:alive",false);alive.flag("actor:c:alive",false);alive.event("findBestAliveBeings","a,b,c,d");
        alive.event("findBestAliveBeings","~");alive.flag("actor:a:alive",true);for(String field:new String[]{"_nTotalChildren","_nTotalKills","_nTotalInfected"})alive.integer("actor:a:"+field,-1);
        alive.event("findBestAliveBeings","a");output.append(alive.finish());
        Capture shared=new Capture("shared-genomes-and-metadata-propagation");
        shared.integer("actor:a:_nTotalChildren",5);shared.integer("actor:a:_nTotalKills",6);shared.integer("actor:a:_nTotalInfected",7);
        shared.event("eventOrganismBorn","d","a");shared.event("eventOrganismDie","b","a");shared.event("eventOrganismInfects","c","a");shared.event("findBestAliveBeings","a,b,c");
        shared.set("genome:g0:_cladeID","String","source:mutated");shared.integer("genome:g0:_generation",Integer.MIN_VALUE);shared.set("genome:g0:gene0._length","double",bits(25.5));shared.snapshot("mutated-shared");
        shared.set("genome:g0:_cladeID","String","~");shared.snapshot("nullable-clade");output.append(shared.finish());
        Capture natural=new Capture("natural-extinction-states");natural.time(32,0,500000,0,true);
        natural.event("eventOrganismDie","d","~");natural.time(31,0,500000,0,true);
        natural.event("eventOrganismDie","d","~");natural.event("eventOrganismDie","d","~");natural.time(29,0,500000,0,true);
        natural.time(29,0,500000,0,true);natural.time(29,0,500000,0,true);output.append(natural.finish());
        for(int state=0;state<4;state++)for(int deaths:new int[]{3,4,6,7}){
            Capture matrix=new Capture("extinction-state-"+state+"-deaths-"+deaths);matrix.longValue("statistics:time",3);matrix.longValue("statistics:deathSum",8);
            matrix.integer("statistics:deathLastTime",deaths);matrix.integer("statistics:massExtintionState",state);matrix.integer("statistics:massExtintions",17);
            matrix.snapshot("controlled-input");matrix.time(12,0,500000,0,true);output.append(matrix.finish());
        }
        Capture rollover=new Capture("history-rollover-105");
        for(int i=0;i<105;i++){
            if(i%3==0){rollover.integer("actor:a:_nTotalChildren",i+1);rollover.call("eventOrganismBorn","c","a");}
            if(i%7==0){rollover.integer("actor:b:_nTotalKills",i+1);rollover.call("eventOrganismDie","d","b");}
            if(i%5==0){rollover.integer("actor:c:_nTotalInfected",i+1);rollover.call("eventOrganismInfects","b","c");}
            rollover.time(i-52,i*0.25,500000-i*3.5,-i*0.125,i==0||i==1||i==98||i==99||i==100||i==104);
        }
        output.append(rollover.finish());
        Capture oversized=new Capture("externally-seeded-history-over-100");
        for(String field:new String[]{"populationList","deathList","birthList","oxygenList","carbonDioxideList","methaneList"})oversized.set("statistics:"+field,"List",history(101));
        oversized.snapshot("controlled-history");oversized.time(7,-0d,Double.NaN,Double.POSITIVE_INFINITY,true);output.append(oversized.finish());
        Capture exceptional=new Capture("gas-signed-zero-nan-infinity");exceptional.time(4,-0d,500000,-0d,true);
        exceptional.time(4,Double.longBitsToDouble(0x7ff8000000000042L),Double.POSITIVE_INFINITY,Double.NEGATIVE_INFINITY,true);
        exceptional.time(4,Double.POSITIVE_INFINITY,Double.NEGATIVE_INFINITY,Double.NaN,true);exceptional.time(4,Double.NEGATIVE_INFINITY,Double.NaN,Double.POSITIVE_INFINITY,true);output.append(exceptional.finish());
        Capture initialNan=new Capture("constructor-nan-and-signed-zero",false,0,Double.longBitsToDouble(0xfff8000000000042L),-0d,Double.POSITIVE_INFINITY);
        initialNan.time(0,1,0,Double.NEGATIVE_INFINITY,true);initialNan.time(0,Double.NEGATIVE_INFINITY,Double.POSITIVE_INFINITY,0,true);output.append(initialNan.finish());
        Capture wrapping=new Capture("integer-and-long-wrapping");
        wrapping.integer("statistics:createdOrganisms",Integer.MAX_VALUE);wrapping.event("eventOrganismCreated");
        wrapping.longValue("statistics:birthSum",Long.MAX_VALUE);wrapping.integer("statistics:birthLastTime",Integer.MAX_VALUE);wrapping.event("eventOrganismBorn","d","a");
        wrapping.longValue("statistics:deathSum",Long.MAX_VALUE);wrapping.integer("statistics:deathLastTime",Integer.MAX_VALUE);wrapping.event("eventOrganismDie","c","~");
        wrapping.longValue("statistics:infectionsSum",Long.MAX_VALUE);wrapping.event("eventOrganismInfects","b","~");
        wrapping.longValue("statistics:populationSum",Long.MAX_VALUE);wrapping.time(1,0,500000,0,true);output.append(wrapping.finish());
        Capture timeWrap=new Capture("time-and-extinction-counter-wrapping");
        timeWrap.longValue("statistics:time",Long.MAX_VALUE);timeWrap.integer("statistics:massExtintions",Integer.MAX_VALUE);timeWrap.integer("statistics:deathLastTime",1);
        timeWrap.time(Integer.MAX_VALUE,1,500001,1,true);output.append(timeWrap.finish());
        Capture counterLimits=new Capture("actor-counter-limits-and-negative-averages");
        for(String field:new String[]{"_nTotalChildren","_nTotalKills","_nTotalInfected"})counterLimits.integer("actor:a:"+field,Integer.MAX_VALUE);
        counterLimits.event("eventOrganismBorn","d","a");counterLimits.event("eventOrganismDie","c","a");counterLimits.event("eventOrganismInfects","b","a");
        for(String field:new String[]{"_nTotalChildren","_nTotalKills","_nTotalInfected"})counterLimits.integer("actor:b:"+field,Integer.MIN_VALUE);
        counterLimits.event("eventOrganismBorn","d","b");counterLimits.event("eventOrganismDie","c","b");counterLimits.event("eventOrganismInfects","a","b");
        counterLimits.longValue("statistics:time",1);counterLimits.longValue("statistics:populationSum",Long.MIN_VALUE);counterLimits.longValue("statistics:deathSum",-3);counterLimits.longValue("statistics:birthSum",-5);counterLimits.longValue("statistics:infectionsSum",-7);
        counterLimits.snapshot("controlled-negative-averages");output.append(counterLimits.finish());
        return output.toString();
    }
    private static String rawNull()throws Exception{
        Capture capture=new Capture("uninitialized-java-actor-genotypes",true,Utils.DEF_INITIAL_ORGANISMS,Utils.DEF_INITIAL_O2,Utils.DEF_INITIAL_CO2,Utils.DEF_INITIAL_CH4);
        capture.integer("actor:a:_nTotalChildren",5);capture.integer("actor:a:_nTotalKills",6);capture.integer("actor:a:_nTotalInfected",7);
        capture.event("eventOrganismBorn","c","a");capture.event("eventOrganismDie","b","a");capture.event("eventOrganismInfects","c","a");capture.event("findBestAliveBeings","a,b,c,d");
        capture.time(4,0,500000,0,true);return capture.finish();
    }
    public static void main(String[]args)throws Exception{
        if(args.length<1||args.length>2||(args.length==2&&!args[1].equals("raw-null-genome")))throw new IllegalArgumentException("Usage: biogenesis.StatisticsOracle NEW_OUTPUT_DIRECTORY [raw-null-genome]");
        Path directory=Path.of(args[0]).toAbsolutePath().normalize();Path parent=directory.getParent().toRealPath();
        for(String property:List.of("user.home","java.io.tmpdir","java.util.prefs.userRoot","java.util.prefs.systemRoot")){
            String value=System.getProperty(property);require(value!=null&&Path.of(value).toRealPath().startsWith(parent),"Explicit isolated profile required: "+property);
        }
        require(!Files.exists(directory),"Output directory must not exist");Files.createDirectory(directory);
        boolean raw=args.length==2;String text=raw?rawNull():canonical();Files.writeString(directory.resolve(raw?"statistics_null_genome.tsv":"statistics.tsv"),text,StandardCharsets.UTF_8);
        System.out.println("STATISTICS_ORACLE PASS cases="+cases+" snapshots="+snapshots+" events="+calls+(raw?" mode=raw-null-genome":""));
    }
}
