// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.awt.Color;
import java.lang.reflect.Field;
import java.util.List;
import java.util.Locale;
import java.util.Random;

public final class ThresholdOracle {
    public static void main(String[] args) throws Exception {
        if (args.length != 0) throw new IllegalArgumentException("No arguments expected");
        for (int value : new int[] {0, 9, 10, -19, 167772150, 167772160, 167772170,
                167772180, -167772170, Integer.MAX_VALUE, Integer.MIN_VALUE}) {
            System.out.println("R\t" + value + "\t" + Math.round(value / 10));
        }
        Utils.WORLD_WIDTH = 2000000000;
        Utils.WORLD_HEIGHT = 2000000000;
        Utils.INITIAL_ORGANISMS = 0;
        for (double length : new double[] {12, 16777215, 16777216, 16777217,
                16777218, 16777219, 16777220, 214748370, 500000000}) {
            Utils.random = new Random(827364);
            Gene gene = new Gene();
            gene.setLength(length);
            gene.setTheta(0);
            gene.setColor(Utils.ColorFLOWER);
            gene.setBranch(-1);
            GeneticCode code = new GeneticCode(List.of(gene),1,0,0,0,1,1,1,1,
                    false,false,false,false,false,false,false,false,false,false,false,false,false,false,false);
            World world = new World(new VisibleWorld(null));
            Field carbon = World.class.getDeclaredField("_CO2");
            carbon.setAccessible(true);
            carbon.setDouble(world, Utils.INITIAL_CO2);
            Organism organism = new Organism(world);
            organism._geneticCode = code;
            if (!organism.pasteOrganism(1000000000, 1000000000)) {
                throw new IllegalStateException("Actual flower placement failed");
            }
            System.out.println("F\t" + String.format(Locale.ROOT, "%016x", Double.doubleToRawLongBits(length))
                    + "\t" + organism._reproducelate + "\t" + organism._earlyReproduceEnergy
                    + "\t" + organism.getReproduceEnergy());
        }
    }
}
