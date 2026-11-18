// SPDX-License-Identifier: GPL-2.0-or-later
package biogenesis;

import java.util.Random;

/** Direct headless construction observations; this does not invoke World.genesis or World.time */
public final class ConstructionOracle {
    public static void main(String[] arguments) {
        if (arguments.length != 0) throw new IllegalArgumentException("No arguments expected");
        for (double coordinate : new double[] {-2.5, -1.5, -0.5, 0.5, 1.5,
                Math.nextDown(0.5), Math.nextUp(0.5), Math.nextDown(-0.5), Math.nextUp(-0.5)}) {
            System.out.printf("Q\t%s\t%d%n", Long.toHexString(Double.doubleToRawLongBits(coordinate)), (int)Math.round(coordinate));
        }
        for (long seed : new long[] {0, 42}) {
            for (int bound : new int[] {1, 1024, 1073741825, Integer.MAX_VALUE}) {
                Random random = new Random(seed);
                System.out.printf("R\t%d\t%d", seed, bound);
                for (int index = 0; index < 20; index++) System.out.printf("\t%d", random.nextInt(bound));
                System.out.printf("\t%d%n", random.nextLong());
            }
        }
        for (long[] input : new long[][] {
                {0, 128, 96, 32}, {42, 128, 96, 32}, {-1, 32, 32, 300}, {2026, 7000, 7000, 512}, {0, 1, 1, 0}}) {
            Utils.random = new Random(input[0]);
            Utils.WORLD_WIDTH = (int)input[1];
            Utils.WORLD_HEIGHT = (int)input[2];
            World world = new World(null);
            world._O2 = Utils.INITIAL_O2;
            world._CO2 = Utils.INITIAL_CO2;
            world._CH4 = Utils.INITIAL_CH4;
            for (int attempt = 0; attempt < input[3]; attempt++) {
                Organism organism = new Organism(world);
                if (organism.randomCreate()) world._organisms.add(organism);
            }
            System.out.printf("W\t%d\t%d\t%d\t%d\t%d\t%s\t%s%n", input[0], input[1], input[2], input[3],
                    world._organisms.size(), Double.toString(world.getO2()), Double.toString(world.getCO2()));
            // Rotation occurs after every placement, preserving original unrotated placement rectangles
            for (Organism organism : world._organisms) {
                organism.calculateBounds(true);
                for (int index = 0; index < organism._segments; index++) {
                    System.out.printf("S\t%d\t%d\t%d\t%d\t%d\t%d%n", organism._ID,
                            organism.x1[index] + organism._centerX, organism.y1[index] + organism._centerY,
                            organism.x2[index] + organism._centerX, organism.y2[index] + organism._centerY,
                            organism._segColor[index].getRGB() & 0xffffff);
                }
            }
            System.out.println("E");
        }
    }
}
