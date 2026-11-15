import net.minecraft.Bootstrap;
import net.minecraft.block.Block;
import net.minecraft.block.Material;

/**
 * Dump 1.8.9's per-block movement material.
 *
 * <p>{@code World.getTopPosition} scans down while the block below does not
 * block movement, and skips {@code Material.FOILAGE}: that is not the modern
 * opacity rule. Each line is:
 *
 * <pre>
 * blockId &lt;tab&gt; class &lt;tab&gt; blocksMovement &lt;tab&gt; foliage
 * </pre>
 *
 * <p>Compile and run against the 1.8.9 named jar and its libraries (see
 * {@code /tmp/opencode/cp18.py} for the classpath):
 *
 * <pre>
 * javac -cp "$JAR:$LIBS" tools/BlockMaterialProbe18.java
 * java -cp "$JAR:$LIBS:." BlockMaterialProbe18 &gt; block-materials-1.8.9.txt
 * </pre>
 */
public class BlockMaterialProbe18 {
    public static void main(String[] args) {
        Bootstrap.initialize();
        // 1.8 registers blocks 0..=197; higher ids resolve to air.
        for (int id = 0; id < 256; id++) {
            Block block;
            try {
                block = Block.getById(id);
            } catch (RuntimeException e) {
                continue;
            }
            if (block == null) {
                continue;
            }
            Material material = block.getMaterial();
            System.out.println(id + "\t" + block.getClass().getSimpleName() + "\t"
                    + material.blocksMovement() + "\t" + (material == Material.FOILAGE));
        }
    }
}
