import java.lang.reflect.Field;
import java.util.List;
import net.minecraft.Bootstrap;
import net.minecraft.entity.EntityCategory;
import net.minecraft.structure.OceanMonumentStructure;
import net.minecraft.structure.TempleStructure;
import net.minecraft.util.collection.Weighting;
import net.minecraft.world.biome.Biome;

/**
 * Dump 1.8.9's per-biome natural spawn lists.
 *
 * <p>Each line is:
 *
 * <pre>
 * biome &lt;tab&gt; biomeId &lt;tab&gt; category &lt;tab&gt; entity class &lt;tab&gt; weight &lt;tab&gt; min &lt;tab&gt; max
 * </pre>
 *
 * <p>with two extra pseudo-biome rows, {@code #swamp_hut} and
 * {@code #ocean_monument}, for the structure-specific monster lists that
 * {@code SurfaceChunkGenerator.getSpawnEntries} substitutes.
 *
 * <p>Compile and run against the 1.8.9 named jar and its libraries (see
 * {@code /tmp/opencode/cp18.py} for the classpath):
 *
 * <pre>
 * javac -cp "$JAR:$LIBS" tools/BiomeSpawnProbe18.java
 * java -cp "$JAR:$LIBS:." BiomeSpawnProbe18 &gt; biome-spawns-1.8.9.txt
 * </pre>
 */
public class BiomeSpawnProbe18 {
    public static void main(String[] args) throws Exception {
        Bootstrap.initialize();
        Biome[] biomes = Biome.getBiomes();
        for (int id = 0; id < biomes.length; id++) {
            Biome biome = biomes[id];
            if (biome == null || biome.id != id) {
                continue;
            }
            for (EntityCategory category : EntityCategory.values()) {
                dump(id, category.name(), biome.getSpawnEntries(category));
            }
        }
        dump(-1, "swamp_hut", new TempleStructure().getMonsterSpawns());
        dump(-2, "ocean_monument", new OceanMonumentStructure().getSpawnableMobs());
    }

    private static final Field WEIGHT;

    static {
        try {
            WEIGHT = Weighting.Weight.class.getDeclaredField("weight");
            WEIGHT.setAccessible(true);
        } catch (ReflectiveOperationException e) {
            throw new ExceptionInInitializerError(e);
        }
    }

    private static void dump(int id, String category, List<Biome.SpawnEntry> list) {
        for (Biome.SpawnEntry entry : list) {
            int weight;
            try {
                weight = WEIGHT.getInt(entry);
            } catch (IllegalAccessException e) {
                throw new IllegalStateException(e);
            }
            System.out.println(id + "\t" + category + "\t" + entry.entity.getSimpleName()
                    + "\t" + weight + "\t" + entry.minGroupSize + "\t" + entry.maxGroupSize);
        }
    }
}
